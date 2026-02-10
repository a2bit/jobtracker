# JobTracker

Self-hosted job application tracker portal. REST API for CLI tools and Claude agents, dark-mode web UI for human use.

## Quick Reference

```bash
# Build
just build          # Debug build
just release        # Release build
just check          # Format + clippy
just test           # Run tests
just validate-ci    # Full CI gate

# Run locally (requires PostgreSQL)
export DATABASE_URL=postgresql://jobtracker:password@localhost:5432/jobtracker
just run

# Docker
just docker-build
```

## Architecture

| Layer | Technology |
|-------|-----------|
| Web framework | Axum 0.8 |
| Database | PostgreSQL 17 via CloudNativePG |
| ORM | sqlx 0.8 (runtime queries, FromRow derive) |
| Templates | Askama 0.15 + askama_web (Phase 2) |
| Frontend | htmx + Tailwind CSS dark mode (Phase 2) |
| Auth | Bearer token, SHA-256 hashed, stored in api_tokens table |
| Deployment | K3s homelab, ArgoCD, Tailscale ingress |

## Project Structure

```
src/
  main.rs             # Entry point, subcommand dispatch (serve/collect)
  config.rs           # CLI args + env vars + subcommands (clap derive)
  db.rs               # Pool creation, migrations
  auth.rs             # Token hashing, generation, middleware
  error.rs            # AppError enum with IntoResponse
  models/             # Database models with sqlx::FromRow
    company.rs        # CRUD + find_or_create for companies
    job.rs            # CRUD + upsert (dedup on source+source_id)
    application.rs    # CRUD for applications (with status tracking)
    event.rs          # Timeline events
    collector.rs      # Collector config management
    collector_run.rs  # Queue table: enqueue, claim (SKIP LOCKED), status updates
  routes/
    api/              # JSON REST API (/api/v1/*)
      jobs.rs         # GET/POST/PUT/DELETE
      companies.rs    # GET/POST/PUT
      applications.rs # GET/POST/PUT/DELETE
      events.rs       # GET/POST
      collectors.rs   # GET/PUT + POST trigger (enqueues run)
      tokens.rs       # GET/POST/DELETE
    ui/               # HTML pages (Phase 2)
      admin.rs        # Token mgmt, collector toggle, run trigger, run history
  collectors/         # Job source collectors (Phase 3)
    mod.rs            # CollectedJob struct, JobCollector trait, registry
    runner.rs         # Worker poll loop (claim_next + process)
    hiringcafe.rs     # HiringCafe collector (reqwest + Python CLI fallback)
migrations/           # SQL migration files (001-003)
deploy/               # Kubernetes manifests
```

## API Endpoints

All API endpoints require `Authorization: Bearer <token>` header.

```
GET    /api/v1/jobs?source=&search=&page=&per_page=
POST   /api/v1/jobs
GET    /api/v1/jobs/:id
PUT    /api/v1/jobs/:id
DELETE /api/v1/jobs/:id

GET    /api/v1/applications?status=
POST   /api/v1/applications
GET    /api/v1/applications/:id
PUT    /api/v1/applications/:id
DELETE /api/v1/applications/:id

GET    /api/v1/companies
POST   /api/v1/companies
GET    /api/v1/companies/:id
PUT    /api/v1/companies/:id

GET    /api/v1/events?application_id=&job_id=
POST   /api/v1/events

GET    /api/v1/collectors
PUT    /api/v1/collectors/:name
POST   /api/v1/collectors/:name/run

GET    /api/v1/tokens
POST   /api/v1/tokens
DELETE /api/v1/tokens/:id

GET    /healthz      # Liveness
GET    /readyz       # Readiness (DB check)
```

## Database

PostgreSQL via CloudNativePG on homelab K3s. Tables:

- `companies` - employer info, ATS platform
- `jobs` - listings with source tracking and dedup (source + source_id unique)
- `applications` - status pipeline: draft, applied, interviewing, rejected, offer, accepted, withdrawn
- `events` - timeline entries linked to applications/jobs
- `collectors` - job source configuration (linkedin, indeed, hiringcafe, xing)
- `collector_runs` - job queue + audit log (pending/running/succeeded/failed, SKIP LOCKED)
- `api_tokens` - SHA-256 hashed bearer tokens with expiry

## Deployment

```
GitHub push -> CI (cargo test, docker build) -> Tailscale -> registry-push -> ArgoCD -> K3s
```

Manifests in `deploy/` follow the n8n-cnpg pattern:

- CNPG cluster with B2 backup via Barman Cloud
- ExternalSecrets from 1Password (HomeLab vault)
- Tailscale ingress at jobtracker.tail74b58.ts.net

## Implementation Phases

- **Phase 1** (done): Cargo scaffold, migrations, REST API, auth middleware
- **Phase 2** (done): Askama templates, htmx dashboard, Tailwind dark mode UI
- **Phase 3** (done): HiringCafe collector, DB queue (SKIP LOCKED), worker subcommand, admin run history
- **Phase 4**: CI/CD pipeline, ArgoCD deployment, 1Password setup
- **Phase 5**: Integration with linkedin-job-applier agent, CV pipeline, metrics

## Configuration

| Env Var | Description | Default |
|---------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | (required) |
| `LISTEN_ADDR` | Bind address (serve mode) | `0.0.0.0:8080` |
| `RUN_MIGRATIONS` | Auto-run migrations on start | `true` |
| `POLL_INTERVAL` | Worker poll interval seconds (collect mode) | `10` |
| `RUST_LOG` | Log filter | `jobtracker=info,tower_http=info` |

## Subcommands

```bash
# Web server (default, backward compatible with bare `jobtracker`)
jobtracker serve --listen-addr 0.0.0.0:8080

# Worker loop for a specific collector
jobtracker collect --collector hiringcafe --poll-interval 10
```

## Collector Architecture

Workers and the web server communicate via PostgreSQL (no inter-pod HTTP).

1. User clicks "Run Now" or calls `POST /api/v1/collectors/hiringcafe/run`
2. Web server inserts row into `collector_runs` (status: pending)
3. Worker polls with `SELECT FOR UPDATE SKIP LOCKED`, claims the run
4. Worker executes the collector, upserts jobs (dedup on source+source_id)
5. Worker marks run as succeeded/failed, updates `collectors.last_run_at`

K8s: 1 Deployment for `jobtracker serve`, 1 Deployment per collector worker.

## Conventions

- Runtime sqlx queries (not compile-time macros) since no DB at build time
- `sqlx::FromRow` derive on all model structs
- `AppError` enum converts to JSON error responses
- Bearer token auth on all `/api/v1/*` routes
- Conventional commits: `type(scope): subject`

## Future Features

- Calendar view integrated with the API (application dates, interviews, follow-ups, deadlines)
