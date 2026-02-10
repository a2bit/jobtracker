use axum::Json;
use axum::extract::{Path, Query, State};
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::application::{
    Application, ApplicationFilters, CreateApplication, UpdateApplication,
};

pub async fn list(
    State(pool): State<PgPool>,
    Query(filters): Query<ApplicationFilters>,
) -> Result<Json<Vec<Application>>, AppError> {
    let apps = Application::list(&pool, &filters).await?;
    Ok(Json(apps))
}

pub async fn get(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> Result<Json<Application>, AppError> {
    let app = Application::get(&pool, id).await?;
    Ok(Json(app))
}

pub async fn create(
    State(pool): State<PgPool>,
    Json(input): Json<CreateApplication>,
) -> Result<Json<Application>, AppError> {
    let app = Application::create(&pool, input).await?;
    Ok(Json(app))
}

pub async fn update(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    Json(input): Json<UpdateApplication>,
) -> Result<Json<Application>, AppError> {
    let app = Application::update(&pool, id, input).await?;
    Ok(Json(app))
}

pub async fn delete(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    Application::delete(&pool, id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}
