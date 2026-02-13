---
title: Server Consistency Audit - Duplicate Code & Legacy Flow Removal
type: refactor
date: 2026-02-03
status: planned
---

# Server Consistency Audit - Duplicate Code & Legacy Flow Removal

## Overview

Comprehensive audit of `packages/server` to eliminate duplicate code, remove legacy/deprecated flows, and ensure consistency with documented architectural patterns (Seesaw 0.6.0+, CLAUDE.md conventions).

**Scope:** Full server codebase audit covering:
- Duplicate implementations
- Deprecated code still in use
- Pattern violations (effect handlers >50 lines, business logic in effects)
- Type inconsistencies
- Dead code paths

## Problem Statement

The server codebase has accumulated technical debt during the extraction library migration. Multiple parallel systems exist for the same functionality, deprecated code is still being called, and architectural patterns are inconsistently applied.

**Key Issues Identified:**

| Category | Count | Impact |
|----------|-------|--------|
| Duplicate functions | 3+ | Maintenance burden, inconsistent behavior |
| Deprecated functions still called | 8+ | Migration incomplete, confusion |
| Effect handlers >50 lines | 1+ | Violates seesaw architecture |
| Large files needing split | 1 | 1786 lines in single file |
| Type definitions | 3+ ContactInfo | Schema confusion, conversion overhead |

## Detailed Findings

### 1. Duplicate Code

#### `truncate_content()` - 3 Implementations

| File | Line | Behavior |
|------|------|----------|
| `crawling/effects/extraction/summarize.rs` | 267 | Word-boundary aware |
| `posts/effects/agentic_extraction.rs` | 1712 | char_indices based |
| `posts/effects/enrichment_tools.rs` | 552 | char_indices based |

**Problem:** Different truncation behavior (word vs char boundary).

#### `hash_content()` - Deprecated but Called

| Caller | File:Line |
|--------|-----------|
| `build_pages.rs` | Lines 26, 56, 79 |

**Replacement:** `extraction::CachedPage::hash_content()`

#### Two `enrich_post*` Functions

| Function | Line | Notes |
|----------|------|-------|
| `enrich_post()` | 452 | Uses EnrichmentContext |
| `enrich_post_with_tools()` | 784 | Marked "LEGACY" in comments |

**Problem:** Unclear which to use, potential drift.

#### Three Parallel Extraction Systems

| System | Location | Status |
|--------|----------|--------|
| Legacy `summarize_pages()` | `crawling/effects/extraction/summarize.rs` | Deprecated |
| Agentic extraction | `posts/effects/agentic_extraction.rs` | Active |
| Extraction library | `extraction::actions::*` | New preferred |

### 2. Deprecated Code Still in Use

#### Functions (Still Exported/Called)

| Function | File | Replacement | Still Called By |
|----------|------|-------------|-----------------|
| `crawl_website()` | `crawling/actions/mod.rs:64` | `ingest_website()` | scheduled_tasks.rs, handlers.rs |
| `summarize_page()` | `crawling/effects/extraction/summarize.rs:46` | Extraction library | handlers.rs |
| `summarize_pages()` | `crawling/effects/extraction/summarize.rs:86` | Extraction library | handlers.rs |
| `hash_content()` | `crawling/effects/extraction/summarize.rs:129` | `extraction::CachedPage::hash_content()` | build_pages.rs |

#### Models (Still in Codebase)

| Model | File | Deprecation Note | Replacement |
|-------|------|------------------|-------------|
| `PageSnapshot` | `crawling/models/page_snapshot.rs:39` | `TODO(migration): Remove` | `extraction::CachedPage` |
| `WebsiteSnapshot` | `crawling/models/website_snapshot.rs:32` | `TODO(migration): Remove` | Removed (junction not needed) |
| `PageSummary` | `crawling/models/page_summary.rs` | Implicit | `extraction::Summary` |

#### Traits/Clients (Replaced by Extraction Library)

| Old | Replacement |
|-----|-------------|
| `BaseWebScraper` | `extraction::Ingestor` |
| `SimpleScraper` | `extraction::HttpIngestor` |
| `FallbackScraper` | `ValidatedIngestor + FirecrawlIngestor` |
| `FirecrawlClient` | `extraction::FirecrawlIngestor` |
| `TavilyClient` | `extraction::TavilyWebSearcher` |
| `NoopSearchService` | `extraction::MockWebSearcher` |

