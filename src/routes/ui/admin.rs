use askama::Template;
use axum::Form;
use axum::extract::{Path, State};
use axum::response::{Html, Redirect};
use serde::Deserialize;
use sqlx::PgPool;

use crate::auth::{generate_token, hash_token};
use crate::error::AppError;
use crate::models::collector::Collector;
use crate::routes::api::tokens::TokenInfo;

#[derive(Template)]
#[template(path = "admin/index.html")]
struct AdminTemplate {
    tokens: Vec<TokenInfo>,
    collectors: Vec<Collector>,
}

pub async fn index(State(pool): State<PgPool>) -> Result<Html<String>, AppError> {
    let tokens = sqlx::query_as::<_, TokenInfo>(
        "SELECT id, name, expires_at, created_at, last_used FROM api_tokens ORDER BY created_at DESC",
    )
    .fetch_all(&pool)
    .await?;

    let collectors = Collector::list(&pool).await?;

    let tmpl = AdminTemplate { tokens, collectors };
    Ok(Html(
        tmpl.render()
            .map_err(|e| AppError::Internal(e.to_string()))?,
    ))
}

#[derive(Debug, Deserialize)]
pub struct CreateTokenForm {
    pub name: String,
}

pub async fn create_token(
    State(pool): State<PgPool>,
    Form(input): Form<CreateTokenForm>,
) -> Result<Html<String>, AppError> {
    let raw_token = generate_token();
    let token_hash = hash_token(&raw_token);

    sqlx::query("INSERT INTO api_tokens (name, token_hash) VALUES ($1, $2)")
        .bind(&input.name)
        .bind(&token_hash)
        .execute(&pool)
        .await?;

    // Return HTML fragment showing the new token (only shown once)
    Ok(Html(format!(
        r#"<div class="bg-status-accepted/10 border border-status-accepted/30 rounded-lg p-4 text-sm">
            <div class="font-medium text-status-accepted mb-1">Token created!</div>
            <div class="text-text-muted mb-2">Copy this token now. It will not be shown again.</div>
            <code class="block bg-surface p-2 rounded text-xs break-all select-all">{raw_token}</code>
        </div>"#
    )))
}

pub async fn revoke_token(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> Result<Redirect, AppError> {
    sqlx::query("DELETE FROM api_tokens WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await?;
    Ok(Redirect::to("/admin"))
}

pub async fn toggle_collector(
    State(pool): State<PgPool>,
    Path(name): Path<String>,
) -> Result<Redirect, AppError> {
    sqlx::query("UPDATE collectors SET enabled = NOT enabled, updated_at = NOW() WHERE name = $1")
        .bind(&name)
        .execute(&pool)
        .await?;
    Ok(Redirect::to("/admin"))
}
