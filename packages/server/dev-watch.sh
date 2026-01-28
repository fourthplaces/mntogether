#!/bin/bash
# Development watch script - runs cargo watch with hot-reload
# Restarts when source files change

set -e

echo "[dev-watch] Starting development server with hot-reload..."
echo "[dev-watch] Watching: /app/packages"
echo "[dev-watch] RUST_LOG=${RUST_LOG:-info}"
echo ""

# Wait for database to be ready
echo "[dev-watch] Waiting for database..."
until pg_isready -h postgres -p 5432 -U postgres; do
    echo "[dev-watch] Database not ready, waiting..."
    sleep 2
done
echo "[dev-watch] Database is ready!"
echo ""

# Run migrations on startup
echo "[dev-watch] Running database migrations..."
cd /app/packages/server
sqlx migrate run || {
    echo "[dev-watch] WARNING: Migrations failed, but continuing..."
}
echo ""

# Run cargo watch
# Watch all packages in the workspace and reload on changes
echo "[dev-watch] Starting cargo watch..."
exec cargo watch \
    -w /app/packages \
    -s 'cargo run --bin api'
