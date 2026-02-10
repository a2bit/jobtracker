use axum::Json;
use axum::extract::{Path, State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::auth::{generate_token, hash_token};
use crate::error::AppError;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TokenInfo {
    pub id: i32,
    pub name: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateToken {
    pub name: String,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct CreatedToken {
    pub id: i32,
    pub name: String,
    pub token: String,
}

pub async fn list(State(pool): State<PgPool>) -> Result<Json<Vec<TokenInfo>>, AppError> {
    let tokens = sqlx::query_as::<_, TokenInfo>(
        "SELECT id, name, expires_at, created_at, last_used FROM api_tokens ORDER BY created_at DESC",
    )
    .fetch_all(&pool)
    .await?;
    Ok(Json(tokens))
}

pub async fn create(
    State(pool): State<PgPool>,
    Json(input): Json<CreateToken>,
) -> Result<Json<CreatedToken>, AppError> {
    let raw_token = generate_token();
    let token_hash = hash_token(&raw_token);

    let row: (i32,) = sqlx::query_as(
        "INSERT INTO api_tokens (name, token_hash, expires_at) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(&input.name)
    .bind(&token_hash)
    .bind(input.expires_at)
    .fetch_one(&pool)
    .await?;

    Ok(Json(CreatedToken {
        id: row.0,
        name: input.name,
        token: raw_token,
    }))
}

pub async fn revoke(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = sqlx::query("DELETE FROM api_tokens WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Token {id} not found")));
    }
    Ok(Json(serde_json::json!({ "revoked": true })))
}
