use axum::extract::{Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::error::AppError;

/// Hash a raw API token for storage/lookup.
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Generate a new random API token.
pub fn generate_token() -> String {
    use rand::Rng;
    let bytes: [u8; 32] = rand::rng().random();
    hex::encode(bytes)
}

/// Middleware that validates Bearer token against api_tokens table.
pub async fn require_api_token(
    State(pool): State<PgPool>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized)?;

    let token_hash = hash_token(token);

    let row: Option<(bool,)> = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM api_tokens WHERE token_hash = $1 AND (expires_at IS NULL OR expires_at > NOW()))",
    )
    .bind(&token_hash)
    .fetch_optional(&pool)
    .await?;

    let exists = row.map(|r| r.0).unwrap_or(false);
    if !exists {
        return Err(AppError::Unauthorized);
    }

    // Update last_used timestamp (fire and forget)
    let pool_clone = pool.clone();
    let hash_clone = token_hash.clone();
    tokio::spawn(async move {
        let _ = sqlx::query("UPDATE api_tokens SET last_used = NOW() WHERE token_hash = $1")
            .bind(&hash_clone)
            .execute(&pool_clone)
            .await;
    });

    Ok(next.run(request).await)
}
