# Dockerfile for Rust API server

# Build stage
FROM rust:latest as builder

WORKDIR /app

# Copy workspace root Cargo.toml
COPY Cargo.toml ./

# Copy all package Cargo.toml files for dependency resolution
COPY packages/server/Cargo.toml ./packages/server/
COPY packages/seesaw-rs/Cargo.toml ./packages/seesaw-rs/
COPY packages/twilio-rs/Cargo.toml ./packages/twilio-rs/
COPY packages/dev-cli/Cargo.toml ./packages/dev-cli/

# Create dummy src files for all packages to cache dependencies
RUN mkdir -p packages/server/src && \
    echo "fn main() {}" > packages/server/src/lib.rs && \
    mkdir -p packages/server/src/server && \
    echo "fn main() {}" > packages/server/src/server/main.rs && \
    mkdir -p packages/server/src/bin && \
    echo "fn main() {}" > packages/server/src/bin/seed_organizations.rs && \
    mkdir -p packages/seesaw-rs/src && \
    echo "fn main() {}" > packages/seesaw-rs/src/lib.rs && \
    mkdir -p packages/twilio-rs/src && \
    echo "fn main() {}" > packages/twilio-rs/src/lib.rs && \
    mkdir -p packages/dev-cli/src && \
    echo "fn main() {}" > packages/dev-cli/src/main.rs

# Build dependencies (cached layer)
RUN cargo build --release --manifest-path packages/server/Cargo.toml --bin server

# Remove dummy src files (keep dev-cli dummy since we don't build it)
RUN rm -rf packages/server/src packages/seesaw-rs/src packages/twilio-rs/src

# Copy real source code for packages we actually build
COPY packages/server/src ./packages/server/src
COPY packages/server/migrations ./packages/server/migrations
COPY packages/seesaw-rs/src ./packages/seesaw-rs/src
COPY packages/twilio-rs/src ./packages/twilio-rs/src

# Build the actual application
RUN cargo build --release --manifest-path packages/server/Cargo.toml --bin server

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
