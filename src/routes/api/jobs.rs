use axum::Json;
use axum::extract::{Path, Query, State};
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::job::{CreateJob, Job, JobFilters, UpdateJob};

pub async fn list(
    State(pool): State<PgPool>,
    Query(filters): Query<JobFilters>,
) -> Result<Json<Vec<Job>>, AppError> {
    let jobs = Job::list(&pool, &filters).await?;
    Ok(Json(jobs))
}

pub async fn get(State(pool): State<PgPool>, Path(id): Path<i32>) -> Result<Json<Job>, AppError> {
    let job = Job::get(&pool, id).await?;
    Ok(Json(job))
}

pub async fn create(
    State(pool): State<PgPool>,
    Json(input): Json<CreateJob>,
) -> Result<Json<Job>, AppError> {
    let job = Job::create(&pool, input).await?;
    Ok(Json(job))
}

pub async fn update(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    Json(input): Json<UpdateJob>,
) -> Result<Json<Job>, AppError> {
    let job = Job::update(&pool, id, input).await?;
    Ok(Json(job))
}

pub async fn upsert(
    State(pool): State<PgPool>,
    Json(input): Json<CreateJob>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (job, was_inserted) = Job::upsert(&pool, input).await?;
    Ok(Json(serde_json::json!({
        "job": job,
        "was_inserted": was_inserted,
    })))
}

pub async fn delete(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    Job::delete(&pool, id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}
