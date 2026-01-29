# Docker Compose Development Setup

Complete guide for running the MN Digital Aid platform with Docker Compose.

## Overview

The docker-compose setup includes:
- **PostgreSQL** (pgvector) - Database with vector search support
- **Redis** - Pub/sub for real-time features
- **API Server** (Rust) - GraphQL backend with hot-reload
- **Next.js App** (Node.js) - Public-facing SSR website with hot-reload

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
   - Next.js App: http://localhost:3000
   - GraphQL API: http://localhost:8080/graphql
   - PostgreSQL: localhost:5432
   - Redis: localhost:6379

## Required Environment Variables

Edit `packages/server/.env`:

```env
# Database (already configured for docker-compose)
DATABASE_URL=postgresql://postgres:postgres@postgres:5432/mndigitalaid
REDIS_URL=redis://redis:6379

# AI/ML (REQUIRED)
OPENAI_API_KEY=sk-...                    # Get from https://platform.openai.com/

# Web scraping (REQUIRED)
FIRECRAWL_API_KEY=fc-...                 # Get from https://firecrawl.dev/

# SMS authentication (REQUIRED for auth)
TWILIO_ACCOUNT_SID=AC...                 # Get from https://twilio.com/
TWILIO_AUTH_TOKEN=...
TWILIO_VERIFY_SERVICE_SID=VA...

# JWT authentication (REQUIRED)
JWT_SECRET=$(openssl rand -base64 32)    # Generate a secure secret
JWT_ISSUER=mndigitalaid

# Optional
TAVILY_API_KEY=tvly-...                  # Optional: Enhanced search
EXPO_ACCESS_TOKEN=...                    # Optional: Push notifications
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

Both the API server and Next.js app support hot-reload:

- **Next.js**: Any changes to `packages/web-next/**` files automatically refresh
- **API Server**: Changes to `packages/server/src/**` trigger rebuild

If hot-reload isn't working:
```bash
# Restart specific service
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
lsof -i :8080  # API
lsof -i :5432  # PostgreSQL
lsof -i :6379  # Redis

# Stop the conflicting service or use docker-compose down
```

### Database Connection Issues

```bash
# Ensure postgres is healthy
docker-compose ps

# Check postgres logs
docker-compose logs postgres

# Reset database (⚠️ deletes all data)
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
docker volume rm mndigitalaid_api_target
docker-compose up -d --build api
```

### Can't Connect to API from Next.js

Check that `NEXT_PUBLIC_API_URL` is set correctly:
- In docker-compose: `http://localhost:8080/graphql`
- If running Next.js locally: `http://localhost:8080/graphql`

## Architecture

```
┌─────────────────────────────────────────────────┐
│  Browser (localhost:3000)                       │
│  Next.js SSR App                                │
└────────────────┬────────────────────────────────┘
                 │ HTTP
                 ↓
┌─────────────────────────────────────────────────┐
│  API Server (localhost:8080)                    │
│  Rust + GraphQL                                 │
└────┬──────────────────────┬─────────────────────┘
     │                      │
     ↓                      ↓
┌─────────────┐      ┌─────────────┐
│ PostgreSQL  │      │   Redis     │
│ (pgvector)  │      │  (pub/sub)  │
│ :5432       │      │   :6379     │
└─────────────┘      └─────────────┘
```

## Performance Tips

- **First build is slow**: Docker caches dependencies, subsequent builds are much faster
- **Volume mounts**: Code changes don't require rebuilds, only service restarts
- **Cargo cache**: The `api-target` volume persists compiled artifacts across rebuilds

## Next Steps

After getting everything running:

1. **Explore the GraphQL API**: http://localhost:8080/graphql
2. **Browse the Next.js site**: http://localhost:3000
3. **Seed test data**: `docker-compose exec api cargo run --bin seed_organizations`
4. **Try semantic search**: http://localhost:3000/search?q=food
5. **Check the admin UI**: See [packages/admin-spa/README.md](packages/admin-spa/README.md)

## Production Deployment

For production deployment to AWS:
- See [infra/README.md](infra/README.md)
- GitHub Actions automatically deploys on push to main/dev
