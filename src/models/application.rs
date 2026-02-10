use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::AppError;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Application {
    pub id: i32,
    pub job_id: i32,
    pub status: String,
    pub cv_variant: Option<String>,
    pub applied_at: Option<DateTime<Utc>>,
    pub response_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateApplication {
    pub job_id: i32,
    pub status: Option<String>,
    pub cv_variant: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateApplication {
    pub status: Option<String>,
    pub cv_variant: Option<String>,
    pub applied_at: Option<DateTime<Utc>>,
    pub response_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApplicationFilters {
    pub status: Option<String>,
}

impl Application {
    pub async fn list(
        pool: &PgPool,
        filters: &ApplicationFilters,
    ) -> Result<Vec<Application>, AppError> {
        let apps = sqlx::query_as::<_, Application>(
            "SELECT * FROM applications WHERE ($1::text IS NULL OR status = $1) ORDER BY created_at DESC",
        )
        .bind(&filters.status)
        .fetch_all(pool)
        .await?;
        Ok(apps)
    }

    pub async fn get(pool: &PgPool, id: i32) -> Result<Application, AppError> {
        sqlx::query_as::<_, Application>("SELECT * FROM applications WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Application {id} not found")))
    }

    pub async fn create(
        pool: &PgPool,
        input: CreateApplication,
    ) -> Result<Application, AppError> {
        let status = input.status.unwrap_or_else(|| "draft".to_string());
        let app = sqlx::query_as::<_, Application>(
            "INSERT INTO applications (job_id, status, cv_variant, notes) VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(input.job_id)
        .bind(&status)
        .bind(&input.cv_variant)
        .bind(&input.notes)
        .fetch_one(pool)
        .await?;
        Ok(app)
    }

    pub async fn update(
        pool: &PgPool,
        id: i32,
        input: UpdateApplication,
    ) -> Result<Application, AppError> {
        let existing = Self::get(pool, id).await?;
        let app = sqlx::query_as::<_, Application>(
            "UPDATE applications SET status = $2, cv_variant = $3, applied_at = $4, response_at = $5, notes = $6, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(input.status.unwrap_or(existing.status))
        .bind(input.cv_variant.or(existing.cv_variant))
        .bind(input.applied_at.or(existing.applied_at))
        .bind(input.response_at.or(existing.response_at))
        .bind(input.notes.or(existing.notes))
        .fetch_one(pool)
        .await?;
        Ok(app)
    }

    pub async fn delete(pool: &PgPool, id: i32) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM applications WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Application {id} not found")));
        }
        Ok(())
    }

    pub async fn count_by_status(pool: &PgPool) -> Result<Vec<(String, i64)>, AppError> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT status, COUNT(*) FROM applications GROUP BY status ORDER BY COUNT(*) DESC",
        )
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }
}
