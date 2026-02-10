-- Initial schema for jobtracker
-- Creates all core tables for job tracking, applications, and collector configuration.

CREATE TABLE companies (
    id          SERIAL PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    website     TEXT,
    careers_url TEXT,
    ats_platform TEXT,
    notes       TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE jobs (
    id              SERIAL PRIMARY KEY,
    company_id      INTEGER NOT NULL REFERENCES companies(id),
    title           TEXT NOT NULL,
    url             TEXT,
    location        TEXT,
    remote_type     TEXT,
    salary_min      INTEGER,
    salary_max      INTEGER,
    salary_currency TEXT DEFAULT 'EUR',
    description     TEXT,
    requirements    TEXT,
    source          TEXT NOT NULL,
    source_id       TEXT,
    found_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ,
    raw_data        JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(source, source_id)
);

CREATE TABLE applications (
    id              SERIAL PRIMARY KEY,
    job_id          INTEGER NOT NULL REFERENCES jobs(id),
    status          TEXT NOT NULL DEFAULT 'draft',
    cv_variant      TEXT,
    applied_at      TIMESTAMPTZ,
    response_at     TIMESTAMPTZ,
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE events (
    id              SERIAL PRIMARY KEY,
    application_id  INTEGER REFERENCES applications(id),
    job_id          INTEGER REFERENCES jobs(id),
    event_type      TEXT NOT NULL,
    description     TEXT,
    occurred_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE collectors (
    id          SERIAL PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    enabled     BOOLEAN NOT NULL DEFAULT true,
    config      JSONB NOT NULL DEFAULT '{}',
    last_run_at TIMESTAMPTZ,
    last_error  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE api_tokens (
    id          SERIAL PRIMARY KEY,
    name        TEXT NOT NULL,
    token_hash  TEXT NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used   TIMESTAMPTZ
);

-- Indexes
CREATE INDEX idx_jobs_source ON jobs(source);
CREATE INDEX idx_jobs_company ON jobs(company_id);
CREATE INDEX idx_jobs_found_at ON jobs(found_at DESC);
CREATE INDEX idx_applications_status ON applications(status);
CREATE INDEX idx_events_application ON events(application_id);
CREATE INDEX idx_events_occurred ON events(occurred_at DESC);

-- Seed default collectors
INSERT INTO collectors (name, enabled, config) VALUES
    ('linkedin', false, '{"search_terms": [], "location": "Germany", "remote": true}'),
    ('indeed', false, '{"search_terms": [], "location": "Germany", "remote": true}'),
    ('hiringcafe', false, '{"search_terms": [], "location": "Germany"}'),
    ('xing', false, '{"search_terms": [], "location": "Germany"}');
