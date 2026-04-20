# ============================================================================
# Root Editorial - Development Makefile
# ============================================================================
# Quick commands for managing the development environment
#
# Usage:
#   make help     - Show this help message
#   make up       - Start all services
#   make down     - Stop all services
#   make logs     - View logs from all services
#   make restart  - Restart all services
# ============================================================================

.PHONY: help up down logs restart restart-server restart-admin restart-minio clean build migrate seed seed-media-upload audit-seed audit-seed-rebaseline shell db-shell test check

# Default target - show help
help:
	@echo "Root Editorial - Development Commands"
	@echo ""
	@echo "Getting Started:"
	@echo "  make up          - Start all services (Postgres, API, Web App, MinIO)"
	@echo "  make up-full     - Start all services including Next.js"
	@echo "  make down        - Stop all services"
	@echo "  make restart        - Restart all services (down + up, picks up config changes)"
	@echo "  make restart-server - Restart server only"
	@echo "  make restart-admin  - Restart admin app only"
	@echo "  make restart-minio  - Restart MinIO (S3 storage)"
	@echo ""
	@echo "Logs & Monitoring:"
	@echo "  make logs        - View logs from all services"
	@echo "  make logs-server - View server logs only"
	@echo "  make logs-admin  - View admin app logs only"
	@echo "  make logs-minio  - View MinIO (S3 storage) logs only"
	@echo "  make logs-db     - View PostgreSQL logs only"
	@echo ""
	@echo "Database:"
	@echo "  make migrate     - Run database migrations"
	@echo "  make seed        - Load seed data from JSON into database"
	@echo "  make db-shell    - Open PostgreSQL shell"
	@echo "  make db-reset    - Drop, migrate, and seed database"
	@echo ""
	@echo "Development:"
	@echo "  make shell       - Open shell in API container"
	@echo "  make build       - Rebuild all containers"
	@echo "  make test        - Run Rust tests"
	@echo "  make check       - Fast compile check without building"
	@echo ""
	@echo "Cleanup:"
	@echo "  make clean       - Remove all containers and volumes (WARNING: data loss)"
	@echo "  make prune       - Clean up Docker build cache"
	@echo ""
	@echo "Tip: Run './dev.sh' for interactive CLI menu"

# ========================================
# Service Management
# ========================================

# Start all core services (Postgres, API, Web App, MinIO)
up:
	docker compose up -d

# Start all services including optional ones (Next.js)
up-full:
	docker compose --profile full up -d

# Stop all services
down:
	docker compose down

# Restart all services (down + up to pick up compose config changes)
restart:
	docker compose down
	docker compose up -d

# Restart server only
restart-server:
	docker compose rm -sf server
	docker compose up -d server

# Restart admin app only
restart-admin:
	docker compose rm -sf admin-app
	docker compose up -d admin-app

# Restart MinIO (S3 storage)
restart-minio:
	docker compose rm -sf minio minio-init
	docker compose up -d minio minio-init

# Rebuild and start all services
build:
	docker compose build --no-cache
	docker compose up -d

# ========================================
# Logs & Monitoring
# ========================================

# View logs from all services
logs:
	docker compose logs -f

# View API server logs
logs-server:
	docker compose logs -f server

# View admin app (Next.js) logs
logs-admin:
	docker compose logs -f admin-app

# Alias for logs-admin
logs-next:
	docker compose logs -f admin-app

# View MinIO (S3 storage) logs
logs-minio:
	docker compose logs -f minio

# View PostgreSQL logs
logs-db:
	docker compose logs -f postgres

# ========================================
# Database Operations
# ========================================

# Run database migrations
migrate:
	docker compose exec server sqlx migrate run

# Upload the local seed image files into the MinIO `media` bucket at the
# `seed/` prefix. Uses a throwaway `minio/mc` client container on the
# compose network so no mc install is needed on the host. Idempotent.
# (minio/mc's default entrypoint is `mc`; override with `sh -c` via
# --entrypoint so we can chain commands.)
seed-media-upload:
	@docker run --rm \
		--network rooteditorial_network \
		-v "$(PWD)/data/seed-media:/seed-media:ro" \
		--entrypoint sh \
		minio/mc:latest \
		-c "mc alias set local http://minio:9000 minioadmin minioadmin >/dev/null && mc cp --recursive /seed-media/ local/media/seed/"

# Seed database from JSON files (data/posts.json, data/organizations.json,
# data/tags.json, data/widgets.json). Uploads seed media first so the
# emitted media rows reference files that actually exist in MinIO.
seed: seed-media-upload
	@node data/seed.mjs | docker compose exec -T postgres psql -U postgres -d rooteditorial

# Audit seed data against the Root Signal data contract.
# Prints gap report + writes machine-readable data/audit-seed.out.json.
# Exits non-zero if any gap category regressed vs the committed baseline
# (data/audit-seed.baseline.json) — use this as a pre-commit sanity check
# when touching data/posts.json.
audit-seed:
	@node data/audit-seed.mjs --check

# Lock in the current seed state as the new baseline. Run after finishing
# an enrichment pass (see docs/guides/SEED_DATA_ENRICHMENT_PLAN.md).
audit-seed-rebaseline:
	@node data/audit-seed.mjs --rebaseline

# Open PostgreSQL shell
db-shell:
	docker compose exec postgres psql -U postgres -d rooteditorial

# Reset database: drop, recreate, migrate, seed
db-reset:
	@echo "Dropping and recreating database..."
	@docker compose exec -T postgres psql -U postgres -c "DROP DATABASE IF EXISTS rooteditorial;" -c "CREATE DATABASE rooteditorial;"
	@echo "Running migrations..."
	@docker compose exec server sqlx migrate run --source /app/packages/server/migrations
	@echo "Seeding..."
	@$(MAKE) seed
	@echo "Done."

# ========================================
# Development Tools
# ========================================

# Open shell in API container
shell:
	docker compose exec server /bin/bash

# Run Rust tests
test:
	docker compose exec server cargo test

# Fast compile check (no binary output)
check:
	docker compose exec server cargo check

# Format Rust code
fmt:
	docker compose exec server cargo fmt

# Run Rust linter
clippy:
	docker compose exec server cargo clippy

# ========================================
# Cleanup
# ========================================

# Remove all containers and volumes (WARNING: data loss)
clean:
	@echo "WARNING: This will delete all containers, volumes, and data!"
	@read -p "Are you sure? (y/N) " -n 1 -r; \
	echo; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		docker compose down -v; \
		echo "Cleanup complete."; \
	else \
		echo "Cancelled"; \
	fi

# Clean up Docker build cache and dangling resources
prune:
	docker builder prune -a -f
	docker volume prune -f
	docker image prune -f

# ========================================
# Status & Health
# ========================================

# Show status of all services
status:
	docker compose ps

# Check health of all services
health:
	@echo "Checking service health..."
	@echo ""
	@echo "PostgreSQL:"
	@docker compose exec postgres pg_isready -U postgres || echo "  FAIL: Not ready"
	@echo ""
	@echo "API Server:"
	@curl -sf http://localhost:8080/health || echo "  FAIL: Not ready"
	@echo ""
	@echo "Web App:"
	@curl -sf http://localhost:3001 > /dev/null && echo "  OK" || echo "  FAIL: Not ready"

# ========================================
# Useful Development Commands
# ========================================

# Watch Rust changes and rebuild
watch:
	docker compose exec server cargo watch -x 'run --bin server'

# Create a new database migration
migration:
	@read -p "Migration name: " name; \
	docker compose exec server sqlx migrate add $$name
