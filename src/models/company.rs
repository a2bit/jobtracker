use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::AppError;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Company {
    pub id: i32,
    pub name: String,
    pub website: Option<String>,
    pub careers_url: Option<String>,
    pub ats_platform: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCompany {
    pub name: String,
    pub website: Option<String>,
    pub careers_url: Option<String>,
    pub ats_platform: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCompany {
    pub name: Option<String>,
    pub website: Option<String>,
    pub careers_url: Option<String>,
    pub ats_platform: Option<String>,
    pub notes: Option<String>,
}

impl Company {
    pub async fn list(pool: &PgPool) -> Result<Vec<Company>, AppError> {
        let companies =
            sqlx::query_as::<_, Company>("SELECT * FROM companies ORDER BY name")
                .fetch_all(pool)
                .await?;
        Ok(companies)
    }

    pub async fn get(pool: &PgPool, id: i32) -> Result<Company, AppError> {
        sqlx::query_as::<_, Company>("SELECT * FROM companies WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Company {id} not found")))
    }

    pub async fn create(pool: &PgPool, input: CreateCompany) -> Result<Company, AppError> {
        let company = sqlx::query_as::<_, Company>(
            "INSERT INTO companies (name, website, careers_url, ats_platform, notes) VALUES ($1, $2, $3, $4, $5) RETURNING *",
        )
        .bind(&input.name)
        .bind(&input.website)
        .bind(&input.careers_url)
        .bind(&input.ats_platform)
        .bind(&input.notes)
        .fetch_one(pool)
        .await?;
        Ok(company)
    }

    pub async fn update(
        pool: &PgPool,
        id: i32,
        input: UpdateCompany,
    ) -> Result<Company, AppError> {
        let existing = Self::get(pool, id).await?;
        let company = sqlx::query_as::<_, Company>(
            "UPDATE companies SET name = $2, website = $3, careers_url = $4, ats_platform = $5, notes = $6, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(input.name.unwrap_or(existing.name))
        .bind(input.website.or(existing.website))
        .bind(input.careers_url.or(existing.careers_url))
        .bind(input.ats_platform.or(existing.ats_platform))
        .bind(input.notes.or(existing.notes))
        .fetch_one(pool)
        .await?;
        Ok(company)
    }
}
