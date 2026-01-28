# Dockerfile for Rust API server

# Build stage
FROM rust:1.83 as builder

WORKDIR /app

# Copy manifest files
COPY Cargo.toml ./

# Create dummy src to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/lib.rs && \
    mkdir -p src/server && \
    echo "fn main() {}" > src/server/main.rs

# Build dependencies (cached layer)
RUN cargo build --release

# Remove dummy src
RUN rm -rf src

# Copy real source code
COPY src ./src
COPY migrations ./migrations

# Build the actual application
RUN cargo build --release --bin api

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
COPY --from=builder /app/target/release/api /usr/local/bin/api

# Copy migrations for runtime
COPY migrations ./migrations

EXPOSE 8080

CMD ["api"]
