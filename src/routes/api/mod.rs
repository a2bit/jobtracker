pub mod applications;
pub mod collectors;
pub mod companies;
pub mod events;
pub mod jobs;
pub mod tokens;

use axum::Router;
use axum::middleware;
use axum::routing::{delete, get, post, put};
use sqlx::PgPool;

use crate::auth::require_api_token;

pub fn router(pool: PgPool) -> Router {
    let protected = Router::new()
        // Jobs
        .route("/jobs", get(jobs::list).post(jobs::create))
        .route(
            "/jobs/{id}",
            get(jobs::get).put(jobs::update).delete(jobs::delete),
        )
        // Applications
        .route(
            "/applications",
            get(applications::list).post(applications::create),
        )
        .route(
            "/applications/{id}",
            get(applications::get)
                .put(applications::update)
                .delete(applications::delete),
        )
        // Companies
        .route("/companies", get(companies::list).post(companies::create))
        .route(
            "/companies/{id}",
            get(companies::get).put(companies::update),
        )
        // Events
        .route("/events", get(events::list).post(events::create))
        // Collectors
        .route("/collectors", get(collectors::list))
        .route("/collectors/{name}", put(collectors::update))
        .route("/collectors/{name}/run", post(collectors::trigger_run))
        // Tokens
        .route("/tokens", get(tokens::list).post(tokens::create))
        .route("/tokens/{id}", delete(tokens::revoke))
        .layer(middleware::from_fn_with_state(
            pool.clone(),
            require_api_token,
        ))
        .with_state(pool);

    Router::new().nest("/api/v1", protected)
}
