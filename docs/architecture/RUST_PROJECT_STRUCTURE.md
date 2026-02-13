# Rust Project Structure

## Overview

MN Together is a **single-crate Rust project** at `packages/server/` with internal module organization. The project previously used a multi-crate workspace (`crates/api/core/db/matching/scraper/`) but consolidated into a single crate for simplicity.

## Top-Level Layout

```
packages/
├── server/           # Main Rust crate (single crate, all server code)
│   ├── Cargo.toml
│   ├── build.rs      # No-op (frontends served separately)
│   ├── migrations/   # SQLx migrations (000001 through 000170+)
│   └── src/
│       ├── lib.rs            # Library root (server_core)
│       ├── bin/              # Binary entry points
│       ├── kernel/           # Shared infrastructure (ServerDeps, AI clients, etc.)
│       ├── common/           # Shared types (entity IDs, pagination, restate_serde)
│       └── domains/          # Business domains (22 domains)
│
├── web/              # Next.js frontend (App Router)
├── admin-app/        # Admin panel (Next.js, separate from web)
├── shared/           # Shared TypeScript (GraphQL schema, types)
├── extraction/       # Rust library crate (web scraping, Firecrawl, Tavily)
├── ai-client/        # Rust library crate (LLM client abstraction)
├── apify-client/     # Rust library crate (Apify API client)
├── twilio-rs/        # Rust library crate (Twilio Verify wrapper)
└── openai-client/    # Rust library crate (OpenAI API client)
```

## Binaries (`src/bin/`)

| Binary               | Purpose                                    |
|------------------------|--------------------------------------------|
| `server`              | Main Restate workflow server (port 9080) + SSE server (port 8081) |
| `run_migrations`      | Run SQLx database migrations               |
| `migrate_cli`         | CLI for migration management               |
| `generate_embeddings` | Batch generate vector embeddings           |

## Kernel (`src/kernel/`)

Shared infrastructure used by all domains:

| Module                | Purpose                                    |
|-----------------------|--------------------------------------------|
| `deps.rs`             | `ServerDeps` struct — dependency container passed to all workflows/activities |
| `mod.rs`              | Re-exports, AI model constants (`GPT_5_MINI`, `Claude`, `OpenAi`) |
| `ai_tools.rs`         | AI tool definitions for function calling    |
| `extraction_service.rs` | Extraction service factory                |
| `llm_request.rs`      | LLM request/response types                 |
| `nats.rs`             | NATS messaging client                      |
| `pii.rs`              | PII detection and scrubbing                |
| `sse.rs`              | SSE (Server-Sent Events) streaming server  |
| `stream_hub.rs`       | Pub/sub hub for real-time events           |
| `tag.rs`              | Tag resolution utilities                   |
| `traits.rs`           | Shared trait definitions                   |
| `test_dependencies.rs` | Test-only dependency stubs                |

### ServerDeps

The central dependency container, created once in `server.rs` and shared via `Arc<ServerDeps>`:

```rust
pub struct ServerDeps {
    pub db_pool: PgPool,
    pub ingestor: Arc<dyn Ingestor>,
    pub openai: Arc<OpenAi>,
    pub claude: Option<Arc<Claude>>,
    pub embedding: Arc<EmbeddingService>,
    pub expo: Arc<ExpoClient>,
    pub twilio: Arc<TwilioAdapter>,
    pub web_searcher: Arc<dyn WebSearcher>,
    pub pii_detector: Arc<dyn PiiDetector>,
    pub extraction: Option<ExtractionService>,
    pub jwt_service: Arc<JwtService>,
    pub stream_hub: StreamHub,
    pub apify: Option<Arc<ApifyClient>>,
    // ...
}
```

## Common (`src/common/`)

Shared types and utilities used across domains:

| Module              | Purpose                                         |
|---------------------|-------------------------------------------------|
| `entity_ids.rs`     | Type-safe ID wrappers (`PostId`, `OrganizationId`, `MemberId`, etc.) |
| `restate_serde.rs`  | `impl_restate_serde!` macro for Restate SDK serialization |
| `restate_types.rs`  | Shared Restate request/response types (`EmptyRequest`, etc.) |
| `pagination.rs`     | Cursor and offset pagination types              |
| `embedding.rs`      | Embedding generation utilities                  |
| `types.rs`          | Shared data types                               |
| `pii/`              | PII detection module                            |
| `auth/`             | Authentication utilities                        |

### Entity IDs

Type-safe UUID wrappers prevent mixing up IDs:

