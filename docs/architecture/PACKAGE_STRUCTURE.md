# Package Structure

## Overview

Root Editorial is a monorepo with a Rust backend and TypeScript frontends. The main server is a single Rust crate with multiple internal business domains.

## Package Map

```
packages/
├── server/          # Rust — Restate workflow server (the backend)
├── admin-app/       # TypeScript — Next.js CMS admin panel
├── web-app/         # TypeScript — Next.js public web app
├── shared/          # TypeScript — Shared GraphQL schema and types
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
- **Shared types**: `src/common/` (entity IDs, pagination, restate_serde)
- **Migrations**: `migrations/` (000001–000170+)

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

All domains live in `packages/server/src/domains/`:

| Domain | Purpose | Has Restate | Has Activities |
|--------|---------|-------------|----------------|
| `auth` | Phone/email OTP (Twilio Verify) | Services | Yes |
| `contacts` | Contact info management | No | No |
| `jobs` | Background job management | Services | No |
| `locations` | Geocoding and geo data | No | No |
| `member` | User profiles, registration | Services, Objects, Workflows | Yes |
| `memo` | LLM response caching | No | No |
| `notes` | Editorial notes | Services | Yes |
| `organization` | Org management/approval | Services, Workflows | No |
| `posts` | Post/listing lifecycle | Services, Objects, Workflows | Yes |
| `providers` | Service provider profiles | Services, Objects | Yes |
| `schedules` | Calendar parsing (RFC 5545) | No | No |
| `source` | Content sources | Services, Objects, Workflows | Yes |
| `tag` | Tagging/categorization | Services | No |

> **Note**: Additional domains may exist in the codebase from the pre-pivot era (chatrooms, crawling, curator, extraction, etc.). These are dead code scheduled for removal. See [ROOT_EDITORIAL_PIVOT.md](../ROOT_EDITORIAL_PIVOT.md).

## Dependency Graph

```
server (main crate)
├── ai-client (LLM abstraction)
├── twilio-rs (phone auth)
└── shared (via GraphQL API, not Rust dep)

admin-app (Next.js)
├── shared (GraphQL types)
└── server (via HTTP → Restate runtime)

web-app (Next.js)
├── shared (GraphQL types)
└── server (via HTTP → Restate runtime)
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
docker compose up -d postgres redis restate
```
