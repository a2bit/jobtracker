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
  main.rs             # Entry point, config, server startup
  config.rs           # CLI args + env vars (clap)
  db.rs               # Pool creation, migrations
  auth.rs             # Token hashing, generation, middleware
  error.rs            # AppError enum with IntoResponse
  models/             # Database models with sqlx::FromRow
    company.rs        # CRUD for companies
    job.rs            # CRUD for jobs (with search/filter)
    application.rs    # CRUD for applications (with status tracking)
    event.rs          # Timeline events
    collector.rs      # Collector config management
  routes/
    api/              # JSON REST API (/api/v1/*)
      jobs.rs         # GET/POST/PUT/DELETE
      companies.rs    # GET/POST/PUT
      applications.rs # GET/POST/PUT/DELETE
      events.rs       # GET/POST
      collectors.rs   # GET/PUT + POST trigger
      tokens.rs       # GET/POST/DELETE
    ui/               # HTML pages (Phase 2)
  collectors/         # Job source collectors (Phase 3)
migrations/           # SQL migration files
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
- **Phase 2** (current): Askama templates, htmx dashboard, Tailwind dark mode UI
- **Phase 3**: Job collectors (LinkedIn, Indeed, HiringCafe, Xing)
- **Phase 4**: CI/CD pipeline, ArgoCD deployment, 1Password setup
- **Phase 5**: Integration with linkedin-job-applier agent, CV pipeline, metrics

## Configuration

| Env Var | Description | Default |
|---------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | (required) |
| `LISTEN_ADDR` | Bind address | `0.0.0.0:8080` |
| `RUN_MIGRATIONS` | Auto-run migrations on start | `true` |
| `RUST_LOG` | Log filter | `jobtracker=info,tower_http=info` |

## Conventions

- Runtime sqlx queries (not compile-time macros) since no DB at build time
- `sqlx::FromRow` derive on all model structs
- `AppError` enum converts to JSON error responses
- Bearer token auth on all `/api/v1/*` routes
- Conventional commits: `type(scope): subject`

## Future Features

- Calendar view integrated with the API (application dates, interviews, follow-ups, deadlines)
