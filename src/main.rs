mod auth;
mod collectors;
mod config;
mod db;
mod error;
mod models;
mod routes;

use axum::Router;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use clap::Parser;
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::config::Config;

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

async fn readyz(pool: PgPool) -> impl IntoResponse {
    let result: Result<(i32,), _> = sqlx::query_as("SELECT 1").fetch_one(&pool).await;
    match result {
        Ok(_) => (StatusCode::OK, "ready"),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "not ready"),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("jobtracker=info,tower_http=info")),
        )
        .init();

    let config = Config::parse();

    tracing::info!("Connecting to database...");
    let pool = db::create_pool(&config.database_url).await?;

    if config.run_migrations {
        tracing::info!("Running database migrations...");
        db::run_migrations(&pool).await?;
        tracing::info!("Migrations complete");
    }

    let readyz_pool = pool.clone();
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(move || readyz(readyz_pool.clone())))
        .merge(routes::ui::router(pool.clone()))
        .merge(routes::api::router(pool))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(&config.listen_addr).await?;
    tracing::info!("Listening on {}", config.listen_addr);
    axum::serve(listener, app).await?;

    Ok(())
}
