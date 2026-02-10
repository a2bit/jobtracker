use askama::Template;
use axum::Form;
use axum::extract::{Path, Query, State};
use axum::response::{Html, Redirect};
use serde::Deserialize;
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::application::Application;
use crate::models::company::Company;
use crate::models::event::Event;
use crate::models::job::{CreateJob, Job, JobFilters};

#[derive(Template)]
#[template(path = "jobs/list.html")]
struct JobListTemplate {
    jobs: Vec<Job>,
    search: String,
    source: String,
}

#[derive(Template)]
#[template(path = "jobs/detail.html")]
struct JobDetailTemplate {
    job: Job,
    company_name: String,
    application: Option<Application>,
    events: Vec<Event>,
}

#[derive(Debug, Deserialize)]
pub struct JobListQuery {
    pub search: Option<String>,
    pub source: Option<String>,
}

pub async fn list(
    State(pool): State<PgPool>,
    Query(query): Query<JobListQuery>,
) -> Result<Html<String>, AppError> {
    let filters = JobFilters {
        source: query.source.clone().filter(|s| !s.is_empty()),
        search: query.search.clone().filter(|s| !s.is_empty()),
        page: Some(1),
        per_page: Some(50),
    };
    let jobs = Job::list(&pool, &filters).await?;
    let tmpl = JobListTemplate {
        jobs,
        search: query.search.unwrap_or_default(),
        source: query.source.unwrap_or_default(),
    };
    Ok(Html(
        tmpl.render()
            .map_err(|e| AppError::Internal(e.to_string()))?,
    ))
}

pub async fn detail(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> Result<Html<String>, AppError> {
    let job = Job::get(&pool, id).await?;
    let company = Company::get(&pool, job.company_id).await?;

    // Find if there's an application for this job
    let application = sqlx::query_as::<_, Application>(
        "SELECT * FROM applications WHERE job_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await?;

    let events = sqlx::query_as::<_, Event>(
        "SELECT * FROM events WHERE job_id = $1 ORDER BY occurred_at DESC",
    )
    .bind(id)
    .fetch_all(&pool)
    .await?;

    let tmpl = JobDetailTemplate {
        job,
        company_name: company.name,
        application,
        events,
    };
    Ok(Html(
        tmpl.render()
            .map_err(|e| AppError::Internal(e.to_string()))?,
    ))
}

#[derive(Debug, Deserialize)]
pub struct CreateJobForm {
    pub company_name: String,
    pub title: String,
    pub url: Option<String>,
    pub location: Option<String>,
    pub remote_type: Option<String>,
    pub salary_min: Option<i32>,
    pub salary_max: Option<i32>,
    pub source: String,
}

pub async fn create(
    State(pool): State<PgPool>,
    Form(input): Form<CreateJobForm>,
) -> Result<Redirect, AppError> {
    // Find or create company
    let company = match sqlx::query_as::<_, Company>("SELECT * FROM companies WHERE name = $1")
        .bind(&input.company_name)
        .fetch_optional(&pool)
        .await?
    {
        Some(c) => c,
        None => {
            Company::create(
                &pool,
                crate::models::company::CreateCompany {
                    name: input.company_name,
                    website: None,
                    careers_url: None,
                    ats_platform: None,
                    notes: None,
                },
            )
            .await?
        }
    };

    let job = Job::create(
        &pool,
        CreateJob {
            company_id: company.id,
            title: input.title,
            url: input.url.filter(|s| !s.is_empty()),
            location: input.location.filter(|s| !s.is_empty()),
            remote_type: input.remote_type.filter(|s| !s.is_empty()),
            salary_min: input.salary_min,
            salary_max: input.salary_max,
            salary_currency: None,
            description: None,
            requirements: None,
            source: input.source,
            source_id: None,
            expires_at: None,
            raw_data: None,
        },
    )
    .await?;

    Ok(Redirect::to(&format!("/jobs/{}", job.id)))
}
