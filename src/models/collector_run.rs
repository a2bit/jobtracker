use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

use crate::error::AppError;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CollectorRun {
    pub id: i32,
    pub collector_name: String,
    pub status: String,
    pub run_kind: String,
    pub jobs_found: Option<i32>,
    pub jobs_new: Option<i32>,
    pub jobs_updated: Option<i32>,
    pub error: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

impl CollectorRun {
    /// Insert a new pending run into the queue.
    pub async fn enqueue(
        pool: &PgPool,
        collector_name: &str,
        run_kind: &str,
    ) -> Result<CollectorRun, AppError> {
        let run = sqlx::query_as::<_, CollectorRun>(
            "INSERT INTO collector_runs (collector_name, run_kind) VALUES ($1, $2) RETURNING *",
        )
        .bind(collector_name)
        .bind(run_kind)
        .fetch_one(pool)
        .await?;
        Ok(run)
    }

    /// Atomically claim the next pending run for this collector.
    /// Uses SELECT FOR UPDATE SKIP LOCKED to allow concurrent workers
    /// without contention.
    pub async fn claim_next(
        pool: &PgPool,
        collector_name: &str,
    ) -> Result<Option<CollectorRun>, AppError> {
        let run = sqlx::query_as::<_, CollectorRun>(
            "UPDATE collector_runs SET status = 'running', started_at = NOW()
             WHERE id = (
                 SELECT id FROM collector_runs
                 WHERE collector_name = $1 AND status = 'pending'
                 ORDER BY requested_at
                 LIMIT 1
                 FOR UPDATE SKIP LOCKED
             )
             RETURNING *",
        )
        .bind(collector_name)
        .fetch_optional(pool)
        .await?;
        Ok(run)
    }

    /// Mark a run as succeeded with job counts.
    pub async fn mark_succeeded(
        pool: &PgPool,
        id: i32,
        jobs_found: i32,
        jobs_new: i32,
        jobs_updated: i32,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE collector_runs SET status = 'succeeded', jobs_found = $2, jobs_new = $3, jobs_updated = $4, finished_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .bind(jobs_found)
        .bind(jobs_new)
        .bind(jobs_updated)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Reset stale "running" runs to "failed" on worker startup.
    /// This handles the case where a worker crashed mid-run.
    pub async fn recover_stale(pool: &PgPool, collector_name: &str) -> Result<u64, AppError> {
        let result = sqlx::query(
            "UPDATE collector_runs SET status = 'failed', error = 'Worker crashed (stale recovery)', finished_at = NOW()
             WHERE collector_name = $1 AND status = 'running'",
        )
        .bind(collector_name)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Mark a run as failed with an error message.
    pub async fn mark_failed(pool: &PgPool, id: i32, error: &str) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE collector_runs SET status = 'failed', error = $2, finished_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .bind(error)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Get recent runs, optionally filtered by collector name.
    pub async fn recent(
        pool: &PgPool,
        collector_name: Option<&str>,
        limit: i64,
    ) -> Result<Vec<CollectorRun>, AppError> {
        let runs = sqlx::query_as::<_, CollectorRun>(
            "SELECT * FROM collector_runs WHERE ($1::text IS NULL OR collector_name = $1) ORDER BY requested_at DESC LIMIT $2",
        )
        .bind(collector_name)
        .bind(limit)
        .fetch_all(pool)
        .await?;
        Ok(runs)
    }
}
