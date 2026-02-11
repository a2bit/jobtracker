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
collectors/
  hiringcafe.py       # Python collector (curl_cffi + impersonate)
  Dockerfile          # Python collector image
deploy/
  cronjob-collector-hiringcafe.yaml  # K8s CronJob (every 6h)
  ...                 # Kubernetes manifests
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

POST   /api/v1/collect/ingest          # Batch ingest (for Python collectors)
POST   /api/v1/jobs/upsert             # Single job upsert (dedup on source+source_id)
POST   /api/v1/companies/find-or-create # Resolve company name to ID

GET    /healthz      # Liveness
GET    /readyz       # Readiness (DB check)
```

## Database

PostgreSQL 17 via CloudNativePG on homelab K3s.

### Direct Access

```bash
# Query via kubectl exec (no psql needed locally)
kubectl --context homelab exec jobtracker-pg-1 -n jobtracker -c postgres \
  -- psql -U postgres -d jobtracker -c "SELECT ..."

# Interactive session (requires -it, won't work in Claude Code)
kubectl --context homelab exec -it jobtracker-pg-1 -n jobtracker -c postgres \
  -- psql -U postgres -d jobtracker
```

Use `-U postgres` (superuser). Peer auth rejects `-U jobtracker` via unix socket.

### Tables

- `companies` - employer info, ATS platform
- `jobs` - listings with source tracking and dedup (source + source_id unique)
- `applications` - status pipeline: draft, applied, interviewing, rejected, offer, accepted, withdrawn
- `events` - timeline entries linked to applications/jobs
- `collectors` - job source configuration (linkedin, indeed, hiringcafe, xing)
- `collector_runs` - job queue + audit log (pending/running/succeeded/failed, SKIP LOCKED)
- `api_tokens` - SHA-256 hashed bearer tokens with expiry

### Collector Enable/Disable

Collectors default to `enabled = false` on first migration. The worker exits immediately
if its collector is disabled. Enable via SQL:

```bash
kubectl --context homelab exec jobtracker-pg-1 -n jobtracker -c postgres \
  -- psql -U postgres -d jobtracker -c "UPDATE collectors SET enabled = true WHERE name = 'hiringcafe';"
```

## Deployment

```
GitHub push -> CI on self-hosted ARC runners (homelab K3s, DinD mode)
  -> cargo test, docker build (2 images: jobtracker + jobtracker-collector)
  -> registry-push.tail74b58.ts.net
  -> ArgoCD Image Updater polls registry (2min) -> git write-back newTag
  -> ArgoCD auto-sync -> K3s
```

CI runs on `homelab-runners` label (ARC runner set, a2bit org). Triggered on
push to main and workflow_dispatch only — no `pull_request` trigger (security:
public repo with self-hosted runners).

### Deploy Manifests (`deploy/`)

| File | Purpose |
|------|---------|
| `external-secret-app.yaml` | Constructs `DATABASE_URL` URI from 1Password password |
| `external-secret-pg.yaml` | Provides username/password for CNPG bootstrap |
| `external-secret-b2.yaml` | Backblaze B2 credentials for backup |
| `objectstore-b2.yaml` | Barman Cloud object store config |
| `cnpg-cluster.yaml` | PostgreSQL cluster (1 instance, 5Gi) |
| `deployment.yaml` | Web server (`serve` subcommand) |
| `deployment-worker.yaml` | HiringCafe worker (`collect` subcommand) |
| `service.yaml` | ClusterIP on port 80 -> 8080 |
| `ingress.yaml` | Tailscale ingress at `jobtracker.tail74b58.ts.net` |
| `kustomization.yaml` | Kustomize base with image tag management |

### CNPG Secret Pattern

CNPG does NOT auto-create a `-app` secret with connection URI. Two ExternalSecrets are needed:

1. `jobtracker-pg-credentials`: provides `username` + `password` for CNPG bootstrap
2. `jobtracker-pg-app`: constructs full `uri` for `DATABASE_URL` env var

The URI template: `postgresql://jobtracker:{{ .password }}@jobtracker-pg-rw.jobtracker.svc:5432/jobtracker`

### ArgoCD Image Updater

Image tag automation is configured in homelab-gitops repo:

- `argocd-image-updater.yaml`: Helm chart v1.1.0 with homelab registry config
- `imageupdater-jobtracker.yaml`: CRD polling `registry-push.tail74b58.ts.net/jobtracker`
  with `newest-build` strategy and `allowTags: regexp:^[0-9a-f]{7}$` (git short SHAs)
- Write-back: commits `newTag` changes to `deploy/kustomization.yaml` via git

### Image Tag

Managed in `deploy/kustomization.yaml` under `images[].newTag`. Updated automatically
by ArgoCD Image Updater or manually for debugging. CI pushes tags matching 7-char git
short SHA (e.g., `a85837d`).

## Implementation Phases

- **Phase 1** (done): Cargo scaffold, migrations, REST API, auth middleware
- **Phase 2** (done): Askama templates, htmx dashboard, Tailwind dark mode UI
- **Phase 3** (done): HiringCafe collector, DB queue (SKIP LOCKED), worker subcommand, admin run history
- **Phase 4** (done): CI/CD pipeline, ArgoCD deployment, Image Updater, 1Password secrets, Slack notifications
- **Phase 5** (done): Upsert-on-conflict updates, collector pagination (5 pages),
  HTTP timeouts, graceful shutdown, stale run recovery, UI pagination,
  HTML error pages, missing DB indexes
- **Phase 6** (done): Batch ingest API, Python collector (curl_cffi + browser
  impersonation), K8s CronJob (every 6h), CI builds two images

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
