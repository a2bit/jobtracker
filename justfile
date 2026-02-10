# jobtracker build recipes

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
