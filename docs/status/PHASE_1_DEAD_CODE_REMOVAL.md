# Phase 1 Postmortem: Dead Code Removal

**Date**: 2026-02-24
**Commit**: `de9f4e3`
**Stats**: 262 files changed, 45,947 lines deleted, 102 lines added

---

## What This Was

Phase 1 of the Root Editorial Pivot — strip everything left over from the Root Signal extraction pipeline that no longer serves the Root Editorial CMS product. The goal: a codebase that only contains code we actually run.

## Scope: Plan vs. Reality

The pivot doc called for removing 5 packages, 8 server domains, and cleaning up config/GraphQL. In practice we removed more than planned because dead code had tendrils everywhere.

**Planned deletions** (from `ROOT_EDITORIAL_PIVOT.md`):
- 5 TS/Rust packages
- 8 server domains (crawling, extraction, website, social_profile, sync, curator, newsletter, providers)
- Config cleanup (.env.example, Cargo.toml, docker-compose.yml, package.json)
- GraphQL resolver + schema cleanup

**Actual deletions** (superset of planned):
- 4 Rust/TS packages (search-app, extraction, openai-client, apify-client)
- 11 server domain directories (added source, plus nested workflow dirs)
- 2 kernel infrastructure modules (extraction_service.rs, nats.rs)
- 2 common infrastructure modules (nats.rs, nats_tap.rs)
- 1 common utility (expo.rs — push notifications)
- 1 dead binary (generate_embeddings)
- 11 dead activity files across alive domains (posts, notes)
- 3 dead workflow directories (posts, organization)
- 5 GraphQL resolver files
- ~100 lines of dead GraphQL schema

The `web/` package was kept (still has build artifacts being referenced) — that's for a separate cleanup.

## Inventory of Deletions

### Packages Removed
| Package | Lines | What it was |
|---------|-------|-------------|
| `extraction/` | ~13k | Crawling/ingestion pipeline library (Firecrawl, HTTP ingestors, signal detection) |
| `openai-client/` | ~2.2k | Superseded by `ai-client` crate |
| `apify-client/` | ~430 | Web scraping API client |
| `search-app/` | ~270 | Dead Next.js stub for standalone search |

### Server Domains Removed (11)
| Domain | Lines | What it was |
|--------|-------|-------------|
| `crawling/` | ~2.4k | Website crawling orchestration, page extraction |
| `extraction/` | ~940 | Extraction service bridge (server-side wrapper around extraction lib) |
| `website/` | ~2.3k | Website models, assessments, research, approval workflows |
| `social_profile/` | ~570 | Instagram/social media profile ingestion |
| `source/` | ~2.9k | Unified source management (website, social, newsletter sources) |
| `sync/` | ~1.6k | Post sync batches, proposals, merge operations |
| `curator/` | ~2.8k | AI curator — brief extraction, writing, safety review |
| `newsletter/` | ~1.1k | Newsletter subscription, confirmation, webhook ingestion |
| `providers/` | ~1.4k | Provider directory (community organizations) |

### Kernel / Common Infrastructure
| File | What it was |
|------|-------------|
| `kernel/extraction_service.rs` | Bridge between server and extraction library |
| `kernel/nats.rs` | NATS messaging publisher for real-time events |
| `common/nats.rs` | NATS payload serialization trait |
| `common/nats_tap.rs` | NATS event tap for debugging |
| `common/utils/expo.rs` | Expo push notification client |
| `bin/generate_embeddings.rs` | One-off script for website assessment embeddings |

### Dead Code Within Alive Domains
| Domain | Files deleted | What they were |
|--------|-------------|----------------|
| `posts/activities/` | 11 files | scraping, deduplication, llm_sync, post_discovery, syncing, sync_operations, sync_utils, post_sync_handler, resource_link_* (3 files) |
| `posts/restate/workflows/` | 3 files | deduplicate_posts, extract_posts_from_url, mod |
| `organization/restate/workflows/` | 3 files | extract_org_posts, clean_up_org_posts, mod |
| `notes/activities/` | 1 file | extraction.rs (note generation from crawled content) |

### ServerDeps Fields Removed (5)
| Field | Type | What it was |
|-------|------|-------------|
| `ingestor` | `Arc<dyn Ingestor>` | Web page ingestion (Firecrawl/HTTP) |
| `push_service` | `Arc<dyn BasePushNotificationService>` | Expo mobile push |
| `web_searcher` | `Arc<dyn WebSearcher>` | Tavily web search |
| `extraction` | `Option<Arc<OpenAIExtractionService>>` | Extraction pipeline bridge |
| `apify_client` | `Option<ApifyClient>` | Apify scraping API |

### Kernel Traits Removed
| Trait | What it was |
|-------|-------------|
| `BasePushNotificationService` | Expo push notification interface |

### AI Tools Removed
| Tool | What it was |
|------|-------------|
| `WebSearchTool` | Tavily search for agent conversations |
| `FetchPageTool` | URL fetch/ingest for agent conversations |

## Stubbed Handlers (Not Deleted)

These Restate service handlers had their implementations replaced with `TerminalError` returns. The trait method signatures are preserved to maintain the API surface — removing them from the trait would require coordinated GraphQL schema changes.

| Service | Handler | Why stubbed |
|---------|---------|-------------|
| `PostsService` | `submit_resource_link` | Called deleted `scraping::submit_resource_link` |
| `PostsService` | `deduplicate` | Called deleted `deduplication::deduplicate_posts` |
| `PostsService` | `deduplicate_cross_source` | Called deleted `deduplication::deduplicate_cross_source_all_orgs` |
| `PostObject` | `regenerate` | Called deleted `crawling::activities::regenerate_single_post` |
| `NotesService` | `generate_notes` | Called deleted `notes::activities::extraction::generate_notes_for_organization` |
| `OrganizationsService` | 6 handlers removed entirely | `regenerate`, `backfill_organizations`, `extract_org_posts`, `run_scheduled_extraction`, `clean_up_org_posts`, `run_curator` — these were removed from the trait itself since no frontend calls them |