### 3. Pattern Violations

#### Effect Handlers >50 Lines

| Handler | File:Lines | Actual Lines | Should Be |
|---------|------------|--------------|-----------|
| `handle_extract_from_pages()` | `handlers.rs:32-140` | ~108 | <50 |

**Fix:** Extract business logic to `actions/extract_posts_from_pages.rs`

#### Business Logic in Effects

`agentic_extraction.rs` lives in `effects/` but contains action-style functions (noted in file comments lines 27-35). These should be in `actions/`.

#### Large File Needing Split

`posts/effects/agentic_extraction.rs` is **1786 lines**. The file's own TODO (lines 12-26) suggests splitting into:

| Module | Lines | Content |
|--------|-------|---------|
| `types.rs` | 32-153 | Data structures |
| `tools.rs` | 154-352 | Tool definitions & prompts |
| `extraction.rs` | 353-705 | Candidate extraction & enrichment |
| `tool_loop.rs` | 706-982 | Tool execution loop |
| `merging.rs` | 983-1145 | Post merging & dedup |
| `pipeline.rs` | 1146-1400 | extract_from_page/website |
| `storage.rs` | 1401-1573 | Storage & sync |
| `conversions.rs` | 1580-1670 | Type conversions |

### 4. Type Inconsistencies

#### Multiple ContactInfo Definitions

| Location | Fields | GraphQL |
|----------|--------|---------|
| `common/extraction_types.rs` | phone, email, website, intake_form_url, contact_name, other | No |
| `posts/data/types.rs:141` | phone, email, website | Yes |
| `organization/data/organization.rs:23` | phone, email, website | Yes |

**Problem:** GraphQL types have fewer fields than unified type. Changing would be breaking API change.

### 5. Event Flow Gaps

#### WebsiteIngested Has No Handler

`ingest_website()` emits `CrawlEvent::WebsiteIngested` but **no handler exists**.

**Impact:** New ingestion path is a dead end - pages stored but no extraction triggered.

#### Scheduled Tasks Use Deprecated Path

`scheduled_tasks.rs` triggers `crawl_website()` via events.

**Impact:** Production scheduled scraping will break when deprecated code removed.

---

## Proposed Solution

### Phase 0: Pre-Audit Preparation (Non-Breaking)

**Goal:** Establish baseline and unblock new path

1. [ ] Document production data volumes in deprecated tables
2. [ ] Audit client usage of `crawlWebsite` GraphQL mutation
3. [ ] **Create `WebsiteIngested` event handler** (critical - unblocks new path)
4. [ ] Add feature flag for gradual migration

### Phase 1: Code Consolidation (Safe Refactors)

**Goal:** Reduce duplication without changing behavior

#### 1.1 Consolidate `truncate_content()`

```rust
// common/utils.rs
pub fn truncate_content(content: &str, max_chars: usize) -> &str {
    if content.len() <= max_chars {
        content
    } else {
        content
            .char_indices()
            .take_while(|(i, _)| *i < max_chars)
            .last()
            .map(|(i, c)| &content[..i + c.len_utf8()])
            .unwrap_or(content)
    }
}
```

- [ ] Create `common/utils.rs` with single implementation
- [ ] Update imports in `agentic_extraction.rs`
- [ ] Update imports in `enrichment_tools.rs`
- [ ] Remove deprecated version in `summarize.rs`

#### 1.2 Consolidate `enrich_post*` Functions

- [ ] Determine canonical version (likely `enrich_post_with_tools`)
- [ ] Deprecate the other with `#[deprecated]` attribute
- [ ] Update callers to use single function

#### 1.3 Create Type Conversions (Not Breaking Changes)

```rust
// common/extraction_types.rs
impl From<ContactInfo> for posts::data::ContactInfo {
    fn from(c: ContactInfo) -> Self {
        Self {
            phone: c.phone,
            email: c.email,
            website: c.website,
        }
    }
}
```

- [ ] Add `From` impls between unified and GraphQL types
- [ ] Keep GraphQL types stable (no breaking changes)

### Phase 2: Split Large Files

**Goal:** Improve maintainability following existing TODO

#### 2.1 Split `agentic_extraction.rs`

