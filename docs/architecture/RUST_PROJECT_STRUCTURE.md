# Rust Project Structure

## Overview

Root Editorial is a **single-crate Rust project** at `packages/server/`
with internal module organization. The project previously used a
multi-crate workspace (`crates/api/core/db/matching/scraper/`) but
consolidated into a single crate for simplicity.

## Top-Level Layout

```
packages/
├── server/           # Main Rust crate (single crate, all server code)
│   ├── Cargo.toml
│   ├── build.rs      # No-op (frontends served separately)
│   ├── migrations/   # SQLx migrations (231+ files, growing)
│   └── src/
│       ├── lib.rs            # Library root (server_core)
│       ├── bin/              # Binary entry points
│       ├── api/              # HTTP API: routes/, auth, error, state
│       ├── kernel/           # Shared infrastructure (ServerDeps, AI clients, etc.)
│       ├── common/           # Shared types (entity IDs, pagination, auth extractors)
│       ├── data_migrations/  # One-shot data migrations (not schema)
│       └── domains/          # Business domains
│
├── admin-app/        # Admin panel (Next.js, CMS, port 3000)
├── web-app/          # Public-facing site (Next.js, port 3001)
├── shared/           # Shared TypeScript (GraphQL schema + resolvers)
├── ai-client/        # Rust library crate (LLM client abstraction)
└── twilio-rs/        # Rust library crate (Twilio Verify wrapper)
```

## Binaries (`src/bin/`)

| Binary               | Purpose                                    |
|----------------------|--------------------------------------------|
| `server`             | Main Axum HTTP server (port 9080)          |
| `run_migrations`     | Run SQLx database migrations               |
| `migrate_cli`        | CLI for migration management               |
| `generate_embeddings` | Batch generate vector embeddings           |

## HTTP API (`src/api/`)

All HTTP routing lives here.

| Module            | Purpose                                       |
|-------------------|-----------------------------------------------|
| `routes/*.rs`     | One file per service (posts, editions, widgets, media, …). Each file declares request/response types, handler fns, and a `router()` that registers them. |
| `auth.rs`         | Auth extractors: `AdminUser`, `AuthenticatedUser`, `OptionalUser`. |
| `error.rs`        | `ApiError` + `ApiResult`. Returns `{"message": "..."}` JSON for BadRequest/Unauthorized/Forbidden/NotFound/Internal. |
| `state.rs`        | `AppState` — the `Arc<ServerDeps>` wrapper injected into every handler. |

Handlers follow a consistent URL convention:

- `/{Service}/{action}` — e.g. `POST /Posts/list_posts`
- `/{Object}/{id}/{action}` — e.g. `POST /Post/{id}/approve`

## Kernel (`src/kernel/`)

Shared infrastructure used by all domains:

| Module                 | Purpose                                    |
|------------------------|--------------------------------------------|
| `deps.rs`              | `ServerDeps` struct — dependency container held behind `Arc` |
| `mod.rs`               | Re-exports, AI model constants             |
| `ai_tools.rs`          | AI tool definitions for function calling   |
| `llm_request.rs`       | LLM request/response types                 |
| `pii.rs`               | PII detection and scrubbing                |
| `tag.rs`               | Tag resolution utilities                   |
| `traits.rs`            | Shared trait definitions                   |
| `test_dependencies.rs` | Test-only dependency stubs (used by the `TestHarness`) |

### ServerDeps

The central dependency container, created once at startup and shared
via `Arc<ServerDeps>`:

```rust
pub struct ServerDeps {
    pub db_pool: PgPool,
    pub openai: Arc<OpenAi>,
    pub claude: Option<Arc<Claude>>,
    pub embedding: Arc<EmbeddingService>,
    pub twilio: Arc<TwilioAdapter>,
    pub web_searcher: Arc<dyn WebSearcher>,
    pub pii_detector: Arc<dyn PiiDetector>,
    pub jwt_service: Arc<JwtService>,
    pub storage: Arc<dyn StorageAdapter>,  // MinIO / S3
    // ...
}
```

