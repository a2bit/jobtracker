use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use serde_json::Value;

use crate::collectors::{CollectedJob, JobCollector};
use crate::error::AppError;

/// Characters that encodeURIComponent does NOT encode.
/// RFC 3986 unreserved: A-Z a-z 0-9 - _ . ! ~ * ' ( )
const ENCODE_URI_COMPONENT_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'!')
    .remove(b'~')
    .remove(b'*')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')');

const BASE_URL: &str = "https://hiring.cafe";
const PAGE_SIZE: u32 = 40;

pub struct HiringCafe;

#[async_trait]
impl JobCollector for HiringCafe {
    fn name(&self) -> &str {
        "hiringcafe"
    }

    async fn collect(&self, config: &Value) -> Result<Vec<CollectedJob>, AppError> {
        let query = config
            .get("jobTitleQuery")
            .and_then(|v| v.as_str())
            .or_else(|| {
                config
                    .get("search_terms")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("");

        // Try native reqwest first, fall back to Python CLI on 429
        match self.collect_native(config, query).await {
            Ok(jobs) => Ok(jobs),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("429") || msg.contains("Too Many Requests") {
                    tracing::warn!("HiringCafe returned 429, falling back to Python CLI");
                    self.collect_via_cli(query).await
                } else {
                    Err(e)
                }
            }
        }
    }
}

impl HiringCafe {
    /// Fetch jobs using reqwest with browser-like headers.
    async fn collect_native(
        &self,
        config: &Value,
        query: &str,
    ) -> Result<Vec<CollectedJob>, AppError> {
        let state = build_state(config, query);
        let encoded = encode_state(&state);

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .build()
            .map_err(|e| AppError::Internal(format!("Failed to build HTTP client: {e}")))?;

        let url = format!(
            "{BASE_URL}/api/search-jobs?s={}&size={PAGE_SIZE}&page=0",
            urlencoded(&encoded)
        );

        let resp = client
            .get(&url)
            .header("Accept", "application/json,text/html,*/*;q=0.8")
            .header("Accept-Language", "de-DE,de;q=0.9,en;q=0.8")
            .header("Sec-Fetch-Dest", "document")
            .header("Sec-Fetch-Mode", "navigate")
            .header("Sec-Fetch-Site", "none")
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("HiringCafe request failed: {e}")))?;

        if resp.status().as_u16() == 429 {
            return Err(AppError::Internal("429 Too Many Requests".to_string()));
        }

        if !resp.status().is_success() {
            return Err(AppError::Internal(format!(
                "HiringCafe returned {}",
                resp.status()
            )));
        }

        let data: Value = resp
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse response: {e}")))?;

        parse_results(&data)
    }

    /// Fallback: shell out to the Python CLI which uses curl_cffi for TLS fingerprinting.
    async fn collect_via_cli(&self, query: &str) -> Result<Vec<CollectedJob>, AppError> {
        let output = tokio::process::Command::new("hiringcafe-cli")
            .args(["search", query, "--llm", "--count", "40"])
            .output()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to run hiringcafe-cli: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Internal(format!(
                "hiringcafe-cli failed: {stderr}"
            )));
        }

        let data: Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| AppError::Internal(format!("Failed to parse CLI output: {e}")))?;

        let jobs = data
            .get("jobs")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut collected = Vec::new();
        for job in &jobs {
            if let Some(cj) = parse_cli_job(job) {
                collected.push(cj);
            }
        }
        Ok(collected)
    }
}

/// Build the full state object required by the HiringCafe API.
/// Starts from defaults and applies config overrides.
fn build_state(config: &Value, query: &str) -> Value {
    let mut state = default_state();

    if !query.is_empty()
        && let Some(obj) = state.as_object_mut()
    {
        obj.insert(
            "jobTitleQuery".to_string(),
            Value::String(query.to_string()),
        );
    }

    // Apply config overrides for fields the API supports
    let override_keys = [
        "locations",
        "workplaceTypes",
        "commitmentTypes",
        "dateFetchedPastNDays",
        "departments",
        "industries",
    ];

    if let (Some(state_obj), Some(config_obj)) = (state.as_object_mut(), config.as_object()) {
        for key in &override_keys {
            if let Some(val) = config_obj.get(*key) {
                state_obj.insert((*key).to_string(), val.clone());
            }
        }
    }

    state
}

