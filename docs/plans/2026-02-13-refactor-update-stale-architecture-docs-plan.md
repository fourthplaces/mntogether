---
title: Update Stale Architecture Documentation
type: refactor
date: 2026-02-13
---

# Update Stale Architecture Documentation

## Overview

Several files in `docs/architecture/` are severely outdated due to two major codebase evolutions:
1. **Seesaw to Restate migration** - The event-driven seesaw-rs framework was fully replaced by Restate durable workflows
2. **Multi-crate to single-crate restructure** - The `crates/api/core/db/matching/scraper/` workspace was consolidated into a single `packages/server/` crate with 22 internal domains

This plan covers auditing, deleting, and rewriting the stale documentation.

## Audit Summary

| Document | Accuracy | Action |
|----------|----------|--------|
| `SEESAW_ARCHITECTURE.md` | 0% | **DELETE** |
| `DOMAIN_ARCHITECTURE.md` | ~15% | **REWRITE** |
| `RUST_PROJECT_STRUCTURE.md` | ~25% | **REWRITE** |
| `PACKAGE_STRUCTURE.md` | ~20% | **REWRITE** |
| `RUST_IMPLEMENTATION.md` | ~50% | **UPDATE** |
| `docs/guides/EMBEDDED_FRONTENDS.md` | ~60% | **UPDATE** |

Current/accurate docs (no changes): `CURATOR_PIPELINE.md`, `DATABASE_SCHEMA.md`, `domain-approval-workflow.md`, `DATA_MODEL.md`, `COMPONENT_INVENTORY.md`, `SCHEMA_DESIGN.md`, `PII_SCRUBBING.md`, `DESIGN_TOKENS.md`, `TAGS_VS_FIELDS.md`, `SIMPLIFIED_SCHEMA.md`, `SCHEMA_RELATIONSHIPS.md`, `CAUSE_COMMERCE_ARCHITECTURE.md`, `CHAT_ARCHITECTURE.md`.

---

## Acceptance Criteria

- [ ] `SEESAW_ARCHITECTURE.md` is deleted
- [ ] `DOMAIN_ARCHITECTURE.md` accurately describes the Restate workflow + activities pattern with correct code examples
- [ ] `RUST_PROJECT_STRUCTURE.md` accurately describes the single-crate structure with 22 domains
- [ ] `PACKAGE_STRUCTURE.md` lists all 22 actual domains with correct responsibilities
- [ ] `RUST_IMPLEMENTATION.md` references Restate workflows instead of seesaw; describes curator pipeline
- [ ] `docs/guides/EMBEDDED_FRONTENDS.md` references correct package paths (`packages/web/`)
- [ ] All code examples in updated docs compile conceptually (match actual patterns in codebase)
- [ ] No remaining references to "seesaw" in any architecture doc (except historical context if needed)

---

## Implementation Phases

### Phase 1: Delete SEESAW_ARCHITECTURE.md

Simply delete the file. It describes a system that no longer exists. The Restate patterns are already documented in `CLAUDE.md` and `CURATOR_PIPELINE.md`.

**File:** `docs/architecture/SEESAW_ARCHITECTURE.md`

---

### Phase 2: Rewrite DOMAIN_ARCHITECTURE.md

The current doc describes a 7-layer seesaw pattern (Models → Data → Events → Commands → Machines → Effects → Edges). The actual architecture is simpler.

**Preserve:**
- Models layer description (still accurate)
- Data layer description (still accurate)
- Anti-patterns section (mostly still applies)
- General separation-of-concerns philosophy

**Remove entirely:**
- Events layer (no domain events in current code)
- Commands layer (no commands)
- Machines layer (no seesaw machines)
- Effects layer (replaced by activities)
- Edges / `dispatch_request` pattern (gone)
- All seesaw code examples

**Replace with:**

#### New Layer Structure

```
domains/<name>/
├── models/        # Database queries (sqlx::query_as) — data access layer
├── data/          # GraphQL/API types, loaders
├── activities/    # Pure async functions taking &ServerDeps
├── restate/
│   ├── workflows/     # Durable orchestration (#[restate_sdk::workflow])
│   ├── services/      # Stateless request handlers
│   └── virtual_objects/ # Stateful handlers (keyed by entity)
└── types.rs       # Domain-specific types
```

#### New Data Flow