## Common (`src/common/`)

Shared types and utilities used across domains:

| Module              | Purpose                                         |
|---------------------|-------------------------------------------------|
| `entity_ids.rs`     | Type-safe ID wrappers (`PostId`, `OrganizationId`, `MemberId`, etc.) |
| `pagination.rs`     | Cursor and offset pagination types              |
| `types.rs`          | Shared data types                               |
| `extraction_types.rs` | Shared extraction types (`ExtractedPost`, …)  |
| `id.rs`             | Generic `Id<T>` wrapper backing entity IDs      |
| `pii/`              | PII detection module                            |
| `auth/`             | Authentication utilities (capabilities, actor checks) |
| `utils/`            | Misc helpers                                    |

### Entity IDs

Type-safe UUID wrappers prevent mixing up IDs:

```rust
// common/entity_ids.rs
pub type PostId = Id<Post>;
pub type OrganizationId = Id<Organization>;
pub type MemberId = Id<Member>;
// ... etc.
```

The `Id<T>` generic keeps the runtime representation a plain `Uuid`
but makes it a compile error to pass a `MemberId` where a `PostId` is
expected.

## Domains (`src/domains/`)

Business domains, each self-contained. The current set spans content
pipeline, user/auth, broadsheet editions, widgets, and infrastructure
support. See [PACKAGE_STRUCTURE.md](./PACKAGE_STRUCTURE.md) for the
full domain list.

### Domain Internal Structure

```
domains/{name}/
├── models/       # SQL queries + row structs (always present)
├── data/         # Shared data types (if the domain has API surface)
├── activities/   # Business logic functions taking &ServerDeps
├── loader.rs     # (optional) DataLoader for N+1 avoidance
└── mod.rs        # Re-exports the public API
```

HTTP handlers for a domain live in `src/api/routes/{domain}.rs` —
**not** inside the domain. Routes are a cross-cutting concern
(authorization, serialization, paths) and keeping them out of the
domain lets the domain be reused between routes and test harnesses.

## Key Dependencies

From `Cargo.toml`:

| Dependency    | Version | Purpose                          |
|---------------|---------|----------------------------------|
| `axum`        | 0.7     | HTTP/JSON server                 |
| `tokio`       | 1.x     | Async runtime                    |
| `sqlx`        | 0.8     | Async PostgreSQL with migrations |
| `serde`       | 1.x     | Serialization                    |
| `pgvector`    | 0.4     | Vector similarity search         |
| `jsonwebtoken`| 9       | JWT authentication               |
| `aws-sdk-s3`  | 1.x     | S3-compatible storage (MinIO/S3) |
| `rig-core`    | 0.9     | LLM framework                    |
| `ai-client`   | local   | LLM client abstraction           |

## Runtime Architecture

```
Next.js Frontend (admin-app :3000, web-app :3001)
    ↓ HTTPS + GraphQL
GraphQL resolvers (in-process in Next.js API routes)
    ↓ HTTP/JSON
Rust Axum Server (port 9080)
    ├── src/api/routes/   — HTTP handlers (one file per service)
    ├── src/domains/      — business logic (activities + models)
    └── src/kernel/       — ServerDeps (DB, AI, storage)
    ↓
PostgreSQL + pgvector
```

## Historical Note

The project previously used:
- **Multi-crate workspace** (`crates/api/core/db/matching/scraper/`)
  → consolidated to single crate.
- **seesaw-rs** (event-driven: Events → Machines → Commands → Effects
  → Edges) → briefly replaced by Restate SDK, which was itself removed
  on 2026-03-17 in favor of plain Axum HTTP handlers.
- **Embedded frontends** (rust-embed) → frontends now served
  separately (Next.js standalone).
- **Restate SDK** (durable workflow execution) → removed; short
  request/response workloads didn't justify the runtime overhead.
  See `ARCHITECTURE_DECISIONS.md` Decision 4.
