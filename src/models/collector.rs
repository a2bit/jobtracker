use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::AppError;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Collector {
    pub id: i32,
    pub name: String,
    pub enabled: bool,
    pub config: serde_json::Value,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCollector {
    pub enabled: Option<bool>,
    pub config: Option<serde_json::Value>,
}

impl Collector {
    pub async fn list(pool: &PgPool) -> Result<Vec<Collector>, AppError> {
        let collectors =
            sqlx::query_as::<_, Collector>("SELECT * FROM collectors ORDER BY name")
                .fetch_all(pool)
                .await?;
        Ok(collectors)
    }

    pub async fn get_by_name(pool: &PgPool, name: &str) -> Result<Collector, AppError> {
        sqlx::query_as::<_, Collector>("SELECT * FROM collectors WHERE name = $1")
            .bind(name)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Collector '{name}' not found")))
    }

    pub async fn update(
        pool: &PgPool,
        name: &str,
        input: UpdateCollector,
    ) -> Result<Collector, AppError> {
        let existing = Self::get_by_name(pool, name).await?;
        let collector = sqlx::query_as::<_, Collector>(
            "UPDATE collectors SET enabled = $2, config = $3, updated_at = NOW() WHERE name = $1 RETURNING *",
        )
        .bind(name)
        .bind(input.enabled.unwrap_or(existing.enabled))
        .bind(input.config.unwrap_or(existing.config))
        .fetch_one(pool)
        .await?;
        Ok(collector)
    }

    pub async fn record_run(
        pool: &PgPool,
        name: &str,
        error: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE collectors SET last_run_at = NOW(), last_error = $2, updated_at = NOW() WHERE name = $1",
        )
        .bind(name)
        .bind(error)
        .execute(pool)
        .await?;
        Ok(())
    }
}