```
API Request → Restate Runtime → Workflow → Activities → Models → Database
                                    ↓
                              ctx.run() for durability
                                    ↓
                              Activities (pure functions)
                                    ↓
                              Models (DB queries)
```

#### Code Examples to Include

**Workflow example** — Use curator pattern from `domains/curator/restate/workflows/curate_org.rs`:
- Trait with `#[restate_sdk::workflow]` macro
- Impl struct with `Arc<ServerDeps>`
- `ctx.set()` for status tracking
- `ctx.run()` for durable blocks
- Activities called as plain async functions

**Activity example** — Use brief extraction from `domains/curator/activities/brief_extraction.rs`:
- Pure async function taking `&ServerDeps`
- Returns `Result<T>`
- Uses `deps.memo()` for caching
- Bounded concurrency with `buffer_unordered()`

**Model example** — Use existing sqlx patterns from any domain model:
- `sqlx::query_as::<_, Self>(...)` (matching CLAUDE.md rules)
- `.bind()` chain
- `.fetch_one()` / `.fetch_all()` / `.fetch_optional()`

---

### Phase 3: Rewrite RUST_PROJECT_STRUCTURE.md

The current doc describes a 5-crate workspace (`crates/api/core/db/matching/scraper/`). Reality is a single crate.

**Preserve:**
- SQLx usage patterns
- pgvector embedding concepts
- General philosophy of separation

**Remove entirely:**
- Multi-crate workspace description
- Individual crate sections (api, core, db, matching, scraper)
- Cargo.toml workspace member examples
- File structure showing `crates/` directory

**Replace with:**

#### Actual Project Layout

```
packages/
├── server/              # Main Rust backend (single crate)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── bin/
│   │   │   ├── server.rs          # Restate workflow server + SSE
│   │   │   ├── migrate_cli.rs     # Migration CLI
│   │   │   ├── run_migrations.rs  # Automated migrations
│   │   │   └── generate_embeddings.rs
│   │   ├── domains/               # 22 domain modules
│   │   │   ├── agents/
│   │   │   ├── auth/
│   │   │   ├── chatrooms/
│   │   │   ├── contacts/
│   │   │   ├── crawling/
│   │   │   ├── curator/
│   │   │   ├── extraction/
│   │   │   ├── heat_map/
│   │   │   ├── jobs/
│   │   │   ├── locations/
│   │   │   ├── member/
│   │   │   ├── memo/
│   │   │   ├── notes/
│   │   │   ├── organization/
│   │   │   ├── posts/
│   │   │   ├── providers/
│   │   │   ├── schedules/
│   │   │   ├── social_profile/
│   │   │   ├── source/
│   │   │   ├── sync/
│   │   │   ├── tag/
│   │   │   └── website/
│   │   ├── kernel/                # Cross-cutting concerns
│   │   │   ├── deps.rs            # ServerDeps (DI container)
│   │   │   ├── llm_request.rs
│   │   │   ├── nats.rs
│   │   │   ├── pii.rs
│   │   │   ├── sse.rs
│   │   │   └── stream_hub.rs
│   │   └── common/                # Shared utilities
│   │       ├── restate_serde.rs   # impl_restate_serde! macro
│   │       ├── entity_ids.rs      # Type-safe ID wrappers
│   │       ├── embedding.rs
│   │       └── pagination.rs
│   └── migrations/                # SQLx migrations (NEVER EDIT)
├── web/                 # Next.js frontend (App Router)
│   ├── app/
│   │   ├── (public)/              # Public routes
│   │   ├── admin/                 # Admin dashboard
│   │   └── api/
│   │       └── restate/           # Restate invocation endpoints
│   ├── components/
│   └── lib/
│       └── restate/               # Restate client SDK
├── extraction/          # Extraction library (separate crate)
├── ai-client/           # AI client library (separate crate)
└── apify-client/        # Apify client (separate crate)
```

#### Key Dependencies

```
restate-sdk = "0.4.0"     # Durable workflow execution
sqlx = "0.8"              # PostgreSQL with compile-time checks
tokio = "1.44"            # Async runtime
axum = "0.8"              # HTTP server (SSE endpoints)
rig-core = "0.9"          # LLM framework
async-nats = "0.38"       # Real-time messaging
pgvector = "0.4"          # Vector similarity search
```

#### Runtime Architecture

```
Next.js (port 3000) → Restate Runtime (port 9070) → Workflow Server (port 9080)
                                                          ↓
                                                     PostgreSQL + NATS
```