Create directory structure:
```
posts/effects/agentic_extraction/
├── mod.rs           # Re-exports
├── types.rs         # PostCandidate, EnrichedPost, etc.
├── prompts.rs       # CANDIDATE_EXTRACTION_PROMPT, etc.
├── candidates.rs    # extract_candidates()
├── enrichment.rs    # enrich_post*, EnrichmentContext
├── merging.rs       # merge_posts()
├── pipeline.rs      # extract_from_page, extract_from_website
├── storage.rs       # store_extraction, sync_enriched_posts
└── conversions.rs   # to_extracted_post, etc.
```

- [ ] Create `agentic_extraction/` directory
- [ ] Move types to `types.rs`
- [ ] Move prompts to `prompts.rs`
- [ ] Move extraction functions to `candidates.rs`
- [ ] Move enrichment to `enrichment.rs`
- [ ] Move merging to `merging.rs`
- [ ] Move pipeline to `pipeline.rs`
- [ ] Move storage to `storage.rs`
- [ ] Move conversions to `conversions.rs`
- [ ] Create `mod.rs` with re-exports
- [ ] Update all imports

#### 2.2 Extract Handler Logic to Action

```rust
// crawling/actions/extract_posts_from_pages.rs
pub async fn extract_posts_from_pages(
    website_id: WebsiteId,
    pages: Vec<PageToSummarize>,
    job_id: JobId,
    deps: &ServerDeps,
) -> Result<ExtractionResult> {
    // Business logic moved here
}
```

- [ ] Create `crawling/actions/extract_posts_from_pages.rs`
- [ ] Move logic from `handle_extract_from_pages`
- [ ] Make handler <50 lines (auth check → action call → emit event)

### Phase 3: Migration Completion

**Goal:** Complete extraction library migration

#### 3.1 Update Scheduled Tasks

- [ ] Replace `crawl_website` calls with `ingest_website` in scheduled_tasks.rs
- [ ] Test scheduled scraping with new path

#### 3.2 Bridge GraphQL Mutation

```rust
// Make crawlWebsite call ingestWebsite internally
#[graphql(deprecation = "Use ingestWebsite instead")]
async fn crawl_website(&self, ...) -> Result<...> {
    // Call new path internally
    self.ingest_website(...).await
}
```

- [ ] Update `crawlWebsite` to call `ingestWebsite` internally
- [ ] Add deprecation warning to GraphQL response

#### 3.3 Update Tests

- [ ] Update `crawler_tests.rs` to use new types
- [ ] Update `scraping_integration_tests.rs` to use extraction_pages
- [ ] Add tests for `ingest_website` flow
- [ ] Add tests for `WebsiteIngested` handler

### Phase 4: Cleanup (Breaking Changes)

**Goal:** Remove deprecated code after verification

#### 4.1 Remove Deprecated Functions

- [ ] Remove `crawl_website()` from `crawling/actions/mod.rs`
- [ ] Remove `summarize_page()`, `summarize_pages()` from `summarize.rs`
- [ ] Remove `hash_content()` from `summarize.rs`
- [ ] Remove entire `crawling/effects/extraction/summarize.rs` if empty

#### 4.2 Remove Deprecated Models

- [ ] Remove `PageSnapshot` model (after verifying no usage)
- [ ] Remove `WebsiteSnapshot` model
- [ ] Remove `PageSummary` model

#### 4.3 Remove Deprecated Traits/Clients

Files to delete:
- [ ] `kernel/simple_scraper.rs` (if exists)
- [ ] `kernel/fallback_scraper.rs` (if exists)
- [ ] `kernel/firecrawl_client.rs` (if exists)
- [ ] `kernel/tavily_client.rs` (if exists)

#### 4.4 Drop Deprecated Tables (Separate Migration)

```sql
-- migrations/000098_drop_legacy_crawler_tables.sql
-- Only after data migration is verified

DROP TABLE IF EXISTS page_summaries;
DROP TABLE IF EXISTS page_snapshots;
DROP TABLE IF EXISTS website_snapshots;
```

- [ ] Verify all data migrated to extraction_pages
- [ ] Create migration to drop tables
- [ ] Test rollback capability

---

## Acceptance Criteria

### Functional Requirements

- [ ] All extraction flows work via new path (extraction library)
- [ ] Scheduled scraping continues to work
- [ ] GraphQL mutations work (deprecated mutations call new code internally)
- [ ] Admin UI functions correctly

