# Multi-stage build for jobtracker
# Stage 1: Build the Rust binary
FROM rust:1.88-bookworm AS builder

WORKDIR /app

# Cache dependencies by building them first
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs
RUN cargo build --release && rm -rf src

# Build the actual application
COPY src/ src/
COPY migrations/ migrations/
COPY templates/ templates/
RUN touch src/main.rs && cargo build --release

# Stage 2: Minimal runtime image
FROM gcr.io/distroless/cc-debian12:nonroot

WORKDIR /app
COPY --from=builder /app/target/release/jobtracker /app/
COPY --from=builder /app/migrations/ /app/migrations/
COPY --from=builder /app/templates/ /app/templates/
COPY static/ /app/static/

EXPOSE 8080

ENTRYPOINT ["/app/jobtracker"]
