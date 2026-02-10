use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::AppError;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Job {
    pub id: i32,
    pub company_id: i32,
    pub title: String,
    pub url: Option<String>,
    pub location: Option<String>,
    pub remote_type: Option<String>,
    pub salary_min: Option<i32>,
    pub salary_max: Option<i32>,
    pub salary_currency: Option<String>,
    pub description: Option<String>,
    pub requirements: Option<String>,
    pub source: String,
    pub source_id: Option<String>,
    pub found_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub raw_data: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateJob {
    pub company_id: i32,
    pub title: String,
    pub url: Option<String>,
    pub location: Option<String>,
    pub remote_type: Option<String>,
    pub salary_min: Option<i32>,
    pub salary_max: Option<i32>,
    pub salary_currency: Option<String>,
    pub description: Option<String>,
    pub requirements: Option<String>,
    pub source: String,
    pub source_id: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub raw_data: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateJob {
    pub title: Option<String>,
    pub url: Option<String>,
    pub location: Option<String>,
    pub remote_type: Option<String>,
    pub salary_min: Option<i32>,
    pub salary_max: Option<i32>,
    pub description: Option<String>,
    pub requirements: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct JobFilters {
    pub source: Option<String>,
    pub search: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl Job {
    pub async fn list(pool: &PgPool, filters: &JobFilters) -> Result<Vec<Job>, AppError> {
        let per_page = filters.per_page.unwrap_or(50).min(100);
        let offset = (filters.page.unwrap_or(1) - 1).max(0) * per_page;

        let jobs = sqlx::query_as::<_, Job>(
            "SELECT * FROM jobs WHERE ($1::text IS NULL OR source = $1) AND ($2::text IS NULL OR title ILIKE '%' || $2 || '%') ORDER BY found_at DESC LIMIT $3 OFFSET $4",
        )
        .bind(&filters.source)
        .bind(&filters.search)
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await?;
        Ok(jobs)
    }

    pub async fn get(pool: &PgPool, id: i32) -> Result<Job, AppError> {
        sqlx::query_as::<_, Job>("SELECT * FROM jobs WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Job {id} not found")))
    }

    pub async fn create(pool: &PgPool, input: CreateJob) -> Result<Job, AppError> {
        let job = sqlx::query_as::<_, Job>(
            "INSERT INTO jobs (company_id, title, url, location, remote_type, salary_min, salary_max, salary_currency, description, requirements, source, source_id, expires_at, raw_data) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) RETURNING *",
        )
        .bind(input.company_id)
        .bind(&input.title)
        .bind(&input.url)
        .bind(&input.location)
        .bind(&input.remote_type)
        .bind(input.salary_min)
        .bind(input.salary_max)
        .bind(&input.salary_currency)
        .bind(&input.description)
        .bind(&input.requirements)
        .bind(&input.source)
        .bind(&input.source_id)
        .bind(input.expires_at)
        .bind(&input.raw_data)
        .fetch_one(pool)
        .await?;
        Ok(job)
    }

    pub async fn update(pool: &PgPool, id: i32, input: UpdateJob) -> Result<Job, AppError> {
        let existing = Self::get(pool, id).await?;
        let job = sqlx::query_as::<_, Job>(
            "UPDATE jobs SET title = $2, url = $3, location = $4, remote_type = $5, salary_min = $6, salary_max = $7, description = $8, requirements = $9, expires_at = $10, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(input.title.unwrap_or(existing.title))
        .bind(input.url.or(existing.url))
        .bind(input.location.or(existing.location))
        .bind(input.remote_type.or(existing.remote_type))
        .bind(input.salary_min.or(existing.salary_min))
        .bind(input.salary_max.or(existing.salary_max))
        .bind(input.description.or(existing.description))
        .bind(input.requirements.or(existing.requirements))
        .bind(input.expires_at.or(existing.expires_at))
        .fetch_one(pool)
        .await?;
        Ok(job)
    }

    pub async fn delete(pool: &PgPool, id: i32) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM jobs WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Job {id} not found")));
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn count(pool: &PgPool) -> Result<i64, AppError> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM jobs")
            .fetch_one(pool)
            .await?;
        Ok(row.0)
    }
}
