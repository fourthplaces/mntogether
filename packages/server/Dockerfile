# Dockerfile for Rust API server

# Build stage
FROM rust:latest as builder

WORKDIR /app

# Copy workspace root Cargo.toml
COPY Cargo.toml ./

# Copy all package Cargo.toml files for dependency resolution
COPY packages/server/Cargo.toml ./packages/server/
COPY packages/twilio-rs/Cargo.toml ./packages/twilio-rs/
COPY packages/extraction/Cargo.toml ./packages/extraction/
COPY packages/openai-client/Cargo.toml ./packages/openai-client/
COPY packages/apify-client/Cargo.toml ./packages/apify-client/

# Create dummy src files for all packages to cache dependencies
RUN mkdir -p packages/server/src && \
    echo "fn main() {}" > packages/server/src/lib.rs && \
    mkdir -p packages/server/src/bin && \
    echo "fn main() {}" > packages/server/src/bin/server.rs && \
    echo "fn main() {}" > packages/server/src/bin/generate_embeddings.rs && \
    echo "fn main() {}" > packages/server/src/bin/migrate_cli.rs && \
    echo "fn main() {}" > packages/server/src/bin/run_migrations.rs && \
    mkdir -p packages/twilio-rs/src && \
    echo "fn main() {}" > packages/twilio-rs/src/lib.rs && \
    mkdir -p packages/extraction/src && \
    echo "fn main() {}" > packages/extraction/src/lib.rs && \
    mkdir -p packages/openai-client/src && \
    echo "fn main() {}" > packages/openai-client/src/lib.rs && \
    mkdir -p packages/apify-client/src && \
    echo "fn main() {}" > packages/apify-client/src/lib.rs

# Build dependencies (cached layer)
RUN cargo build --release --manifest-path packages/server/Cargo.toml --bin server

# Remove dummy src files and compiled artifacts so cargo detects source changes
RUN rm -rf packages/server/src packages/twilio-rs/src packages/extraction/src packages/openai-client/src packages/apify-client/src && \
    rm -f target/release/server target/release/run_migrations target/release/deps/server_core-* target/release/deps/libserver_core-*

# Copy real source code for packages we actually build
COPY packages/server/src ./packages/server/src
COPY packages/server/migrations ./packages/server/migrations
COPY packages/twilio-rs/src ./packages/twilio-rs/src
COPY packages/extraction/src ./packages/extraction/src
COPY packages/extraction/migrations ./packages/extraction/migrations
COPY packages/openai-client/src ./packages/openai-client/src
COPY packages/apify-client/src ./packages/apify-client/src

# Build both binaries (touch lib.rs to force recompilation)
RUN touch packages/server/src/lib.rs packages/twilio-rs/src/lib.rs packages/extraction/src/lib.rs packages/openai-client/src/lib.rs packages/apify-client/src/lib.rs && \
    cargo build --release --manifest-path packages/server/Cargo.toml --bin server --bin run_migrations

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    libpq5 && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binaries from builder
COPY --from=builder /app/target/release/server /usr/local/bin/server
COPY --from=builder /app/target/release/run_migrations /usr/local/bin/run_migrations

EXPOSE 8080

CMD ["sh", "-c", "run_migrations && server"]