### Non-Functional Requirements

- [ ] All effect handlers <50 lines
- [ ] No duplicate function implementations
- [ ] Single source of truth for extraction types
- [ ] All deprecated functions removed or wrapped with #[deprecated]

### Quality Gates

- [ ] All existing tests pass
- [ ] New tests cover `ingest_website` flow
- [ ] No `TODO(migration)` comments remain
- [ ] No calls to deprecated functions (except through wrappers)

---

## Risk Analysis & Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking scheduled scraping | High | High | Test thoroughly before removing old path |
| GraphQL schema changes | Medium | High | Keep deprecated mutations as wrappers |
| Data loss from table drops | Medium | Critical | Verify migration, keep reversible |
| Test failures | High | Medium | Update tests incrementally |
| Client breakage | Low | Medium | Coordinate deprecation timeline |

---

## Success Metrics

| Metric | Before | After |
|--------|--------|-------|
| Duplicate `truncate_content` | 3 | 1 |
| Deprecated functions called | 8+ | 0 |
| Effect handlers >50 lines | 1+ | 0 |
| `agentic_extraction.rs` lines | 1786 | ~200 (mod.rs) |
| ContactInfo definitions | 3+ | 1 unified + GraphQL wrappers |
| `TODO(migration)` comments | 4+ | 0 |

---

## Files to Modify

### Phase 1 (Consolidation)

| File | Change |
|------|--------|
| `common/utils.rs` | NEW - add `truncate_content()` |
| `common/mod.rs` | Export utils module |
| `posts/effects/agentic_extraction.rs` | Update import |
| `posts/effects/enrichment_tools.rs` | Update import |
| `crawling/effects/extraction/summarize.rs` | Remove duplicate |
| `common/extraction_types.rs` | Add From impls |

### Phase 2 (Split)

| File | Change |
|------|--------|
| `posts/effects/agentic_extraction/` | NEW directory |
| `posts/effects/agentic_extraction/mod.rs` | NEW - re-exports |
| `posts/effects/agentic_extraction/types.rs` | NEW - data types |
| `posts/effects/agentic_extraction/prompts.rs` | NEW - prompts |
| `posts/effects/agentic_extraction/candidates.rs` | NEW - extraction |
| `posts/effects/agentic_extraction/enrichment.rs` | NEW - enrichment |
| `posts/effects/agentic_extraction/merging.rs` | NEW - merge logic |
| `posts/effects/agentic_extraction/pipeline.rs` | NEW - pipeline |
| `posts/effects/agentic_extraction/storage.rs` | NEW - storage |
| `posts/effects/agentic_extraction/conversions.rs` | NEW - conversions |
| `crawling/actions/extract_posts_from_pages.rs` | NEW - action |
| `crawling/effects/handlers.rs` | Slim down handler |

### Phase 3 (Migration)

| File | Change |
|------|--------|
| `kernel/scheduled_tasks.rs` | Use new path |
| `server/graphql/mutations/crawling.rs` | Bridge mutation |
| Tests (multiple) | Update to new types |

### Phase 4 (Cleanup)

| File | Change |
|------|--------|
| `crawling/actions/mod.rs` | Remove `crawl_website` |
| `crawling/effects/extraction/summarize.rs` | DELETE |
| `crawling/models/page_snapshot.rs` | DELETE |
| `crawling/models/website_snapshot.rs` | DELETE |
| `crawling/models/page_summary.rs` | DELETE |
| `kernel/simple_scraper.rs` | DELETE (if exists) |
| `kernel/fallback_scraper.rs` | DELETE (if exists) |
| `kernel/firecrawl_client.rs` | DELETE (if exists) |
| `kernel/tavily_client.rs` | DELETE (if exists) |

---

## References

### Internal References

- Migration tracker: `packages/server/src/domains/crawling/MIGRATION.md`
- Unified types: `packages/server/src/common/extraction_types.rs`
- Seesaw architecture: `CLAUDE.md` (Seesaw Architecture Rules section)
- Extraction library: `packages/extraction/`

### Related Plans

- `docs/plans/2026-02-03-consolidate-crawling-to-extraction-library-plan.md`
- `docs/plans/2026-02-03-refactor-extraction-system-alignment-plan.md`
