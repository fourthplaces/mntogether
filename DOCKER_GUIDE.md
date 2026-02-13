# Docker Setup Guide

This guide covers running the Minnesota Digital Aid platform using Docker Compose.

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
# AI Services (REQUIRED)
OPENAI_API_KEY=sk-...
VOYAGE_API_KEY=pa-...

# Web Scraping (REQUIRED)
FIRECRAWL_API_KEY=fc-...

# SMS Authentication (REQUIRED)
TWILIO_ACCOUNT_SID=AC...
TWILIO_AUTH_TOKEN=...
TWILIO_VERIFY_SERVICE_SID=VA...

# JWT Secret (REQUIRED - generate with: openssl rand -base64 32)
JWT_SECRET=your_jwt_secret_here_at_least_32_bytes

# Admin Access (REQUIRED for admin features)
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
- **Redis** (port 6379) - Job queue and pub/sub
- **API Server** (port 8080) - Rust GraphQL server
- **Web App** (port 3001) - React admin dashboard

### 3. Run Migrations

```bash
make migrate
```

### 4. Access the Application

- **API Server**: http://localhost:8080
- **GraphQL Playground**: http://localhost:8080/graphql
- **Web App**: http://localhost:3001
- **Admin Dashboard**: http://localhost:3001/admin
- **Health Check**: http://localhost:8080/health

## Service Profiles

### Default Services (Core)

Includes Postgres, Redis, API Server, and Web App:

```bash
docker compose up -d
```

### Full Stack (including Next.js)

```bash
# Using Make
make up-full

# Or using Docker Compose
docker compose --profile full up -d
```

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
make logs-web    # Web app only
make logs-db     # PostgreSQL only
make logs-redis  # Redis only
```

### Database Operations

```bash
make migrate     # Run migrations
make seed        # Seed organizations
make db-shell    # Open PostgreSQL shell
make db-reset    # Reset database (⚠️  data loss)
```

### Development Tools

```bash
make shell       # Open shell in API container
make redis-cli   # Open Redis CLI
make test        # Run Rust tests
make check       # Fast compile check
make fmt         # Format Rust code
make clippy      # Run linter
```

### Cleanup

```bash
make clean       # Remove all containers and volumes (⚠️  data loss)
make prune       # Clean Docker build cache
```

## Development Workflow

### 1. Making Changes

The development setup includes hot-reloading:

- **Rust API**: Uses `cargo-watch` to rebuild on file changes
- **Web App**: Vite dev server with hot module replacement

### 2. Running Tests

```bash
# Run all tests
make test

# Run specific test
docker compose exec api cargo test --test organization_needs_tests
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

### 4. Seeding Data

Seed the database with real organizations:

```bash
make seed
```

This imports 50+ immigrant resource organizations with AI-powered tag extraction.

## Troubleshooting

### Services Won't Start

Check if ports are already in use:

```bash
# Check port usage
lsof -i :5432  # PostgreSQL
lsof -i :6379  # Redis
lsof -i :8080  # API
lsof -i :3001  # Web App
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

Remove all containers and volumes (⚠️  data loss):

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
6. **Use managed Redis** (not the Docker image)
7. **Set up SSL/TLS** termination (nginx, Traefik, or cloud load balancer)
8. **Configure logging** (structured logs to stdout)
9. **Set up monitoring** (health checks, metrics)
10. **Regular backups** of PostgreSQL data

## Architecture

```
┌─────────────────┐
│   Web App       │
│  (React/Vite)   │
│   Port 3001     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐      ┌─────────────┐      ┌─────────────┐
│   API Server    │─────▶│  PostgreSQL │      │    Redis    │
│  (Rust/GraphQL) │◀─────│  (pgvector) │      │  (pub/sub)  │
│   Port 8080     │      │  Port 5432  │      │  Port 6379  │
└─────────────────┘      └─────────────┘      └─────────────┘
         │
         │ (External APIs)
         ├─▶ OpenAI (GPT-4o, embeddings)
         ├─▶ Voyage AI (embeddings)
         ├─▶ Firecrawl (web scraping)
         ├─▶ Twilio (SMS auth)
         └─▶ Expo (push notifications)
```

## Data Persistence

Docker volumes are used for data persistence:

- `mndigitalaid_postgres_data` - PostgreSQL database
- `mndigitalaid_redis_data` - Redis data
- `mndigitalaid_rust_target` - Rust build cache

These volumes persist even after `docker compose down`. To remove them:

```bash
make clean  # Interactive prompt
# or
docker compose down -v  # Force remove
```

## Next Steps

- Read [QUICK_START.md](docs/setup/QUICK_START.md) for detailed setup
- See [API_INTEGRATION_GUIDE.md](docs/guides/API_INTEGRATION_GUIDE.md) for API usage
- Check [DEPLOYMENT.md](docs/setup/DEPLOYMENT.md) for production deployment
- Review [SECURITY.md](docs/security/SECURITY.md) for security best practices

## Support

- Issues: [GitHub Issues](https://github.com/fourthplaces/mndigitalaid/issues)
- Documentation: [docs/](docs/)
- Interactive CLI: Run `./dev.sh` for guided setup
