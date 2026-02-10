use askama::Template;
use axum::Form;
use axum::extract::{Path, Query, State};
use axum::response::{Html, Redirect};
use serde::Deserialize;
use sqlx::PgPool;

use crate::error::{AppError, HtmlError};
use crate::models::application::{Application, ApplicationFilters, UpdateApplication};
use crate::models::event::{CreateEvent, Event};
use crate::models::job::Job;

/// Enriched application with job/company context for list display.
pub struct ApplicationRow {
    pub app: Application,
    pub job_title: String,
    pub company_name: String,
}

#[derive(Template)]
#[template(path = "applications/list.html")]
struct AppListTemplate {
    applications: Vec<ApplicationRow>,
    status_filter: String,
    statuses: Vec<String>,
}

#[derive(Template)]
#[template(path = "applications/detail.html")]
struct AppDetailTemplate {
    application: Application,
    job: Job,
    company_name: String,
    events: Vec<Event>,
    statuses: Vec<String>,
}

#[derive(Template)]
#[template(path = "partials/timeline.html")]
struct TimelinePartial {
    events: Vec<Event>,
}

fn all_statuses() -> Vec<String> {
    vec![
        "draft".into(),
        "applied".into(),
        "interviewing".into(),
        "rejected".into(),
        "offer".into(),
        "accepted".into(),
        "withdrawn".into(),
    ]
}

#[derive(Debug, Deserialize)]
pub struct AppListQuery {
    pub status: Option<String>,
}

pub async fn list(
    State(pool): State<PgPool>,
    Query(query): Query<AppListQuery>,
) -> Result<Html<String>, HtmlError> {
    let filters = ApplicationFilters {
        status: query.status.clone().filter(|s| !s.is_empty()),
    };
    let apps = Application::list(&pool, &filters).await?;

    // Enrich with job/company info
    let mut rows = Vec::with_capacity(apps.len());
    for app in apps {
        let job_info: Option<(String, String)> = sqlx::query_as(
            "SELECT j.title, c.name FROM jobs j JOIN companies c ON j.company_id = c.id WHERE j.id = $1",
        )
        .bind(app.job_id)
        .fetch_optional(&pool)
        .await?;

        let (job_title, company_name) = job_info.unwrap_or(("Unknown".into(), "Unknown".into()));
        rows.push(ApplicationRow {
            app,
            job_title,
            company_name,
        });
    }

    let tmpl = AppListTemplate {
        applications: rows,
        status_filter: query.status.unwrap_or_default(),
        statuses: all_statuses(),
    };
    Ok(Html(
        tmpl.render()
            .map_err(|e| AppError::Internal(e.to_string()))?,
    ))
}

pub async fn detail(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> Result<Html<String>, HtmlError> {
    let application = Application::get(&pool, id).await?;
    let job = Job::get(&pool, application.job_id).await?;
    let company_name: (String,) = sqlx::query_as("SELECT name FROM companies WHERE id = $1")
        .bind(job.company_id)
        .fetch_one(&pool)
        .await?;

    let events = sqlx::query_as::<_, Event>(
        "SELECT * FROM events WHERE application_id = $1 OR job_id = $2 ORDER BY occurred_at DESC",
    )
    .bind(id)
    .bind(job.id)
    .fetch_all(&pool)
    .await?;

    let tmpl = AppDetailTemplate {
        application,
        job,
        company_name: company_name.0,
        events,
        statuses: all_statuses(),
    };
    Ok(Html(
        tmpl.render()
            .map_err(|e| AppError::Internal(e.to_string()))?,
    ))
}

#[derive(Debug, Deserialize)]
pub struct CreateAppForm {
    pub job_id: i32,
}

pub async fn create(
    State(pool): State<PgPool>,
    Form(input): Form<CreateAppForm>,
) -> Result<Redirect, HtmlError> {
    let app = Application::create(
        &pool,
        crate::models::application::CreateApplication {
            job_id: input.job_id,
            status: None,
            cv_variant: None,
            notes: None,
        },
    )
    .await?;
    Ok(Redirect::to(&format!("/applications/{}", app.id)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateAppForm {
    pub status: Option<String>,
    pub cv_variant: Option<String>,
    pub notes: Option<String>,
}

pub async fn update(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    Form(input): Form<UpdateAppForm>,
) -> Result<Redirect, HtmlError> {
    Application::update(
        &pool,
        id,
        UpdateApplication {
            status: input.status.filter(|s| !s.is_empty()),
            cv_variant: input.cv_variant.filter(|s| !s.is_empty()),
            applied_at: None,
            response_at: None,
            notes: input.notes.filter(|s| !s.is_empty()),
        },
    )
    .await?;
    Ok(Redirect::to(&format!("/applications/{id}")))
}

#[derive(Debug, Deserialize)]
pub struct CreateEventForm {
    pub application_id: Option<i32>,
    pub job_id: Option<i32>,
    pub event_type: String,
    pub description: Option<String>,
}

pub async fn create_event(
    State(pool): State<PgPool>,
    Form(input): Form<CreateEventForm>,
) -> Result<Html<String>, HtmlError> {
    let event = Event::create(
        &pool,
        CreateEvent {
            application_id: input.application_id,
            job_id: input.job_id,
            event_type: input.event_type,
            description: input.description.filter(|s| !s.is_empty()),
            occurred_at: None,
        },
    )
    .await?;

    // Return the updated timeline partial
    let app_id = event.application_id;
    let job_id = event.job_id;

    let events = sqlx::query_as::<_, Event>(
        "SELECT * FROM events WHERE ($1::int4 IS NULL OR application_id = $1) AND ($2::int4 IS NULL OR job_id = $2) ORDER BY occurred_at DESC",
    )
    .bind(app_id)
    .bind(job_id)
    .fetch_all(&pool)
    .await?;

    let tmpl = TimelinePartial { events };
    Ok(Html(
        tmpl.render()
            .map_err(|e| AppError::Internal(e.to_string()))?,
    ))
}