## Surprises and Gotchas

### 1. Chatrooms domain was alive (rescued from deletion)

The chatrooms domain looked dead — it's not in the pivot doc's "keep" list and has no obvious connection to editorial. But `PostObject`'s `add_comment` and `get_comments` handlers depend on `chatrooms::models::Container` and `chatrooms::models::Message`. Post comments are very much alive in the CMS.

**Fix**: Restored from git (`git checkout HEAD -- packages/server/src/domains/chatrooms/`), added `pub mod chatrooms;` back to `domains/mod.rs`.

**Lesson**: Always grep for `use crate::domains::<name>` across the *entire* codebase before deleting a domain, not just the domain's own files.

### 2. Accidental dependency removal (bytes, schemars)

When cleaning Cargo.toml workspace members and dependencies, `bytes` and `schemars` were removed because they looked like extraction-library dependencies. But:
- `bytes` is used by the `impl_restate_serde!` macro (Restate SDK serialization bridge)
- `schemars` is used by `JsonSchema` derives in alive AI extraction types, PII detector, and ai_tools

**Fix**: Added both back to `[dependencies]` in `packages/server/Cargo.toml`.

**Lesson**: Don't remove dependencies based on "looks like it belongs to dead code." Grep for actual usage: `use schemars`, `use bytes`, `bytes::Bytes` across alive code before removing.

### 3. Resource link pipeline was entirely dead

The resource link submission pipeline (user submits a URL → scrape → extract → create posts) wasn't called out in the pivot doc as dead, but every step depends on deleted infrastructure:
- `resource_link_scraping.rs` → uses `FirecrawlIngestor`, `HttpIngestor`, `deps.extraction` (all removed)
- `resource_link_creation.rs` → uses `Website::find_by_domain` (website domain removed)
- `resource_link_extraction.rs` → no callers remain

**Fix**: Deleted all 3 files, stubbed the `submit_resource_link` handler.

**Lesson**: After removing infrastructure, grep for the removed types/fields to find code that's now *transitively* dead — code that compiled before but can't anymore.

### 4. Organization service had massive hidden dead code

The `OrganizationsService` had 6 handlers (~390 lines) that were dead — they called into crawling activities, curator workflows, and source models. The `OrganizationResult` struct also carried 3 dead fields (`website_count`, `social_profile_count`, `snapshot_count`) populated by queries against deleted tables.

**Lesson**: When a domain is "alive" but was tightly integrated with dead domains, expect to find dead handlers hiding inside it. Check every handler's implementation, not just imports.

## Verification

| Check | Result |
|-------|--------|
| `cargo check` | 0 errors, 0 warnings |
| `tsc --noEmit` (web-app) | Clean |
| `tsc --noEmit` (admin-app) | Clean |
| Dead import grep (`use crate::domains::{dead}`) | 0 matches |
| Dead field grep (`deps.ingestor`, `deps.push_service`, etc.) | 0 matches |
| Dead crate grep (`use extraction::`, `use apify_client::`) | 0 matches |

## What Went Well

1. **Layer-by-layer approach worked**: Cleaning kernel → common → domains (bottom-up) prevented cascading errors. Each layer was fixed before moving to the next.

2. **Parallel agents for complex files**: Three background agents handled the organization service (390 lines of surgery), post virtual object, and jobs service simultaneously — saved significant time.

3. **Grep-driven discovery**: Every deletion was preceded by a grep for usage. This caught the chatrooms rescue, the resource link pipeline, and the `bytes`/`schemars` dependency issues before they became multi-hour debugging sessions.

4. **Stubbing over removing for Restate handlers**: Replacing handler bodies with `TerminalError` instead of removing trait methods preserved the API surface. This is safer than coordinated schema + trait + resolver changes all at once.

## What Could Be Better

1. **The pivot doc's deletion list was incomplete**. It didn't mention the `source` domain, the resource link pipeline, the NATS infrastructure, Expo push notifications, or the `generate_embeddings` binary. Future phases should do a full dependency trace before starting.

2. **No automated dead-code detection**. We relied on manual grepping and cargo check errors. A tool that traces from `main()` and marks unreachable code would have caught everything upfront.

3. **The chatrooms near-miss** could have been avoided with a pre-deletion script: `for domain in $DOMAINS_TO_DELETE; do grep -r "use crate::domains::$domain" --include="*.rs" src/ | grep -v "src/domains/$domain"; done`

## Remaining Cleanup — Resolved

All items originally deferred from Phase 1 have been completed:

- ~~Stubbed handlers should eventually be removed from Restate traits + GraphQL schema~~ — **Done.** All stubs removed or replaced with proper error responses during the Restate→HTTP migration.
- ~~Dead database tables from deleted domains still exist~~ — **Done.** Dropped in earlier migrations (149, 148, 136, etc.).
- ~~`packages/web/` was not deleted~~ — **Done.** Deleted 2026-03-08 (only contained empty build cache dirs).
- `data_migrations/normalize_website_urls.rs` is a no-op migration (safe to leave, annotated).

## Stats Summary

| Metric | Value |
|--------|-------|
| Files deleted | 258 |
| Files modified | 4 packages + 26 server files |
| Lines removed | 45,947 |
| Lines added | 102 (stubs + fixes) |
| Net reduction | 45,845 lines |
| Compilation errors fixed | ~45 |
| Domains removed | 11 |
| Packages removed | 4 |
| Time | ~3 hours across 2 sessions |
