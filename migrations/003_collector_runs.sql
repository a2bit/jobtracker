-- Job queue table for collector runs.
-- Workers claim pending runs with SELECT FOR UPDATE SKIP LOCKED.
-- Also serves as audit log for the admin UI.

CREATE TABLE collector_runs (
    id             SERIAL PRIMARY KEY,
    collector_name TEXT NOT NULL REFERENCES collectors(name),
    status         TEXT NOT NULL DEFAULT 'pending',
    run_kind       TEXT NOT NULL DEFAULT 'manual',
    jobs_found     INTEGER DEFAULT 0,
    jobs_new       INTEGER DEFAULT 0,
    error          TEXT,
    requested_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at     TIMESTAMPTZ,
    finished_at    TIMESTAMPTZ
);

CREATE INDEX idx_collector_runs_pending
    ON collector_runs(status) WHERE status = 'pending';
CREATE INDEX idx_collector_runs_collector
    ON collector_runs(collector_name, requested_at DESC);
