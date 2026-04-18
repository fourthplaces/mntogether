# Package Structure

## Overview

Root Editorial is a monorepo with a Rust backend and TypeScript frontends. The main server is a single Rust crate with multiple internal business domains.

## Package Map

```
packages/
├── server/          # Rust — Axum HTTP/JSON API server (the backend)
├── admin-app/       # TypeScript — Next.js CMS admin panel
├── web-app/         # TypeScript — Next.js public web app
├── shared/          # TypeScript — Shared GraphQL schema + resolvers
├── ai-client/       # Rust — LLM client abstraction (OpenAI, OpenRouter)
└── twilio-rs/       # Rust — Twilio Verify wrapper
```

## Rust Packages

### `server` (main crate)

The core application. Contains all business logic organized into domains.

- **Entry point**: `src/bin/server.rs`
- **Library**: `src/lib.rs` (exported as `server_core`)
- **Domains**: `src/domains/` (business domains)
- **Infrastructure**: `src/kernel/` (ServerDeps, AI clients)
- **HTTP API**: `src/api/routes/` (one file per service: posts.rs, editions.rs, widgets.rs, etc.)
- **Shared types**: `src/common/` (entity IDs, pagination, auth extractors)
- **Migrations**: `migrations/` (231+ files)

### `ai-client` (library crate)

LLM client abstraction for structured AI calls. Supports OpenAI and OpenRouter.

### `twilio-rs` (library crate)

Twilio Verify API wrapper for phone/email OTP authentication.

## TypeScript Packages

### `admin-app` (Next.js)

CMS admin panel for content moderation and management.

- **Port**: 3000
- **Framework**: Next.js with App Router
- **Features**: Approving/rejecting posts, managing orgs, editorial workflows
- **Auth**: JWT-authenticated (admin only)

### `web-app` (Next.js App Router)

Public-facing web application.

- **Port**: 3001
- **Framework**: Next.js with App Router
- **GraphQL**: urql client with codegen

### `shared`

Shared TypeScript code between admin-app and web-app.

- GraphQL schema definitions
- Shared types
- GraphQL codegen configuration

## Key Domains (server)

All domains live in `packages/server/src/domains/`. Each domain
contains `models/` (SQL queries), `activities/` (business logic
functions taking `&ServerDeps`), and optional `data/` types. HTTP
handlers live in `src/api/routes/{domain}.rs` and delegate to the
domain's activities.

| Domain | Purpose |
|--------|---------|
| `auth` | Phone/email OTP (Twilio Verify) |
| `contacts` | Contact info management |
| `editions` | Weekly broadsheet editions + layout engine |
| `jobs` | Background job management |
| `locations` | Geocoding and geo data |
| `media` | Presigned upload + media library |
| `member` | User profiles, registration |
| `memo` | LLM response caching |
| `notes` | Editorial notes |
| `organization` | Org management/approval |
| `posts` | Post/listing lifecycle |
| `providers` | Service provider profiles |
| `schedules` | Calendar parsing (RFC 5545) |
| `source` | Content sources |
| `tag` | Tagging/categorization |
| `widgets` | Widget authoring + layout |

> **Note**: Additional domains may exist in the codebase from the pre-pivot era (crawling, curator, extraction, etc.). These are legacy code from Root Signal. See [ROOT_EDITORIAL_PIVOT.md](ROOT_EDITORIAL_PIVOT.md).

## Dependency Graph

```
server (main crate)
├── ai-client (LLM abstraction)
├── twilio-rs (phone auth)
└── shared (via HTTP API, not a Rust dep)

admin-app (Next.js)
├── shared (GraphQL schema + resolvers)
└── server (resolvers POST to http://server:9080)

web-app (Next.js)
├── shared (GraphQL schema + resolvers)
└── server (resolvers POST to http://server:9080)
```

## Build & Run

```bash
# Rust server
cargo build --bin server
cargo run --bin server    # Starts on port 9080

# Admin panel
cd packages/admin-app && yarn dev    # Port 3000

# Public web app
cd packages/web-app && yarn dev      # Port 3001

# Infrastructure
docker compose up -d postgres minio
```
