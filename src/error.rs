use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// HTML error wrapper for UI routes. Returns styled error pages instead of JSON.
pub struct HtmlError(pub AppError);

impl IntoResponse for HtmlError {
    fn into_response(self) -> Response {
        let (status, message) = match &self.0 {
            AppError::Database(e) => {
                tracing::error!("Database error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong".to_string(),
                )
            }
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {msg}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong".to_string(),
                )
            }
        };
        let html = format!(
            r#"<!doctype html><html><head><title>Error {code}</title>
<script src="https://cdn.tailwindcss.com"></script></head>
<body class="bg-[#1e1e2e] text-[#cdd6f4] flex items-center justify-center min-h-screen">
<div class="text-center">
  <div class="text-6xl font-bold text-[#f38ba8]">{code}</div>
  <div class="mt-4 text-lg">{message}</div>
  <a href="/" class="mt-6 inline-block text-[#89b4fa] hover:underline">Back to dashboard</a>
</div></body></html>"#,
            code = status.as_u16(),
            message = message,
        );
        (status, axum::response::Html(html)).into_response()
    }
}

impl From<AppError> for HtmlError {
    fn from(e: AppError) -> Self {
        HtmlError(e)
    }
}

impl From<sqlx::Error> for HtmlError {
    fn from(e: sqlx::Error) -> Self {
        HtmlError(AppError::Database(e))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Internal error: {0}")]
    #[allow(dead_code)]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Database(e) => {
                if let sqlx::Error::Database(db_err) = e
                    && db_err.is_unique_violation()
                {
                    return (
                        StatusCode::CONFLICT,
                        axum::Json(json!({ "error": "Resource already exists" })),
                    )
                        .into_response();
                }
                tracing::error!("Database error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {msg}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        let body = axum::Json(json!({ "error": message }));
        (status, body).into_response()
    }
}
