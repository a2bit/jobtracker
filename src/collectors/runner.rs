use std::time::Duration;

use sqlx::PgPool;

use crate::collectors::{CollectedJob, get_collector};
use crate::models::collector::Collector;
use crate::models::collector_run::CollectorRun;
use crate::models::company::Company;
use crate::models::job::{CreateJob, Job};

/// Main worker loop: poll for pending runs and process them.
/// Recovers stale runs on startup and exits gracefully on SIGTERM/SIGINT.
pub async fn run(pool: PgPool, collector_name: &str, poll_interval: u64) -> anyhow::Result<()> {
    let collector_impl = get_collector(collector_name)
        .ok_or_else(|| anyhow::anyhow!("Unknown collector: {collector_name}"))?;

    let collector = Collector::get_by_name(&pool, collector_name).await?;
    if !collector.enabled {
        anyhow::bail!("Collector '{collector_name}' is disabled");
    }

    // Recover any runs left in "running" state from a previous crash
    let stale = CollectorRun::recover_stale(&pool, collector_name).await?;
    if stale > 0 {
        tracing::warn!("Recovered {stale} stale 'running' runs for '{collector_name}'");
    }

    tracing::info!(
        "Worker started for collector '{collector_name}', polling every {poll_interval}s"
    );

    loop {
        tokio::select! {
            biased;
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Shutdown signal received, exiting gracefully");
                break;
            }
            result = async {
                if let Some(run) = CollectorRun::claim_next(&pool, collector_name).await? {
                    tracing::info!("Claimed run {} for '{collector_name}'", run.id);
                    process_run(&pool, &*collector_impl, &run).await;
                }
                tokio::time::sleep(Duration::from_secs(poll_interval)).await;
                Ok::<(), anyhow::Error>(())
            } => {
                result?;
            }
        }
    }

    Ok(())
}

async fn process_run(pool: &PgPool, collector: &dyn super::JobCollector, run: &CollectorRun) {
    let config = match Collector::get_by_name(pool, &run.collector_name).await {
        Ok(c) => c.config,
        Err(e) => {
            let msg = format!("Failed to load collector config: {e}");
            tracing::error!("{msg}");
            let _ = CollectorRun::mark_failed(pool, run.id, &msg).await;
            return;
        }
    };

    match collector.collect(&config).await {
        Ok(jobs) => {
            let (found, new, updated) = upsert_jobs(pool, jobs).await;
            tracing::info!(
                "Run {} completed: {found} found, {new} new, {updated} updated",
                run.id
            );
            let _ = CollectorRun::mark_succeeded(pool, run.id, found, new, updated).await;
            let _ = Collector::record_run(pool, &run.collector_name, None).await;
        }
        Err(e) => {
            let error = e.to_string();
            tracing::error!("Run {} failed: {error}", run.id);
            let _ = CollectorRun::mark_failed(pool, run.id, &error).await;
            let _ = Collector::record_run(pool, &run.collector_name, Some(&error)).await;
        }
    }
}

async fn upsert_jobs(pool: &PgPool, jobs: Vec<CollectedJob>) -> (i32, i32, i32) {
    let found = jobs.len() as i32;
    let mut new = 0;
    let mut updated = 0;

    for collected in jobs {
        let company = match Company::find_or_create(pool, &collected.company_name).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    "Failed to resolve company '{}': {e}",
                    collected.company_name
                );
                continue;
            }
        };

        let input = CreateJob {
            company_id: company.id,
            title: collected.title,
            url: collected.url,
            location: collected.location,
            remote_type: collected.remote_type,
            salary_min: collected.salary_min,
            salary_max: collected.salary_max,
            salary_currency: collected.salary_currency,
            description: collected.description,
            requirements: None,
            source: collected.source,
            source_id: Some(collected.source_id),
            expires_at: None,
            raw_data: collected.raw_data,
        };

        match Job::upsert(pool, input).await {
            Ok((_job, was_inserted)) => {
                if was_inserted {
                    new += 1;
                } else {
                    updated += 1;
                }
            }
            Err(e) => {
                tracing::warn!("Failed to upsert job: {e}");
            }
        }
    }

    (found, new, updated)
}
