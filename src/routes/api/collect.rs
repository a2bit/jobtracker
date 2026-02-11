use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::collector_run::CollectorRun;
use crate::models::company::Company;
use crate::models::job::{CreateJob, Job};

#[derive(Debug, Deserialize)]
pub struct IngestRequest {
    pub collector_name: String,
    pub jobs: Vec<IngestJob>,
}

#[derive(Debug, Deserialize)]
pub struct IngestJob {
    pub company_name: String,
    pub title: String,
    pub url: Option<String>,
    pub location: Option<String>,
    pub remote_type: Option<String>,
    pub salary_min: Option<i32>,
    pub salary_max: Option<i32>,
    pub salary_currency: Option<String>,
    pub description: Option<String>,
    pub source: String,
    pub source_id: String,
    pub raw_data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub run_id: i32,
    pub found: i32,
    pub new: i32,
    pub updated: i32,
}

/// POST /api/v1/collect/ingest
///
/// Batch ingest jobs from an external collector. Resolves company names
/// to IDs internally and upserts each job. Creates a collector_run record
/// for audit trail.
pub async fn ingest(
    State(pool): State<PgPool>,
    Json(input): Json<IngestRequest>,
) -> Result<Json<IngestResponse>, AppError> {
    if input.jobs.is_empty() {
        return Err(AppError::BadRequest("No jobs provided".to_string()));
    }

    // Create a collector run record for this ingest
    let run = CollectorRun::enqueue(&pool, &input.collector_name, "api").await?;

    // Claim it immediately (transition pending -> running)
    sqlx::query("UPDATE collector_runs SET status = 'running', started_at = NOW() WHERE id = $1")
        .bind(run.id)
        .execute(&pool)
        .await?;

    let mut found = 0i32;
    let mut new = 0i32;
    let mut updated = 0i32;

    for ingest_job in &input.jobs {
        found += 1;

        // Resolve company name to ID (creates if needed)
        let company = Company::find_or_create(&pool, &ingest_job.company_name).await?;

        let create_job = CreateJob {
            company_id: company.id,
            title: ingest_job.title.clone(),
            url: ingest_job.url.clone(),
            location: ingest_job.location.clone(),
            remote_type: ingest_job.remote_type.clone(),
            salary_min: ingest_job.salary_min,
            salary_max: ingest_job.salary_max,
            salary_currency: ingest_job.salary_currency.clone(),
            description: ingest_job.description.clone(),
            requirements: None,
            source: ingest_job.source.clone(),
            source_id: Some(ingest_job.source_id.clone()),
            expires_at: None,
            raw_data: ingest_job.raw_data.clone(),
        };

        let (_job, was_inserted) = Job::upsert(&pool, create_job).await?;
        if was_inserted {
            new += 1;
        } else {
            updated += 1;
        }
    }

    // Mark run as succeeded with counts
    CollectorRun::mark_succeeded(&pool, run.id, found, new, updated).await?;

    Ok(Json(IngestResponse {
        run_id: run.id,
        found,
        new,
        updated,
    }))
}
