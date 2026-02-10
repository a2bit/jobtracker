use axum::Json;
use axum::extract::{Path, State};
use sqlx::PgPool;

use crate::error::AppError;
use crate::models::company::{Company, CreateCompany, UpdateCompany};

pub async fn list(State(pool): State<PgPool>) -> Result<Json<Vec<Company>>, AppError> {
    let companies = Company::list(&pool).await?;
    Ok(Json(companies))
}

pub async fn get(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> Result<Json<Company>, AppError> {
    let company = Company::get(&pool, id).await?;
    Ok(Json(company))
}

pub async fn create(
    State(pool): State<PgPool>,
    Json(input): Json<CreateCompany>,
) -> Result<Json<Company>, AppError> {
    let company = Company::create(&pool, input).await?;
    Ok(Json(company))
}

pub async fn update(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    Json(input): Json<UpdateCompany>,
) -> Result<Json<Company>, AppError> {
    let company = Company::update(&pool, id, input).await?;
    Ok(Json(company))
}
