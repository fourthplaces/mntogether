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

# Create dummy src files for all packages to cache dependencies
RUN mkdir -p packages/server/src && \
    echo "fn main() {}" > packages/server/src/lib.rs && \
    mkdir -p packages/server/src/server && \
    echo "fn main() {}" > packages/server/src/server/main.rs && \
    mkdir -p packages/server/src/bin && \
    echo "fn main() {}" > packages/server/src/bin/seed_organizations.rs && \
    echo "fn main() {}" > packages/server/src/bin/generate_embeddings.rs && \
    echo "fn main() {}" > packages/server/src/bin/migrate_cli.rs && \
    mkdir -p packages/twilio-rs/src && \
    echo "fn main() {}" > packages/twilio-rs/src/lib.rs && \
    mkdir -p packages/extraction/src && \
    echo "fn main() {}" > packages/extraction/src/lib.rs && \
    mkdir -p packages/openai-client/src && \
    echo "fn main() {}" > packages/openai-client/src/lib.rs

# Build dependencies (cached layer)
RUN cargo build --release --manifest-path packages/server/Cargo.toml --bin server

# Remove dummy src files and compiled artifacts so cargo detects source changes
RUN rm -rf packages/server/src packages/twilio-rs/src packages/extraction/src packages/openai-client/src && \
    rm -f target/release/server target/release/deps/server_core-* target/release/deps/libserver_core-*

# Copy real source code for packages we actually build
COPY packages/server/src ./packages/server/src
COPY packages/server/migrations ./packages/server/migrations
COPY packages/twilio-rs/src ./packages/twilio-rs/src
COPY packages/extraction/src ./packages/extraction/src
COPY packages/openai-client/src ./packages/openai-client/src

# Build the actual application (touch lib.rs to force recompilation)
RUN touch packages/server/src/lib.rs packages/twilio-rs/src/lib.rs packages/extraction/src/lib.rs packages/openai-client/src/lib.rs && \
    cargo build --release --manifest-path packages/server/Cargo.toml --bin server

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

# Copy binary from builder
COPY --from=builder /app/target/release/server /usr/local/bin/server

# Copy migrations for runtime
COPY --from=builder /app/packages/server/migrations ./migrations

EXPOSE 8080

CMD ["server"]
