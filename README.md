# Root Editorial

An open CMS for community journalism — helping non-technical editors curate and publish community-focused content.

## Overview

Root Editorial is the editorial layer of a three-part architecture:

- **Root Signal** (separate repo) — AI-powered content discovery and crawling
- **Root Editorial** (this repo) — CMS for editorial curation and publishing
- **MN Together** — The first instance/theme, focused on Minnesota community journalism

### What This Repo Does

- **CMS GUI** for non-tech editors to curate community content
- **Post lifecycle management** — create, review, approve, publish, expire
- **Organization management** — track community organizations and their content sources
- **AI-assisted editorial tooling** — PII detection, summary generation, editorial notes
- **GraphQL API** — serving admin and public web apps
- **Broadsheet layout engine** (coming) — 3-column newspaper-style digital editions

## Quick Start

### Prerequisites

- Docker and Docker Compose ([Install Docker Desktop](https://www.docker.com/products/docker-desktop/))
- Rust toolchain (for local dev without Docker)
- Node.js 22+ and Yarn 4+ (for frontend apps)

### Setup

```bash
# 1. Clone and enter repo
gh repo clone fourthplaces/mntogether
cd mntogether

# 2. Generate Cargo.lock (gitignored)
cargo generate-lockfile

# 3. Set up environment
cp .env.example .env
# Edit .env:
#   JWT_SECRET=<generate with: openssl rand -base64 32>
#   TEST_IDENTIFIER_ENABLED=true

# 4. Start infrastructure
docker compose up -d

# 5. Run migrations
docker compose exec server sqlx migrate run --source /app/packages/server/migrations

# 6. Restore test data
docker compose exec -T postgres psql -U postgres -d rooteditorial < data/local_test_db.sql
```

### Start Development

```bash
# Backend (Rust server on port 9080)
cargo run --bin server

# Admin app (CMS on port 3000)
cd packages/admin-app && yarn dev

# Public web app (port 3001)
cd packages/web-app && yarn dev
```

### Test Login

With `TEST_IDENTIFIER_ENABLED=true`:
- Phone: `+1234567890`
- Code: any value (Twilio verification is skipped)

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
         ┌─────────────────┐
         │ Restate Runtime  │
         │  Port 9070/8180  │
         └────────┬────────┘
                  ▼
┌─────────────────┐      ┌─────────────┐      ┌─────────────┐
│   Rust Server   │─────▶│  PostgreSQL  │      │    Redis    │
│  (Restate svc)  │      │  (pgvector)  │      │  (caching)  │
│   Port 9080     │      │  Port 5432   │      │  Port 6379  │
└─────────────────┘      └─────────────┘      └─────────────┘
         │
         │ (External APIs)
         ├─▶ OpenAI / OpenRouter (LLM)
         └─▶ Twilio (SMS/email auth)
```

## Workspace Packages

```
packages/
├── server/          # Rust — Restate workflow server (backend)
├── admin-app/       # TypeScript — Next.js CMS admin panel
├── web-app/         # TypeScript — Next.js public web app
├── shared/          # TypeScript — Shared GraphQL schema and types
├── ai-client/       # Rust — LLM client abstraction
└── twilio-rs/       # Rust — Twilio Verify wrapper
```

## Technology Stack

| Component | Technology |
|-----------|-----------|
| Backend | Rust + Restate SDK 0.4.0 |
| Database | PostgreSQL + pgvector |
| Cache | Redis |
| LLM | OpenAI / OpenRouter |
| Auth | Twilio Verify (phone/email OTP) + JWT |
| Frontend | Next.js (App Router) |
| GraphQL | Shared schema (packages/shared) |

## Documentation

See the [docs/](docs/README.md) directory for complete documentation:

- **[Root Editorial Pivot](docs/architecture/ROOT_EDITORIAL_PIVOT.md)** — The pivot bible
- **[Local Dev Setup](docs/setup/LOCAL_DEV_SETUP.md)** — Detailed setup with test data
- **[Docker Guide](docs/setup/DOCKER_GUIDE.md)** — Docker Compose reference
- **[Architecture](docs/architecture/DOMAIN_ARCHITECTURE.md)** — Domain architecture
- **[API Guide](docs/guides/API_INTEGRATION_GUIDE.md)** — GraphQL API integration

## License

MIT
