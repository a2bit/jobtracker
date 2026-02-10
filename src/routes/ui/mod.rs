mod admin;
mod applications;
mod dashboard;
mod jobs;

use axum::Router;
use axum::routing::{get, post};
use sqlx::PgPool;
use tower_http::services::ServeDir;

pub fn router(pool: PgPool) -> Router {
    Router::new()
        // Dashboard
        .route("/", get(dashboard::index))
        // Jobs
        .route("/jobs", get(jobs::list).post(jobs::create))
        .route("/jobs/{id}", get(jobs::detail))
        // Applications
        .route(
            "/applications",
            get(applications::list).post(applications::create),
        )
        .route(
            "/applications/{id}",
            get(applications::detail).put(applications::update),
        )
        // Events (form submissions from UI, returns partial)
        .route("/events", post(applications::create_event))
        // Admin
        .route("/admin", get(admin::index))
        .route("/admin/tokens", post(admin::create_token))
        .route(
            "/admin/tokens/{id}",
            axum::routing::delete(admin::revoke_token),
        )
        .route(
            "/admin/collectors/{name}/toggle",
            post(admin::toggle_collector),
        )
        .with_state(pool)
        .nest_service("/static", ServeDir::new("static"))
}
