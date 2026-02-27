.PHONY: help up down logs migrate test seed reset-db restart-admin restart-web

help: ## Show this help message
	@echo "Available commands:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

up: ## Start all services with docker-compose
	docker-compose up -d

down: ## Stop all services
	docker-compose down

logs: ## Follow logs from all services
	docker-compose logs -f

migrate: ## Run database migrations
	docker-compose exec server cargo sqlx migrate run

test: ## Run tests
	docker-compose exec server cargo test

build: ## Build Rust project
	docker-compose exec server cargo build

check: ## Check Rust project (fast compile check)
	docker-compose exec server cargo check

shell: ## Open shell in API container
	docker-compose exec server /bin/bash

db-shell: ## Open psql shell
	docker-compose exec postgres psql -U postgres -d rooteditorial

web-logs: ## Follow Next.js app logs
	docker-compose logs -f web

web-shell: ## Open shell in Next.js container
	docker-compose exec web /bin/sh

web-restart: ## Restart Next.js service
	docker-compose restart web

web-app-logs: ## Follow web app logs
	docker-compose logs -f web-app

web-app-shell: ## Open shell in web app container
	docker-compose exec web-app /bin/sh

web-app-restart: ## Restart web app service
	docker-compose restart web-app

restart-admin: ## Restart admin-app with fresh Turbopack cache (fixes stuck "Compiling...")
	docker compose down admin-app && docker compose up -d admin-app

restart-web: ## Restart web-app with fresh Turbopack cache
	docker compose down web-app && docker compose up -d web-app

seed: ## Load seed data from JSON into the database
	@node data/seed.mjs | docker compose exec -T postgres psql -U postgres -d rooteditorial

reset-db: ## Drop database, re-migrate, and seed
	@echo "Dropping and recreating database..."
	@docker compose exec -T postgres psql -U postgres -c "DROP DATABASE IF EXISTS rooteditorial;" -c "CREATE DATABASE rooteditorial;"
	@echo "Running migrations..."
	@docker compose exec server sqlx migrate run --source /app/packages/server/migrations
	@echo "Seeding..."
	@$(MAKE) seed
	@echo "Done."
