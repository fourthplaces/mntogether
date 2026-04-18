# Docker Setup Guide

This guide covers running the Root Editorial platform using Docker Compose.

## Prerequisites

- Docker 20.10+ and Docker Compose ([Install Docker Desktop](https://www.docker.com/products/docker-desktop/))
- API keys for required services (see Environment Setup below)

## Quick Start

### 1. Environment Setup

Create your environment file:

```bash
cp .env.example .env
```

Edit `.env` and add your API keys. **Minimum required keys:**

```env
# SMS Authentication (REQUIRED)
TWILIO_ACCOUNT_SID=AC...
TWILIO_AUTH_TOKEN=...
TWILIO_VERIFY_SERVICE_SID=VA...

# JWT Secret (REQUIRED - generate with: openssl rand -base64 32)
JWT_SECRET=your_jwt_secret_here_at_least_32_bytes

# Admin Access (REQUIRED for CMS admin features)
ADMIN_IDENTIFIERS=admin@example.com,+15551234567
```

### 2. Start All Services

```bash
# Using Make (recommended)
make up

# Or using Docker Compose directly
docker compose up -d
```

This starts:
- **PostgreSQL** (port 5432) - Database with pgvector extension
- **Rust Server** (port 9080) - Axum HTTP/JSON API + SSE streams
- **MinIO** (ports 9000 API / 9001 console) - S3-compatible media storage

### 3. Run Migrations

```bash
make migrate
```

### 4. Access the Application

- **CMS Admin App**: http://localhost:3000 (run separately with `yarn dev`)
- **Public Web App**: http://localhost:3001 (run separately with `yarn dev`)
- **Rust API**: http://localhost:9080
- **Health Check**: http://localhost:9080/health
- **MinIO Console**: http://localhost:9001

## Common Commands

### Service Management

```bash
make up          # Start all services
make down        # Stop all services
make restart     # Restart all services
make build       # Rebuild containers
make status      # Show service status
make health      # Check service health
```

### Viewing Logs

```bash
make logs        # All services
make logs-api    # API server only
make logs-db     # PostgreSQL only
```

### Database Operations

```bash
make migrate     # Run migrations
make db-shell    # Open PostgreSQL shell
make db-reset    # Reset database (data loss)
```

### Development Tools

```bash
make shell       # Open shell in API container
make test        # Run Rust tests
make check       # Fast compile check
make fmt         # Format Rust code
make clippy      # Run linter
```

### Cleanup

```bash
make clean       # Remove all containers and volumes (data loss)
make prune       # Clean Docker build cache
```

## Development Workflow

### 1. Making Changes

The development setup includes hot-reloading:

- **Rust API**: Uses `cargo-watch` to rebuild on file changes
- **Next.js Apps**: Vite/Next dev server with hot module replacement (run separately)

### 2. Running Tests

```bash
# Run all tests
make test

# Run specific test
docker compose exec api cargo test --test some_test_name
```

### 3. Database Migrations

Create a new migration:

```bash
make migration
# Enter migration name when prompted
```

Run migrations:

```bash
make migrate
```

## Troubleshooting

### Services Won't Start

Check if ports are already in use:

```bash
# Check port usage
lsof -i :5432  # PostgreSQL
lsof -i :9080  # Rust Server
lsof -i :9000  # MinIO API
lsof -i :9001  # MinIO console
```

### Database Connection Issues

1. Verify PostgreSQL is healthy:
   ```bash
   docker compose exec postgres pg_isready -U postgres
   ```

2. Check logs:
   ```bash
   make logs-db
   ```

### API Server Not Starting

1. Check environment variables are set:
   ```bash
   docker compose exec api env | grep API_KEY
   ```

2. View API logs:
   ```bash
   make logs-api
   ```

3. Verify migrations ran:
   ```bash
   make migrate
   ```

### Hot Reload Not Working

1. Restart the API container:
   ```bash
   docker compose restart api
   ```

2. Check volume mounts:
   ```bash
   docker compose config
   ```

### Out of Disk Space

Clean up Docker cache:

```bash
make prune
```

Remove all containers and volumes (data loss):

```bash
make clean
```

## Production Deployment

For production deployment, use the production Dockerfile:

```bash
docker compose -f docker-compose.prod.yml up -d
```

**Important production considerations:**

1. **Use production Dockerfile** (`packages/server/Dockerfile`)
2. **Set secure JWT_SECRET** (32+ bytes, random)
3. **Configure ALLOWED_ORIGINS** explicitly
4. **Disable TEST_IDENTIFIER_ENABLED**
5. **Use managed PostgreSQL** (not the Docker image)
6. **Set up SSL/TLS** termination (nginx, Traefik, or cloud load balancer)
8. **Configure logging** (structured logs to stdout)
9. **Set up monitoring** (health checks, metrics)
10. **Regular backups** of PostgreSQL data

## Architecture

```
┌──────────────────┐     ┌──────────────────┐
│   Admin App      │     │   Web App        │
│  (Next.js CMS)   │     │  (Next.js public)│
│   Port 3000      │     │   Port 3001      │
└────────┬─────────┘     └────────┬─────────┘
         │                        │
         └────────┬───────────────┘
                  ▼
         ┌───────────────────┐
         │  GraphQL resolvers │
         │  (in Next.js API)  │
         └─────────┬──────────┘
                   ▼
         ┌────────────────────┐      ┌─────────────┐
         │  Rust Axum Server   │─────▶│  PostgreSQL │
         │  HTTP/JSON + SSE    │      │  (pgvector) │
         │  Port 9080          │      │  Port 5432  │
         └──────────┬──────────┘      └─────────────┘
                    │
                    │ (External APIs)
                    ├─▶ OpenAI (LLM)
                    ├─▶ Twilio (SMS auth)
                    └─▶ MinIO / S3 (media)
```

## Data Persistence

Docker volumes are used for data persistence:

- `rooteditorial_postgres_data` - PostgreSQL database
- `rooteditorial_rust_target` - Rust build cache

These volumes persist even after `docker compose down`. To remove them:

```bash
make clean  # Interactive prompt
# or
docker compose down -v  # Force remove
```
