# Package Structure

## Overview

MN Together is a monorepo with a Rust backend and TypeScript frontends. The main server is a single Rust crate with 22 internal business domains.

## Package Map

```
packages/
├── server/          # Rust — Restate workflow server (the backend)
├── web/             # TypeScript — Next.js public web app (App Router)
├── admin-app/       # TypeScript — Next.js admin panel
├── shared/          # TypeScript — Shared GraphQL schema and types
├── extraction/      # Rust — Web scraping library (Firecrawl, Tavily, HTTP)
├── ai-client/       # Rust — LLM client abstraction
├── apify-client/    # Rust — Apify API client
├── twilio-rs/       # Rust — Twilio Verify wrapper
└── openai-client/   # Rust — OpenAI API client
```

## Rust Packages

### `server` (main crate)

The core application. Contains all business logic organized into 22 domains.

- **Entry point**: `src/bin/server.rs`
- **Library**: `src/lib.rs` (exported as `server_core`)
- **Domains**: `src/domains/` (22 business domains)
- **Infrastructure**: `src/kernel/` (ServerDeps, AI clients, SSE)
- **Shared types**: `src/common/` (entity IDs, pagination, restate_serde)
- **Migrations**: `migrations/` (000001–000170+)

### `extraction` (library crate)

Web content ingestion and search.

- `Ingestor` trait with `FirecrawlIngestor`, `HttpIngestor`, `ValidatedIngestor` (SSRF protection)
- `WebSearcher` trait with `TavilyWebSearcher`
- Page caching and content extraction
- Features: `postgres`, `openai`, `firecrawl`

### `ai-client` (library crate)

LLM client abstraction for structured AI calls.

### `apify-client` (library crate)

Client for Apify web scraping actors (social media extraction).

### `twilio-rs` (library crate)

Twilio Verify API wrapper for phone OTP authentication.

### `openai-client` (library crate)

OpenAI API client for embeddings and completions.

## TypeScript Packages

### `web` (Next.js App Router)

Public-facing web application.

- **Port**: 3000
- **Framework**: Next.js with App Router
- **Communicates with**: Restate runtime via HTTP (port 9070)
- **Real-time**: SSE from server (port 8081)
- **GraphQL**: urql client with codegen

### `admin-app` (Next.js)

Admin panel for content moderation and management.

- Approving/rejecting posts and organizations
- Managing sources and websites
- Viewing proposals and sync batches
- JWT-authenticated

### `shared`

Shared TypeScript code between web and admin-app.

- GraphQL schema definitions
- Shared types
- GraphQL codegen configuration

## Domain Inventory (22 domains)

All domains live in `packages/server/src/domains/`:

| # | Domain | Purpose | Has Restate | Has Activities |
|---|--------|---------|-------------|----------------|
| 1 | `agents` | AI agent identity and tracking | No | Yes |
| 2 | `auth` | Phone OTP (Twilio Verify) | Services | Yes |
| 3 | `chatrooms` | Real-time LLM chat | Services, Objects | Yes |
| 4 | `contacts` | Contact info management | No | No |
| 5 | `crawling` | Website crawl orchestration | Workflows | Yes |
| 6 | `curator` | AI curator pipeline | Workflows | Yes |
| 7 | `extraction` | Page extraction/parsing | Services | Yes |
| 8 | `heat_map` | Geographic density viz | Services | Yes |
| 9 | `jobs` | Background job management | Services | No |
| 10 | `locations` | Geocoding and geo data | No | No |
| 11 | `member` | User profiles, registration | Services, Objects, Workflows | Yes |
| 12 | `memo` | LLM response caching | No | No |
| 13 | `notes` | Editorial notes | Services | Yes |
| 14 | `organization` | Org management/approval | Services, Workflows | No |
| 15 | `posts` | Post/listing lifecycle | Services, Objects, Workflows | Yes |
| 16 | `providers` | Service provider profiles | Services, Objects | Yes |
| 17 | `schedules` | Calendar parsing (RFC 5545) | No | No |
| 18 | `social_profile` | Social media linking | Services | Yes |
| 19 | `source` | Content sources | Services, Objects, Workflows | Yes |
| 20 | `sync` | Sync batches/proposals | Services | Yes |
| 21 | `tag` | Tagging/categorization | Services | No |
| 22 | `website` | Website research/regen | Services, Objects, Workflows | Yes |

## Dependency Graph

```
server (main crate)
├── extraction (web scraping)
├── ai-client (LLM abstraction)
├── apify-client (social media scraping)
├── twilio-rs (phone auth)
├── openai-client (embeddings)
└── shared (via GraphQL API, not Rust dep)

web (Next.js)
├── shared (GraphQL types)
└── server (via HTTP → Restate runtime)

admin-app (Next.js)
├── shared (GraphQL types)
└── server (via HTTP → Restate runtime)
```

## Build & Run

```bash
# Rust server
cargo build --bin server
cargo run --bin server    # Starts on port 9080 (Restate) + 8081 (SSE)

# Next.js web app
cd packages/web && yarn dev    # Port 3000

# Admin panel
cd packages/admin-app && yarn dev

# Infrastructure
docker-compose up -d postgres redis nats restate
```
