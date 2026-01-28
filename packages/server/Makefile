.PHONY: help up down logs migrate test

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
	docker-compose exec api cargo sqlx migrate run

test: ## Run tests
	docker-compose exec api cargo test

build: ## Build Rust project
	docker-compose exec api cargo build

check: ## Check Rust project (fast compile check)
	docker-compose exec api cargo check

shell: ## Open shell in API container
	docker-compose exec api /bin/bash

db-shell: ## Open psql shell
	docker-compose exec postgres psql -U postgres -d mndigitalaid

redis-cli: ## Open redis-cli
	docker-compose exec redis redis-cli