/// The full default state matching the JS source (module 29652).
/// Built programmatically to avoid hitting the json! macro recursion limit.
fn default_state() -> Value {
    let null = Value::Null;
    let empty_arr = || Value::Array(vec![]);
    let s = |v: &str| Value::String(v.to_string());
    let b = Value::Bool;
    let n = |v: i64| Value::Number(serde_json::Number::from(v));

    let mut m = serde_json::Map::with_capacity(48);

    m.insert("locations".into(), empty_arr());
    m.insert(
        "workplaceTypes".into(),
        Value::Array(vec![s("Remote"), s("Hybrid"), s("Onsite")]),
    );
    m.insert("defaultToUserLocation".into(), b(true));
    m.insert(
        "commitmentTypes".into(),
        Value::Array(vec![
            s("Full-time"),
            s("Part-time"),
            s("Contract"),
            s("Internship"),
            s("Temporary"),
            s("Volunteer"),
        ]),
    );
    m.insert("jobTitleQuery".into(), s(""));
    m.insert("jobDescriptionQuery".into(), s(""));
    m.insert("dateFetchedPastNDays".into(), n(121));

    // Compensation filters
    let any_obj = |label: &str| {
        let mut obj = serde_json::Map::new();
        obj.insert("label".into(), s(label));
        obj.insert("value".into(), null.clone());
        Value::Object(obj)
    };
    m.insert("currency".into(), any_obj("Any"));
    m.insert("frequency".into(), any_obj("Any"));
    m.insert("minCompensationLowEnd".into(), null.clone());
    m.insert("minCompensationHighEnd".into(), null.clone());
    m.insert("maxCompensationLowEnd".into(), null.clone());
    m.insert("maxCompensationHighEnd".into(), null.clone());
    m.insert("restrictJobsToTransparentSalaries".into(), b(false));
    m.insert("calcFrequency".into(), s("Yearly"));

    // Experience ranges
    m.insert("roleYoeRange".into(), Value::Array(vec![n(0), n(20)]));
    m.insert("excludeIfRoleYoeIsNotSpecified".into(), b(false));
    m.insert("managementYoeRange".into(), Value::Array(vec![n(0), n(20)]));
    m.insert("excludeIfManagementYoeIsNotSpecified".into(), b(false));

    // Degree fields (e.g. associatesDegreeFieldsOfStudy, excludedAssociatesDegreeFieldsOfStudy)
    for prefix in ["associates", "bachelors", "masters", "doctorate"] {
        m.insert(format!("{prefix}DegreeFieldsOfStudy"), empty_arr());
        let cap = format!("{}{}", &prefix[..1].to_uppercase(), &prefix[1..]);
        m.insert(format!("excluded{cap}DegreeFieldsOfStudy"), empty_arr());
    }

    // Degree requirements
    for prefix in [
        "associatesDegreeRequirements",
        "bachelorsDegreeRequirements",
        "mastersDegreeRequirements",
        "doctorateDegreeRequirements",
    ] {
        m.insert(prefix.into(), empty_arr());
    }

    // Licenses
    m.insert("licensesAndCertifications".into(), empty_arr());
    m.insert("excludedLicensesAndCertifications".into(), empty_arr());
    m.insert("excludeAllLicensesAndCertifications".into(), b(false));

    // Categories
    m.insert("departments".into(), empty_arr());
    m.insert("excludedDepartments".into(), empty_arr());
    m.insert("industries".into(), empty_arr());
    m.insert("excludedIndustries".into(), empty_arr());
    m.insert("companyKeywords".into(), empty_arr());
    m.insert("excludedCompanyKeywords".into(), empty_arr());
    m.insert("hideJobTypes".into(), empty_arr());
    m.insert("applicationFormEase".into(), empty_arr());

    // Language
    m.insert("languageRequirements".into(), empty_arr());
    m.insert("excludedLanguageRequirements".into(), empty_arr());
    m.insert("languageRequirementsOperator".into(), s("OR"));
    m.insert(
        "excludeJobsWithAdditionalLanguageRequirements".into(),
        b(false),
    );

    m.insert("benefitsAndPerks".into(), empty_arr());

    Value::Object(m)
}

