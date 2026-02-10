pub mod hiringcafe;
pub mod runner;

use async_trait::async_trait;
use serde::Serialize;

use crate::error::AppError;

/// A job as collected from an external source.
/// Contains company_name (not company_id) since collectors don't know internal IDs.
/// The runner resolves company names to IDs via Company::find_or_create.
#[derive(Debug, Serialize)]
pub struct CollectedJob {
    pub company_name: String,
    pub title: String,
    pub url: Option<String>,
    pub location: Option<String>,
    pub remote_type: Option<String>,
    pub salary_min: Option<i32>,
    pub salary_max: Option<i32>,
    pub salary_currency: Option<String>,
    pub description: Option<String>,
    pub source: String,
    pub source_id: String,
    pub raw_data: Option<serde_json::Value>,
}

/// Trait that all job collectors must implement.
/// Collectors are pure: they fetch jobs and return structured data.
/// Database operations (company resolution, job upsert) are handled by the runner.
#[async_trait]
#[allow(dead_code)]
pub trait JobCollector: Send + Sync {
    /// Human-readable name matching the collectors table entry.
    fn name(&self) -> &str;

    /// Fetch jobs from the external source using the provided JSONB config.
    async fn collect(&self, config: &serde_json::Value) -> Result<Vec<CollectedJob>, AppError>;
}

/// Look up a collector implementation by name.
pub fn get_collector(name: &str) -> Option<Box<dyn JobCollector>> {
    match name {
        "hiringcafe" => Some(Box::new(hiringcafe::HiringCafe)),
        _ => None,
    }
}
