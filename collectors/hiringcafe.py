#!/usr/bin/env python3
# /// script
# requires-python = ">=3.12"
# dependencies = ["curl_cffi", "click"]
# ///
"""
HiringCafe job collector for JobTracker.

Fetches jobs from HiringCafe's search API using curl_cffi to impersonate
a Chrome browser (bypasses TLS fingerprint blocking that rejects reqwest).
Posts results to JobTracker's batch ingest API.

Usage:
    uv run collectors/hiringcafe.py
    uv run collectors/hiringcafe.py --query "platform engineer" --pages 3
    uv run collectors/hiringcafe.py --dry-run
"""

from __future__ import annotations

import base64
import json
import os
import sys
import time
from urllib.parse import quote

import click
from curl_cffi import requests

PAGE_SIZE = 40
MAX_RETRIES = 3
RETRY_BASE_DELAY = 5
PAGE_DELAY = 3

# Characters that encodeURIComponent does NOT encode
# (matching the Rust ENCODE_URI_COMPONENT_SET)
SAFE_CHARS = "-_.!~*'()"


def default_state() -> dict:
    """Build the full state object required by the HiringCafe API.

    Matches the JS source (module 29652) defaults exactly.
    """
    any_obj = lambda label: {"label": label, "value": None}

    state = {
        "locations": [],
        "workplaceTypes": ["Remote", "Hybrid", "Onsite"],
        "defaultToUserLocation": True,
        "commitmentTypes": [
            "Full-time",
            "Part-time",
            "Contract",
            "Internship",
            "Temporary",
            "Volunteer",
        ],
        "jobTitleQuery": "",
        "jobDescriptionQuery": "",
        "dateFetchedPastNDays": 121,
        # Compensation
        "currency": any_obj("Any"),
        "frequency": any_obj("Any"),
        "minCompensationLowEnd": None,
        "minCompensationHighEnd": None,
        "maxCompensationLowEnd": None,
        "maxCompensationHighEnd": None,
        "restrictJobsToTransparentSalaries": False,
        "calcFrequency": "Yearly",
        # Experience
        "roleYoeRange": [0, 20],
        "excludeIfRoleYoeIsNotSpecified": False,
        "managementYoeRange": [0, 20],
        "excludeIfManagementYoeIsNotSpecified": False,
        # Degree fields
        "associatesDegreeFieldsOfStudy": [],
        "excludedAssociatesDegreeFieldsOfStudy": [],
        "bachelorsDegreeFieldsOfStudy": [],
        "excludedBachelorsDegreeFieldsOfStudy": [],
        "mastersDegreeFieldsOfStudy": [],
        "excludedMastersDegreeFieldsOfStudy": [],
        "doctorateDegreeFieldsOfStudy": [],
        "excludedDoctorateDegreeFieldsOfStudy": [],
        # Degree requirements
        "associatesDegreeRequirements": [],
        "bachelorsDegreeRequirements": [],
        "mastersDegreeRequirements": [],
        "doctorateDegreeRequirements": [],
        # Licenses
        "licensesAndCertifications": [],
        "excludedLicensesAndCertifications": [],
        "excludeAllLicensesAndCertifications": False,
        # Categories
        "departments": [],
        "excludedDepartments": [],
        "industries": [],
        "excludedIndustries": [],
        "companyKeywords": [],
        "excludedCompanyKeywords": [],
        "hideJobTypes": [],
        "applicationFormEase": [],
        # Language
        "languageRequirements": [],
        "excludedLanguageRequirements": [],
        "languageRequirementsOperator": "OR",
        "excludeJobsWithAdditionalLanguageRequirements": False,
        # Benefits
        "benefitsAndPerks": [],
    }
    return state


def encode_state(state: dict) -> str:
    """Encode state as HiringCafe expects: JSON -> encodeURIComponent -> base64."""
    json_str = json.dumps(state, separators=(",", ":"))
    uri_encoded = quote(json_str, safe=SAFE_CHARS)
    return base64.b64encode(uri_encoded.encode()).decode()


def parse_job(raw: dict) -> dict | None:
    """Parse a single job from the HiringCafe API response."""
    vpd = raw.get("v5_processed_job_data")
    if not vpd:
        return None

    vcd = raw.get("v5_processed_company_data", {})
    ji = raw.get("job_information", {})

    company_name = vpd.get("company_name") or vcd.get("name") or "Unknown"
    title = vpd.get("core_job_title") or ji.get("title") or "Untitled"

    source_id = raw.get("objectID") or raw.get("requisition_id")
    if not source_id:
        return None

    salary_min = vpd.get("yearly_min_compensation")
    salary_max = vpd.get("yearly_max_compensation")
    if salary_min is not None:
        salary_min = int(salary_min)
    if salary_max is not None:
        salary_max = int(salary_max)

    return {
        "company_name": company_name,
        "title": title,
        "url": raw.get("apply_url"),
        "location": vpd.get("formatted_workplace_location"),
        "remote_type": vpd.get("workplace_type"),
        "salary_min": salary_min,
        "salary_max": salary_max,
        "salary_currency": vpd.get("listed_compensation_currency"),
        "description": ji.get("description"),
        "source": "hiringcafe",
        "source_id": str(source_id),
        "raw_data": raw,
    }


