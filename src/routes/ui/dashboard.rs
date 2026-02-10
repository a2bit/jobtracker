use askama::Template;
use axum::extract::State;
use axum::response::Html;
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::event::Event;
use crate::models::job::Job;

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    job_count: i64,
    app_count: i64,
    interviewing_count: i64,
    offer_count: i64,
    status_counts: Vec<(String, i64)>,
    recent_jobs: Vec<Job>,
    recent_events: Vec<Event>,
}

pub async fn index(State(pool): State<PgPool>) -> Result<Html<String>, AppError> {
    let job_count = Job::count(&pool).await.unwrap_or(0);

    let status_counts = crate::models::application::Application::count_by_status(&pool)
        .await
        .unwrap_or_default();

    let app_count: i64 = status_counts.iter().map(|(_, c)| c).sum();
    let interviewing_count = status_counts
        .iter()
        .find(|(s, _)| s == "interviewing")
        .map(|(_, c)| *c)
        .unwrap_or(0);
    let offer_count = status_counts
        .iter()
        .filter(|(s, _)| s == "offer" || s == "accepted")
        .map(|(_, c)| c)
        .sum();

    let recent_jobs = Job::recent(&pool, 5).await.unwrap_or_default();
    let recent_events = Event::recent(&pool, 10).await.unwrap_or_default();

    let tmpl = DashboardTemplate {
        job_count,
        app_count,
        interviewing_count,
        offer_count,
        status_counts,
        recent_jobs,
        recent_events,
    };
    Ok(Html(
        tmpl.render()
            .map_err(|e| AppError::Internal(e.to_string()))?,
    ))
}