```rust
// common/entity_ids.rs
pub struct PostId(Uuid);
pub struct OrganizationId(Uuid);
pub struct MemberId(Uuid);
pub struct SourceId(Uuid);
pub struct ContainerId(Uuid);
// ... etc.
```

### Restate Serialization

Restate SDK has its own Serialize/Deserialize traits (not serde's). The `impl_restate_serde!` macro bridges them:

```rust
// common/restate_serde.rs
impl_restate_serde!(CurateOrgRequest);  // Makes type work with Restate
```

## Domains (`src/domains/`)

22 business domains, each self-contained:

### Core Content Pipeline
| Domain       | Purpose                                      | Restate Types |
|-------------|----------------------------------------------|---------------|
| `curator`   | AI curator pipeline (brief → analyze → stage) | Workflows     |
| `posts`     | Post/listing CRUD and lifecycle               | Services, Objects, Workflows |
| `organization` | Organization management and approval       | Services, Workflows |
| `source`    | Content sources (websites, social profiles)   | Services, Objects, Workflows |
| `extraction` | Web page extraction and parsing              | Services      |
| `crawling`  | Website crawling orchestration               | Workflows     |
| `website`   | Website research and post regeneration       | Services, Objects, Workflows |
| `sync`      | Sync batches and proposals                   | Services      |
| `notes`     | Editorial notes on organizations             | Services      |

### User & Communication
| Domain           | Purpose                                  | Restate Types |
|-----------------|------------------------------------------|---------------|
| `auth`          | Phone OTP authentication (Twilio Verify) | Services      |
| `member`        | User profiles and registration           | Services, Objects, Workflows |
| `chatrooms`     | Real-time chat with LLM                  | Services, Objects |
| `contacts`      | Contact information management           | (models only) |
| `social_profile` | Social media profile linking            | Services      |
| `providers`     | Service provider profiles                | Services, Objects |

### Infrastructure & Support
| Domain      | Purpose                                    | Restate Types |
|------------|---------------------------------------------|---------------|
| `agents`   | AI agent identity and tracking              | (activities/models only) |
| `heat_map` | Geographic density visualization            | Services      |
| `jobs`     | Background job management                   | Services      |
| `locations` | Geographic data and geocoding              | (models only) |
| `memo`     | LLM response caching (content-addressed)   | (models only) |
| `schedules` | Calendar/schedule parsing (RFC 5545)       | (models only) |
| `tag`      | Tagging and categorization                  | Services      |

### Domain Internal Structure

Each domain follows the pattern described in [DOMAIN_ARCHITECTURE.md](./DOMAIN_ARCHITECTURE.md):

```
domains/{name}/
├── models/       # SQL models (always present)
├── data/         # API data types (if domain has API surface)
├── activities/   # Business logic functions (if domain has logic)
├── restate/      # Restate handlers (if domain is callable)
│   ├── services/
│   ├── workflows/
│   └── virtual_objects/
└── mod.rs
```

## Key Dependencies

From `Cargo.toml`:

| Dependency     | Version | Purpose                          |
|---------------|---------|----------------------------------|
| `restate-sdk` | 0.4.0   | Durable workflow execution       |
| `sqlx`        | 0.8     | Async PostgreSQL with migrations |
| `tokio`       | 1.x     | Async runtime                    |
| `axum`        | 0.8     | SSE HTTP server                  |
| `rig-core`    | 0.9     | LLM framework                   |
| `serde`       | 1.x     | Serialization                    |
| `pgvector`    | 0.4     | Vector similarity search         |
| `async-nats`  | 0.38    | NATS messaging                   |
| `jsonwebtoken` | 9      | JWT authentication               |
| `extraction`  | local   | Web scraping (Firecrawl, Tavily) |
| `ai-client`   | local   | LLM client abstraction           |

## Runtime Architecture

```
Next.js Frontend (port 3000)
    ↓ HTTP
Restate Runtime (port 9070, proxy/gateway)
    ↓ HTTP
Rust Server (port 9080, Restate endpoint)
    ├── Services (stateless handlers)
    ├── Workflows (durable multi-step)
    └── Virtual Objects (keyed state)
    ↓
PostgreSQL + NATS

SSE Server (port 8081, axum)
    ↑ WebSocket/SSE
Next.js Frontend (real-time updates)
```

## Historical Note

The project previously used:
- **Multi-crate workspace** (`crates/api/core/db/matching/scraper/`) → consolidated to single crate
- **seesaw-rs** (event-driven: Events → Machines → Commands → Effects → Edges) → replaced with Restate SDK
- **Embedded frontends** (rust-embed) → frontends now served separately