def fetch_page(session: requests.Session, encoded_state: str, page: int) -> dict:
    """Fetch a single page from HiringCafe with retry on 429."""
    url = (
        f"https://hiring.cafe/api/search-jobs"
        f"?s={quote(encoded_state, safe='')}&size={PAGE_SIZE}&page={page}"
    )

    backoff = RETRY_BASE_DELAY
    for attempt in range(MAX_RETRIES + 1):
        resp = session.get(
            url,
            headers={
                "Accept": "application/json,text/html,*/*;q=0.8",
                "Accept-Language": "de-DE,de;q=0.9,en;q=0.8",
                "Sec-Fetch-Dest": "document",
                "Sec-Fetch-Mode": "navigate",
                "Sec-Fetch-Site": "none",
            },
            timeout=30,
        )

        if resp.status_code == 429:
            if attempt < MAX_RETRIES:
                print(
                    f"  429 rate limited, retrying in {backoff}s "
                    f"(attempt {attempt + 1}/{MAX_RETRIES})"
                )
                time.sleep(backoff)
                backoff *= 2
                continue
            raise RuntimeError("HiringCafe 429 after all retries exhausted")

        resp.raise_for_status()
        return resp.json()

    raise RuntimeError("unreachable")


def collect_jobs(query: str, max_pages: int) -> list[dict]:
    """Collect jobs from HiringCafe API with pagination."""
    state = default_state()
    if query:
        state["jobTitleQuery"] = query

    encoded = encode_state(state)

    session = requests.Session(impersonate="chrome")

    all_jobs = []
    for page in range(max_pages):
        if page > 0:
            time.sleep(PAGE_DELAY)

        print(f"Fetching page {page}...", end=" ", flush=True)
        data = fetch_page(session, encoded, page)

        results = data.get("results", [])
        page_jobs = []
        for raw in results:
            job = parse_job(raw)
            if job:
                page_jobs.append(job)

        print(f"{len(page_jobs)} jobs")
        all_jobs.extend(page_jobs)

        if len(results) < PAGE_SIZE:
            break

    return all_jobs


def ingest_jobs(jobs: list[dict], base_url: str, token: str) -> dict:
    """POST jobs to the JobTracker batch ingest API."""
    url = f"{base_url}/api/v1/collect/ingest"
    payload = {
        "collector_name": "hiringcafe",
        "jobs": jobs,
    }

    resp = requests.post(
        url,
        json=payload,
        headers={
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/json",
        },
        timeout=60,
    )
    resp.raise_for_status()
    return resp.json()


@click.command()
@click.option("--query", default="", help="Job title search query")
@click.option("--pages", default=5, help="Maximum pages to fetch (40 jobs/page)")
@click.option("--dry-run", is_flag=True, help="Collect but don't ingest")
def main(query: str, pages: int, dry_run: bool) -> None:
    """Collect jobs from HiringCafe and ingest into JobTracker."""
    base_url = os.environ.get("JOBTRACKER_URL", "http://localhost:8080")
    token = os.environ.get("JOBTRACKER_TOKEN", "")

    if not dry_run and not token:
        print("Error: JOBTRACKER_TOKEN is required (set env var)", file=sys.stderr)
        sys.exit(1)

    print(f"Collecting from HiringCafe (query={query!r}, max_pages={pages})")
    jobs = collect_jobs(query, pages)
    print(f"Collected {len(jobs)} jobs total")

    if not jobs:
        print("No jobs found, exiting")
        return

    if dry_run:
        print(f"Dry run: would ingest {len(jobs)} jobs")
        for job in jobs[:5]:
            print(f"  - {job['title']} at {job['company_name']}")
        if len(jobs) > 5:
            print(f"  ... and {len(jobs) - 5} more")
        return

    print(f"Ingesting {len(jobs)} jobs to {base_url}...")
    result = ingest_jobs(jobs, base_url, token)
    print(
        f"Ingested {result['found']} jobs "
        f"(new: {result['new']}, updated: {result['updated']}, "
        f"run_id: {result['run_id']})"
    )


if __name__ == "__main__":
    main()