---

### Phase 4: Rewrite PACKAGE_STRUCTURE.md

The current doc lists 5 domains (volunteer, need, notification, csv_import, discovery). None of those match reality.

**Preserve:**
- Privacy-first design philosophy
- Text-first storage concept
- Markdown support

**Remove entirely:**
- 5-domain listing and descriptions
- Embedded admin SPA description (uses rust-embed pattern that's outdated)
- Incorrect Cargo.toml structure

**Replace with:**

#### All 22 Domains with Descriptions

| Domain | Has Workflows | Purpose |
|--------|:---:|---------|
| `agents` | - | Agent-related functionality and event orchestration |
| `auth` | Restate services | Authentication, JWT, OTP via Twilio |
| `chatrooms` | Yes | Real-time messaging |
| `contacts` | - | Contact data models |
| `crawling` | Yes | Website crawling orchestration (Firecrawl/Apify) |
| `curator` | Yes | LLM-based content curation pipeline (7 phases) |
| `extraction` | Yes | Content extraction from crawled pages |
| `heat_map` | Yes | Geographic/demographic heat mapping |
| `jobs` | Yes | Job scheduling and queue management |
| `locations` | - | Location/geography data |
| `member` | Yes | User/member profiles |
| `memo` | - | LLM result caching (keyed by prompt+content) |
| `notes` | Yes | Note-taking and alerts |
| `organization` | Yes | Community organization management |
| `posts` | Yes | Post content, approval workflows |
| `providers` | Yes | External provider integrations (Firecrawl, Tavily) |
| `schedules` | - | Event scheduling (RFC 5545 rrule) |
| `social_profile` | Yes | Social media profile management |
| `source` | Yes | Content source management and ingestion |
| `sync` | Yes | Data synchronization |
| `tag` | Yes | Tagging/categorization system |
| `website` | Yes | Website metadata and relationships |

#### Domain Internal Structure Pattern

Show the standard domain structure:
```
domains/<name>/
├── mod.rs
├── models/          # DB queries (sqlx::query_as)
├── data/            # API types, GraphQL loaders
├── activities/      # Pure async fns taking &ServerDeps
├── restate/
│   └── workflows/   # Durable orchestrations
└── types.rs         # Domain types
```

Not all domains have all directories — simpler domains may only have `models/`.

---

### Phase 5: Update RUST_IMPLEMENTATION.md

This doc's core philosophy is still valid but the implementation details are wrong.

**Preserve:**
- "Relevance notifier, not perfect matcher" philosophy
- Bias toward recall
- Privacy-first approach
- "What we're NOT building" section

**Update:**
- Replace NotificationEngine code with curator pipeline overview
- Replace simplified architecture diagram with Restate workflow flow
- Update data models to show current tables (organizations, posts, sync_proposals, etc.)
- Reference `CURATOR_PIPELINE.md` for detailed pipeline docs
- Remove seesaw references

---

### Phase 6: Update docs/guides/EMBEDDED_FRONTENDS.md

Light touch — just update package paths.

**Changes:**
- `packages/admin-spa/` → verify current path
- `packages/web-app/` → `packages/web/`
- Update any build.rs references to match current setup

---

## Cross-Reference Validation

After all updates, verify:
- [ ] No architecture doc references "seesaw" (except maybe a "Previously used seesaw-rs, migrated to Restate" note)
- [ ] All file paths mentioned in docs actually exist in the codebase
- [ ] Code examples match patterns in CLAUDE.md (especially `sqlx::query_as::<_, Type>` not macro version)
- [ ] Domain count is consistent across all docs (22 domains)
- [ ] Runtime ports are consistent (3000, 9070, 9080)
- [ ] `CURATOR_PIPELINE.md` is referenced from relevant docs as the exemplar workflow

## References

### Source of Truth Files
- `packages/server/Cargo.toml` — dependencies
- `packages/server/src/domains/` — actual domain list
- `packages/server/src/bin/server.rs` — binary entry point
- `packages/server/src/kernel/deps.rs` — ServerDeps container
- `packages/server/src/common/restate_serde.rs` — Restate serialization macro
- `CLAUDE.md` — Restate workflow patterns and coding rules

### Exemplar Documentation
- `docs/architecture/CURATOR_PIPELINE.md` — gold standard for current architecture docs
