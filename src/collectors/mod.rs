// Collector module - Phase 3 implementation.
// Defines the trait and runner for modular job source collectors.

use async_trait::async_trait;

use crate::error::AppError;
use crate::models::job::CreateJob;

/// Trait that all job collectors must implement.
/// Each collector fetches jobs from an external source and returns them
/// as a vector of CreateJob structs ready for database insertion.
#[async_trait]
#[allow(dead_code)]
pub trait JobCollector: Send + Sync {
    /// Human-readable name matching the collectors table entry.
    fn name(&self) -> &str;

    /// Fetch jobs from the external source using the provided config.
    async fn collect(&self, config: &serde_json::Value) -> Result<Vec<CreateJob>, AppError>;
}
