# ============================================================================
# Minnesota Digital Aid - Development Makefile
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

.PHONY: help up down logs restart clean build migrate seed shell db-shell redis-cli test check

# Default target - show help
help:
	@echo "Minnesota Digital Aid - Development Commands"
	@echo ""
	@echo "🚀 Getting Started:"
	@echo "  make up          - Start all services (Postgres, Redis, API, Web App)"
	@echo "  make up-full     - Start all services including Next.js"
	@echo "  make down        - Stop all services"
	@echo "  make restart     - Restart all services"
	@echo ""
	@echo "📋 Logs & Monitoring:"
	@echo "  make logs        - View logs from all services"
	@echo "  make logs-api    - View API server logs only"
	@echo "  make logs-web    - View web app logs only"
	@echo "  make logs-db     - View PostgreSQL logs only"
	@echo ""
	@echo "🗄️  Database:"
	@echo "  make migrate     - Run database migrations"
	@echo "  make seed        - Seed database with organizations"
	@echo "  make db-shell    - Open PostgreSQL shell"
	@echo "  make db-reset    - Reset database (drops and recreates)"
	@echo ""
	@echo "🔧 Development:"
	@echo "  make shell       - Open shell in API container"
	@echo "  make redis-cli   - Open Redis CLI"
	@echo "  make build       - Rebuild all containers"
	@echo "  make test        - Run Rust tests"
	@echo "  make check       - Fast compile check without building"
	@echo ""
	@echo "🧹 Cleanup:"
	@echo "  make clean       - Remove all containers and volumes (⚠️  data loss)"
	@echo "  make prune       - Clean up Docker build cache"
	@echo ""
	@echo "💡 Tip: Run './dev.sh' for interactive CLI menu"

# ========================================
# Service Management
# ========================================

# Start all core services (Postgres, Redis, API, Web App)
up:
	docker compose up -d

# Start all services including optional ones (Next.js)
up-full:
	docker compose --profile full up -d

# Stop all services
down:
	docker compose down

# Restart all services
restart:
	docker compose restart

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
logs-api:
	docker compose logs -f api

# View web app logs
logs-web:
	docker compose logs -f web-app

# View web-next logs
logs-next:
	docker compose logs -f web-next

# View PostgreSQL logs
logs-db:
	docker compose logs -f postgres

# View Redis logs
logs-redis:
	docker compose logs -f redis

# ========================================
# Database Operations
# ========================================

# Run database migrations
migrate:
	docker compose exec api sqlx migrate run

# Seed database with organizations
seed:
	docker compose exec api cargo run --bin seed_organizations

# Generate embeddings for existing data
embeddings:
	docker compose exec api cargo run --bin generate_embeddings

# Open PostgreSQL shell
db-shell:
	docker compose exec postgres psql -U postgres -d mndigitalaid

# Reset database (⚠️  drops all data)
db-reset:
	@echo "⚠️  WARNING: This will delete all data!"
	@read -p "Are you sure? (y/N) " -n 1 -r; \
	echo; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		docker compose exec postgres psql -U postgres -c "DROP DATABASE IF EXISTS mndigitalaid;"; \
		docker compose exec postgres psql -U postgres -c "CREATE DATABASE mndigitalaid;"; \
		$(MAKE) migrate; \
		echo "✅ Database reset complete"; \
	else \
		echo "Cancelled"; \
	fi

# ========================================
# Development Tools
# ========================================

# Open shell in API container
shell:
	docker compose exec api /bin/bash

# Open Redis CLI
redis-cli:
	docker compose exec redis redis-cli

# Run Rust tests
test:
	docker compose exec api cargo test

# Fast compile check (no binary output)
check:
	docker compose exec api cargo check

# Format Rust code
fmt:
	docker compose exec api cargo fmt

# Run Rust linter
clippy:
	docker compose exec api cargo clippy

# ========================================
# Cleanup
# ========================================

# Remove all containers and volumes (⚠️  data loss)
clean:
	@echo "⚠️  WARNING: This will delete all containers, volumes, and data!"
	@read -p "Are you sure? (y/N) " -n 1 -r; \
	echo; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		docker compose down -v; \
		docker volume rm mndigitalaid_postgres_data mndigitalaid_redis_data mndigitalaid_rust_target 2>/dev/null || true; \
		echo "✅ Cleanup complete"; \
	else \
		echo "Cancelled"; \
	fi

# Clean up Docker build cache
prune:
	docker system prune -f
	docker volume prune -f

# ========================================
# Status & Health
# ========================================

# Show status of all services
status:
	docker compose ps

# Check health of all services
health:
	@echo "🔍 Checking service health..."
	@echo ""
	@echo "📊 PostgreSQL:"
	@docker compose exec postgres pg_isready -U postgres || echo "❌ Not ready"
	@echo ""
	@echo "📊 Redis:"
	@docker compose exec redis redis-cli ping || echo "❌ Not ready"
	@echo ""
	@echo "📊 API Server:"
	@curl -sf http://localhost:8080/health || echo "❌ Not ready"
	@echo ""
	@echo "📊 Web App:"
	@curl -sf http://localhost:3001 > /dev/null && echo "✅ Running" || echo "❌ Not ready"

# ========================================
# Useful Development Commands
# ========================================

# Watch Rust changes and rebuild
watch:
	docker compose exec api cargo watch -x 'run --bin server'

# Generate GraphQL schema
schema:
	docker compose exec api cargo run --bin generate_schema

# Interactive development CLI
dev-cli:
	./dev.sh

# Create a new database migration
migration:
	@read -p "Migration name: " name; \
	docker compose exec api sqlx migrate add $$name