/// Encode state as the HiringCafe API expects:
/// JSON.stringify -> encodeURIComponent -> btoa
fn encode_state(state: &Value) -> String {
    let json_str = serde_json::to_string(state).unwrap_or_default();
    let uri_encoded: String = utf8_percent_encode(&json_str, ENCODE_URI_COMPONENT_SET).to_string();
    BASE64.encode(uri_encoded.as_bytes())
}

/// URL-encode a string for use in query parameters.
fn urlencoded(s: &str) -> String {
    utf8_percent_encode(s, ENCODE_URI_COMPONENT_SET).to_string()
}

/// Parse the native API search response into CollectedJob structs.
fn parse_results(data: &Value) -> Result<Vec<CollectedJob>, AppError> {
    let results = data
        .get("results")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::Internal("Missing 'results' in response".to_string()))?;

    let mut jobs = Vec::new();
    for raw in results {
        if let Some(job) = parse_api_job(raw) {
            jobs.push(job);
        }
    }
    Ok(jobs)
}

/// Parse a single job from the native API response format.
fn parse_api_job(raw: &Value) -> Option<CollectedJob> {
    let vpd = raw.get("v5_processed_job_data")?;
    let vcd = raw.get("v5_processed_company_data").unwrap_or(raw);
    let ji = raw.get("job_information").unwrap_or(raw);

    let company_name = vpd
        .get("company_name")
        .or_else(|| vcd.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let title = vpd
        .get("core_job_title")
        .or_else(|| ji.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("Untitled")
        .to_string();

    let source_id = raw
        .get("objectID")
        .or_else(|| raw.get("requisition_id"))
        .and_then(|v| v.as_str())?
        .to_string();

    let location = vpd
        .get("formatted_workplace_location")
        .and_then(|v| v.as_str())
        .map(String::from);

    let remote_type = vpd
        .get("workplace_type")
        .and_then(|v| v.as_str())
        .map(String::from);

    let (salary_min, salary_max) = extract_salary(vpd);
    let salary_currency = vpd
        .get("listed_compensation_currency")
        .and_then(|v| v.as_str())
        .map(String::from);

    let url = raw
        .get("apply_url")
        .and_then(|v| v.as_str())
        .map(String::from);

    let description = ji
        .get("description")
        .and_then(|v| v.as_str())
        .map(String::from);

    Some(CollectedJob {
        company_name,
        title,
        url,
        location,
        remote_type,
        salary_min,
        salary_max,
        salary_currency,
        description,
        source: "hiringcafe".to_string(),
        source_id,
        raw_data: Some(raw.clone()),
    })
}

/// Parse a job from the Python CLI's --llm JSON output format.
fn parse_cli_job(job: &Value) -> Option<CollectedJob> {
    let title = job.get("title").and_then(|v| v.as_str())?.to_string();
    let company_name = job
        .get("company")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let source_id = job
        .get("id")
        .or_else(|| job.get("requisition_id"))
        .and_then(|v| v.as_str())?
        .to_string();

    Some(CollectedJob {
        company_name,
        title,
        url: job
            .get("apply_url")
            .and_then(|v| v.as_str())
            .map(String::from),
        location: job
            .get("location")
            .and_then(|v| v.as_str())
            .map(String::from),
        remote_type: job
            .get("workplace_type")
            .and_then(|v| v.as_str())
            .map(String::from),
        salary_min: None,
        salary_max: None,
        salary_currency: None,
        description: job
            .get("description_html")
            .and_then(|v| v.as_str())
            .map(String::from),
        source: "hiringcafe".to_string(),
        source_id,
        raw_data: Some(job.clone()),
    })
}

/// Extract yearly salary min/max from v5_processed_job_data.
fn extract_salary(vpd: &Value) -> (Option<i32>, Option<i32>) {
    let min = vpd
        .get("yearly_min_compensation")
        .and_then(|v| v.as_f64())
        .map(|v| v as i32);
    let max = vpd
        .get("yearly_max_compensation")
        .and_then(|v| v.as_f64())
        .map(|v| v as i32);
    (min, max)
}
