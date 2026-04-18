# Docker Compose Development Setup

Complete guide for running Root Editorial with Docker Compose.

## Overview

The docker-compose setup includes:
- **PostgreSQL** (pgvector) - Database with vector search support
- **API Server** (Rust) - Axum HTTP server with cargo-watch hot-reload
- **Admin App** (Next.js) - CMS admin panel (port 3000)
- **Web App** (Next.js) - Public-facing site (port 3001)

## Quick Start

1. **Set up environment variables**:
   ```bash
   cd packages/server
   cp .env.example .env
   # Edit .env and add your API keys (see below)
   ```

2. **Start all services**:
   ```bash
   docker-compose up
   # Or run in background: docker-compose up -d
   ```

3. **Run database migrations** (in a new terminal):
   ```bash
   make migrate
   # Or: docker-compose exec api cargo sqlx migrate run
   ```

4. **Access the services**:
   - Admin App (CMS): http://localhost:3000
   - Web App (Public): http://localhost:3001
   - Rust API: http://localhost:9080
   - PostgreSQL: localhost:5432

## Required Environment Variables

Edit `packages/server/.env`:

```env
# Database (already configured for docker-compose)
DATABASE_URL=postgresql://postgres:postgres@postgres:5432/rooteditorial

# AI/ML (REQUIRED)
OPENAI_API_KEY=sk-...                    # Get from https://platform.openai.com/

# SMS authentication (REQUIRED for auth)
TWILIO_ACCOUNT_SID=AC...                 # Get from https://twilio.com/
TWILIO_AUTH_TOKEN=...
TWILIO_VERIFY_SERVICE_SID=VA...

# JWT authentication (REQUIRED)
JWT_SECRET=$(openssl rand -base64 32)    # Generate a secure secret
JWT_ISSUER=rooteditorial

# Optional
ADMIN_IDENTIFIERS=+1234567890            # Optional: Admin phone numbers
```

## Development Workflow

### Starting/Stopping Services

```bash
# Start all services
make up
# Or: docker-compose up -d

# Stop all services
make down
# Or: docker-compose down

# View logs (all services)
make logs
# Or: docker-compose logs -f

# View specific service logs
make web-next-logs           # Next.js app
docker-compose logs -f api   # API server
docker-compose logs -f postgres  # Database
```

### Database Operations

```bash
# Run migrations
make migrate

# Open PostgreSQL shell
make db-shell

# Seed test data
docker-compose exec api cargo run --bin seed_organizations
```

### Next.js Operations

```bash
# View Next.js logs
make web-next-logs

# Restart Next.js (if needed)
make web-next-restart

# Open shell in Next.js container
make web-next-shell

# Install new npm package (from host)
cd packages/web-next
yarn add <package-name>
# Restart: make web-next-restart
```

### Web App Operations

```bash
# View web app logs
make web-app-logs

# Restart web app (if needed)
make web-app-restart

# Open shell in web app container
make web-app-shell

# Install new npm package (from host)
cd packages/web-app
yarn add <package-name>
# Restart: make web-app-restart
```

### API Server Operations

```bash
# Build API
make build

# Run tests
make test

# Check compilation (fast)
make check

# Open shell in API container
make shell

# View available commands
make help
```

## Hot Reload

All services support hot-reload during development:

- **Web App (React SPA)**: Any changes to `packages/web-app/**` automatically refresh
- **Next.js (SSR)**: Any changes to `packages/web-next/**` automatically refresh
- **API Server**: Changes to `packages/server/src/**` trigger rebuild

If hot-reload isn't working:
```bash
# Restart specific service
docker-compose restart web-app
docker-compose restart web-next
docker-compose restart api

# Or restart all
docker-compose restart
```

## Troubleshooting

### Port Conflicts

If you see "port already in use":
```bash
# Check what's using the port
lsof -i :3000  # Next.js
lsof -i :9080  # API
lsof -i :5432  # PostgreSQL

# Stop the conflicting service or use docker-compose down
```

### Database Connection Issues

```bash
# Ensure postgres is healthy
docker-compose ps

# Check postgres logs
docker-compose logs postgres

# Reset database (WARNING: deletes all data)
docker-compose down -v
docker-compose up -d postgres
make migrate
```

### Next.js Not Starting

```bash
# Check logs
make web-next-logs

# Rebuild container
docker-compose up -d --build web-next

# Clear node_modules and reinstall
docker-compose down
rm -rf packages/web-next/node_modules
docker-compose up -d --build web-next
```

### API Build Errors

```bash
# View detailed logs
docker-compose logs api

# Rebuild from scratch
docker-compose down
docker-compose up -d --build api

# Clear cargo cache (if needed)
docker volume rm rooteditorial_api_target
docker-compose up -d --build api
```

### Can't Connect to API from Next.js

Check that `NEXT_PUBLIC_API_URL` is set correctly:
- In docker-compose: `http://localhost:9080/graphql`
- If running Next.js locally: `http://localhost:9080/graphql`

## Architecture

```
┌─────────────────────────┐  ┌─────────────────────────┐
│  Admin App (:3000)      │  │  Web App (:3001)        │
│  Next.js CMS            │  │  Next.js Public         │
│                         │  │                         │
└────────┬────────────────┘  └────────┬────────────────┘
         │                            │
         │ HTTP                       │ HTTP
         ↓                            ↓
         └────────────┬───────────────┘
                      ↓
         ┌────────────────────────────────┐
         │  Rust Server (localhost:9080)  │
         │  Axum HTTP + SSE               │
         └────────────┬──────────────────┘
                      │
                      ↓
               ┌─────────────┐
               │ PostgreSQL  │
               │ (pgvector)  │
               │ :5432       │
               └─────────────┘
```

## Performance Tips

- **First build is slow**: Docker caches dependencies, subsequent builds are much faster
- **Volume mounts**: Code changes don't require rebuilds, only service restarts
- **Cargo cache**: The `api-target` volume persists compiled artifacts across rebuilds

## Next Steps

After getting everything running:

1. **Explore the GraphQL API**: http://localhost:9080/graphql
2. **Browse the Next.js site**: http://localhost:3000
3. **Seed test data**: `docker-compose exec api cargo run --bin seed_organizations`
4. **Try semantic search**: http://localhost:3000/search?q=food
5. **Check the admin UI**: See [packages/admin-app/README.md](../../packages/admin-app/README.md)

## Production Deployment

For production deployment to AWS:
- See [infra/README.md](infra/README.md)
- GitHub Actions automatically deploys on push to main/dev
