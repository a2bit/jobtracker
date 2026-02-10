use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::AppError;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Event {
    pub id: i32,
    pub application_id: Option<i32>,
    pub job_id: Option<i32>,
    pub event_type: String,
    pub description: Option<String>,
    pub occurred_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateEvent {
    pub application_id: Option<i32>,
    pub job_id: Option<i32>,
    pub event_type: String,
    pub description: Option<String>,
    pub occurred_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct EventFilters {
    pub application_id: Option<i32>,
    pub job_id: Option<i32>,
}

impl Event {
    pub async fn list(pool: &PgPool, filters: &EventFilters) -> Result<Vec<Event>, AppError> {
        let events = sqlx::query_as::<_, Event>(
            "SELECT * FROM events WHERE ($1::int4 IS NULL OR application_id = $1) AND ($2::int4 IS NULL OR job_id = $2) ORDER BY occurred_at DESC LIMIT 100",
        )
        .bind(filters.application_id)
        .bind(filters.job_id)
        .fetch_all(pool)
        .await?;
        Ok(events)
    }

    pub async fn create(pool: &PgPool, input: CreateEvent) -> Result<Event, AppError> {
        let occurred_at = input.occurred_at.unwrap_or_else(Utc::now);
        let event = sqlx::query_as::<_, Event>(
            "INSERT INTO events (application_id, job_id, event_type, description, occurred_at) VALUES ($1, $2, $3, $4, $5) RETURNING *",
        )
        .bind(input.application_id)
        .bind(input.job_id)
        .bind(&input.event_type)
        .bind(&input.description)
        .bind(occurred_at)
        .fetch_one(pool)
        .await?;
        Ok(event)
    }

    #[allow(dead_code)]
    pub async fn recent(pool: &PgPool, limit: i64) -> Result<Vec<Event>, AppError> {
        let events =
            sqlx::query_as::<_, Event>("SELECT * FROM events ORDER BY occurred_at DESC LIMIT $1")
                .bind(limit)
                .fetch_all(pool)
                .await?;
        Ok(events)
    }
}
