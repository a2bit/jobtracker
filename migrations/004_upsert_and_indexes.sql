-- Phase 5: upsert tracking + missing indexes

-- Track how many jobs were updated (not just new) per collector run
ALTER TABLE collector_runs ADD COLUMN jobs_updated INTEGER DEFAULT 0;

-- Missing indexes for FK lookups (events and applications both query by job_id)
CREATE INDEX IF NOT EXISTS idx_events_job ON events(job_id);
CREATE INDEX IF NOT EXISTS idx_applications_job ON applications(job_id);
