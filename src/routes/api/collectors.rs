use axum::Json;
use axum::extract::{Path, State};
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::collector::{Collector, UpdateCollector};
use crate::models::collector_run::CollectorRun;

pub async fn list(State(pool): State<PgPool>) -> Result<Json<Vec<Collector>>, AppError> {
    let collectors = Collector::list(&pool).await?;
    Ok(Json(collectors))
}

pub async fn update(
    State(pool): State<PgPool>,
    Path(name): Path<String>,
    Json(input): Json<UpdateCollector>,
) -> Result<Json<Collector>, AppError> {
    let collector = Collector::update(&pool, &name, input).await?;
    Ok(Json(collector))
}

pub async fn trigger_run(
    State(pool): State<PgPool>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let collector = Collector::get_by_name(&pool, &name).await?;
    if !collector.enabled {
        return Err(AppError::BadRequest(format!(
            "Collector '{}' is disabled",
            collector.name
        )));
    }

    let run = CollectorRun::enqueue(&pool, &name, "manual").await?;

    Ok(Json(serde_json::json!({
        "status": "queued",
        "run_id": run.id,
        "collector": name,
    })))
}
