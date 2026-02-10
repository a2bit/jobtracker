use axum::extract::{Query, State};
use axum::Json;
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::event::{CreateEvent, Event, EventFilters};

pub async fn list(
    State(pool): State<PgPool>,
    Query(filters): Query<EventFilters>,
) -> Result<Json<Vec<Event>>, AppError> {
    let events = Event::list(&pool, &filters).await?;
    Ok(Json(events))
}

pub async fn create(
    State(pool): State<PgPool>,
    Json(input): Json<CreateEvent>,
) -> Result<Json<Event>, AppError> {
    let event = Event::create(&pool, input).await?;
    Ok(Json(event))
}
