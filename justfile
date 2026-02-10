# jobtracker build recipes

kctx := "homelab"
ns := "jobtracker"
pg_pod := "jobtracker-pg-1"

# Default recipe: show available recipes
default:
    @just --list

# Build in debug mode
build:
    cargo build

# Build in release mode
release:
    cargo build --release

# Run all tests
test:
    cargo test

# Fast pre-commit check (format check + clippy)
check:
    cargo fmt -- --check
    cargo clippy -- -D warnings

# Run the server locally (requires DATABASE_URL)
run *ARGS:
    cargo run -- {{ARGS}}

# Format code
fmt:
    cargo fmt

# Run clippy
clippy:
    cargo clippy -- -D warnings

# Full CI gate: fmt + clippy + test
validate-ci:
    cargo fmt -- --check
    cargo clippy -- -D warnings
    cargo test

# Run database migrations (requires DATABASE_URL)
migrate:
    cargo run -- --run-migrations true --listen-addr 127.0.0.1:0

# Build Docker image
docker-build tag="latest":
    docker build -t jobtracker:{{tag}} .

# Clean build artifacts
clean:
    cargo clean

# --- Cluster operations ---

# Run SQL against the cluster database
[no-exit-message]
sql query:
    @kubectl --context {{kctx}} exec {{pg_pod}} -n {{ns}} -c postgres -- psql -U postgres -d {{ns}} -c "{{query}}"

# Enable a collector (starts the worker picking up runs)
collector-enable name="hiringcafe":
    @just sql "UPDATE collectors SET enabled = true, updated_at = NOW() WHERE name = '{{name}}';"
    @echo "Collector '{{name}}' enabled"

# Disable a collector (worker will exit on next poll)
collector-disable name="hiringcafe":
    @just sql "UPDATE collectors SET enabled = false, updated_at = NOW() WHERE name = '{{name}}';"
    @echo "Collector '{{name}}' disabled"

# Show collector status
collector-status:
    @just sql "SELECT name, enabled, last_run_at, last_error FROM collectors ORDER BY name;"

# Enqueue a collector run
collector-run name="hiringcafe":
    @just sql "INSERT INTO collector_runs (collector_name, run_kind) VALUES ('{{name}}', 'manual') RETURNING id, status;"

# Show recent collector runs
runs name="" limit="10":
    @just sql "SELECT id, collector_name, status, jobs_found, jobs_new, jobs_updated, error, finished_at FROM collector_runs WHERE ('{{name}}' = '' OR collector_name = '{{name}}') ORDER BY id DESC LIMIT {{limit}};"

# Show job counts by source
job-counts:
    @just sql "SELECT source, count(*) AS total FROM jobs GROUP BY source ORDER BY total DESC;"

# Restart the worker deployment
worker-restart name="hiringcafe":
    kubectl --context {{kctx}} rollout restart deployment -n {{ns}} jobtracker-worker-{{name}}

# Tail worker logs
worker-logs name="hiringcafe" lines="50":
    kubectl --context {{kctx}} logs -n {{ns}} -l app.kubernetes.io/component=worker-{{name}} --tail={{lines}}

# Follow worker logs
worker-follow name="hiringcafe":
    kubectl --context {{kctx}} logs -n {{ns}} -l app.kubernetes.io/component=worker-{{name}} -f
