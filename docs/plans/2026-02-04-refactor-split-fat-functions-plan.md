---
title: Split Fat Functions into Single-Responsibility Units
type: refactor
date: 2026-02-04
priority: high
prerequisite_for: event-chaining refactor
---

# Split Fat Functions into Single-Responsibility Units

## Overview

Split oversized functions (>50 lines) into focused, single-responsibility units. This is a **prerequisite** for the event-chaining refactor - functions must be atomized before they can be converted to event-triggered effects.

**Total functions to refactor:** 9
**Total lines affected:** ~1,400
**Estimated reduction in max function size:** 270 → 50 lines

## Why This Matters

1. **Enables event chaining** - Can't emit events between steps if steps are inline
2. **Any step can become a job** - Small functions can be enqueued independently
3. **Testability** - Small functions can be unit tested in isolation
4. **Readability** - Each function fits on one screen

---

## Priority 1: CRITICAL (270 lines)

### `apply_sync_operations` - llm_sync.rs:336

**Current:** 270 lines doing 5 distinct operations

**File:** `packages/server/src/domains/posts/actions/llm_sync.rs`

**Current Structure:**
```rust
pub async fn apply_sync_operations(
    website_id: WebsiteId,
    fresh_posts: &[ExtractedPost],
    existing_posts: &[Post],
    operations: Vec<SyncOperation>,
    pool: &PgPool,
) -> Result<LlmSyncResult> {
    // Build lookup maps (15 lines)
    // For each operation:
    match op {
        SyncOperation::Insert { .. } => { /* 25 lines */ }
        SyncOperation::Update { .. } => { /* 40 lines */ }
        SyncOperation::Delete { .. } => { /* 50 lines */ }
        SyncOperation::Merge { .. } => { /* 90 lines */ }
        SyncOperation::Skip { .. } => { /* 10 lines */ }
    }
    // Summary logging (20 lines)
}
```

**Refactor to:**

```rust
// New file: posts/actions/sync_operations.rs

/// Apply a single INSERT operation
pub async fn apply_insert(
    fresh_id: &str,
    fresh_posts: &HashMap<String, &ExtractedPost>,
    website_id: WebsiteId,
    pool: &PgPool,
) -> Result<SyncOpResult> {
    let post = fresh_posts.get(fresh_id)
        .ok_or_else(|| anyhow!("Fresh post not found: {}", fresh_id))?;

    let inserted = insert_post(post, website_id, pool).await?;
    info!(post_id = %inserted.id, title = %inserted.title, "Inserted new post");

    Ok(SyncOpResult::Inserted(inserted.id))
}

/// Apply a single UPDATE operation
pub async fn apply_update(
    fresh_id: &str,
    existing_id: &str,
    merge_description: bool,
    fresh_posts: &HashMap<String, &ExtractedPost>,
    existing_posts: &HashMap<String, &Post>,
    pool: &PgPool,
) -> Result<SyncOpResult> {
    let fresh = fresh_posts.get(fresh_id)
        .ok_or_else(|| anyhow!("Fresh post not found: {}", fresh_id))?;
    let existing = existing_posts.get(existing_id)
        .ok_or_else(|| anyhow!("Existing post not found: {}", existing_id))?;

    let updated = update_post(existing, fresh, merge_description, pool).await?;
    info!(post_id = %updated.id, "Updated post");

    Ok(SyncOpResult::Updated(updated.id))
}

/// Apply a single DELETE operation
pub async fn apply_delete(
    existing_id: &str,
    reason: &str,
    existing_posts: &HashMap<String, &Post>,
    pool: &PgPool,
) -> Result<SyncOpResult> {
    let existing = existing_posts.get(existing_id)
        .ok_or_else(|| anyhow!("Existing post not found: {}", existing_id))?;

    // Check if already inactive
    if existing.status != "active" {
        info!(post_id = %existing_id, status = %existing.status, "Skipping delete - not active");
        return Ok(SyncOpResult::Skipped);
    }

    Post::soft_delete_with_reason(existing.id, reason, pool).await?;
    info!(post_id = %existing_id, reason = %reason, "Soft deleted post");

    Ok(SyncOpResult::Deleted(existing.id))
}

/// Apply a single MERGE operation (canonical absorbs duplicates)
pub async fn apply_merge(
    canonical_id: &str,
    duplicate_ids: &[String],
    merged_title: Option<&str>,
    merged_description: Option<&str>,
    reason: &str,
    existing_posts: &HashMap<String, &Post>,
    pool: &PgPool,
) -> Result<SyncOpResult> {
    let canonical = existing_posts.get(canonical_id)
        .ok_or_else(|| anyhow!("Canonical post not found: {}", canonical_id))?;

    // Update canonical with merged content if provided
    if merged_title.is_some() || merged_description.is_some() {
        Post::update_content(
            canonical.id,
            merged_title,
            merged_description,
            pool
        ).await?;
    }

    // Soft delete duplicates
    let mut deleted_count = 0;
    for dup_id in duplicate_ids {
        if let Some(dup) = existing_posts.get(dup_id) {
            if dup.status == "active" {
                Post::soft_delete_with_reason(dup.id, reason, pool).await?;
                deleted_count += 1;
            }
        }
    }

    info!(
        canonical_id = %canonical_id,
        duplicates_deleted = deleted_count,
        "Merged posts"
    );

    Ok(SyncOpResult::Merged { canonical: canonical.id, deleted: deleted_count })
}

/// Dispatcher - now thin
pub async fn apply_sync_operations(
    website_id: WebsiteId,
    fresh_posts: &[ExtractedPost],
    existing_posts: &[Post],
    operations: Vec<SyncOperation>,
    pool: &PgPool,
) -> Result<LlmSyncResult> {
    let fresh_map = build_fresh_lookup(fresh_posts);
    let existing_map = build_existing_lookup(existing_posts);

    let mut result = LlmSyncResult::default();

    for op in operations {
        match op {
            SyncOperation::Insert { fresh_id } => {
                apply_insert(&fresh_id, &fresh_map, website_id, pool).await?;
                result.inserted += 1;
            }
            SyncOperation::Update { fresh_id, existing_id, merge_description } => {
                apply_update(&fresh_id, &existing_id, merge_description, &fresh_map, &existing_map, pool).await?;
                result.updated += 1;
            }
            SyncOperation::Delete { existing_id, reason } => {
                apply_delete(&existing_id, &reason, &existing_map, pool).await?;
                result.deleted += 1;
            }
            SyncOperation::Merge { canonical_id, duplicate_ids, merged_title, merged_description, reason } => {
                let res = apply_merge(&canonical_id, &duplicate_ids, merged_title.as_deref(), merged_description.as_deref(), &reason, &existing_map, pool).await?;
                if let SyncOpResult::Merged { deleted, .. } = res {
                    result.merged += deleted;
                }
            }
            SyncOperation::Skip { .. } => {
                result.skipped += 1;
            }
        }
    }

    Ok(result)
}
```

**Files to create/modify:**

| File | Action |
|------|--------|
| `posts/actions/sync_operations.rs` | **NEW** - Individual operation functions |
| `posts/actions/llm_sync.rs` | Simplify `apply_sync_operations` to dispatcher |
| `posts/actions/mod.rs` | Export new module |

**Line count change:** 270 → ~40 (dispatcher) + 4×30 (operations) = ~160 total, better organized

---

## Priority 2: HIGH (200 lines)

### `llm_sync_posts` - llm_sync.rs:132

**Current:** 200 lines mixing data prep, logging, and LLM orchestration

**Problems:**
- 30+ lines of diagnostic logging (lines 148-180)
- 40+ lines of operation logging (lines 250-320)
- Data preparation mixed with orchestration

**Refactor to:**

```rust
// Extract: Data preparation
async fn prepare_sync_inputs(
    fresh_posts: &[ExtractedPost],
    existing_posts: &[Post],
    pool: &PgPool,
) -> Result<(Vec<FreshPostForLlm>, Vec<ExistingPostForLlm>)> {
    // Load contacts, format for LLM
    // ~40 lines
}

// Extract: Diagnostic logging (dev/debug only)
fn log_sync_diagnostics(
    fresh: &[FreshPostForLlm],
    existing: &[ExistingPostForLlm],
) {
    // All the info!() calls for debugging
    // ~30 lines
}

// Extract: Operation result logging
fn log_sync_operations(
    operations: &[SyncOperation],
    fresh: &[FreshPostForLlm],
    existing: &[ExistingPostForLlm],
) {
    // Log each operation decision
    // ~40 lines
}

// Simplified orchestrator
pub async fn llm_sync_posts(
    website_id: WebsiteId,
    fresh_posts: Vec<ExtractedPost>,
    deps: &ServerDeps,
) -> Result<LlmSyncResult> {
    let existing = Post::find_active_by_website(website_id, &deps.db_pool).await?;

    let (fresh_llm, existing_llm) = prepare_sync_inputs(&fresh_posts, &existing, &deps.db_pool).await?;

    #[cfg(debug_assertions)]
    log_sync_diagnostics(&fresh_llm, &existing_llm);

    let prompt = build_sync_prompt(&fresh_llm, &existing_llm);
    let response = deps.ai.complete(&prompt).await?;
    let operations = parse_sync_response(&response)?;

    #[cfg(debug_assertions)]
    log_sync_operations(&operations, &fresh_llm, &existing_llm);

    apply_sync_operations(website_id, &fresh_posts, &existing, operations, &deps.db_pool).await
}
```

**Line count change:** 200 → ~50 (orchestrator) + 3×35 (helpers) = ~155 total

---

## Priority 3: MEDIUM (145-157 lines each)

### 3a. `assess_website` - website_approval/actions/mod.rs:88

**Current:** 157 lines

**Extract:**

```rust
// Extract: Homepage fetching with fallback
async fn fetch_homepage_content(
    domain: &str,
    extraction: &OpenAIExtractionService,
    ingestor: &dyn Ingestor,
) -> Result<Option<String>> {
    // Try extraction service first
    // Fallback to ingestor
    // ~45 lines
}

// Extract: Research freshness check
async fn find_fresh_research(
    website_id: WebsiteId,
    max_age_hours: i64,
    pool: &PgPool,
) -> Option<WebsiteResearch> {
    // ~20 lines
}
```

**Line count change:** 157 → ~80

---

### 3b. `generate_assessment` - website_approval/actions/mod.rs:255

**Current:** 135 lines

**Extract:**

```rust
// Extract: Load all research data
async fn load_research_data(
    research_id: Uuid,
    pool: &PgPool,
) -> Result<ResearchData> {
    // Load homepage, queries, results
    // ~30 lines
}

// Extract: Generate and store embedding (non-fatal)
async fn generate_assessment_embedding(
    assessment_id: Uuid,
    markdown: &str,
    deps: &ServerDeps,
) -> Result<()> {
    // ~25 lines
}
```

**Line count change:** 135 → ~70

---

### 3c. `conduct_searches` - website_approval/actions/mod.rs:395

**Current:** 110 lines

**Extract:**

```rust
// Extract: Execute single search and store
async fn execute_and_store_search(
    research_id: Uuid,
    query: &str,
    searcher: &dyn WebSearcher,
    pool: &PgPool,
) -> Result<SearchResult> {
    // Execute search
    // Store query record
    // Store results
    // ~40 lines
}
```

**Line count change:** 110 → ~60

---

### 3d. `extract_posts_from_pages` - post_extraction.rs:551

**Current:** 145 lines with code duplication

**Extract:**

```rust
// Extract: Shared enrichment logic (duplicated in extract_posts_from_content)
async fn enrich_posts_with_investigation(
    narratives: Vec<NarrativePost>,
    deps: &ServerDeps,
) -> Vec<ExtractedPost> {
    // Run investigation in parallel
    // Combine results
    // ~45 lines (currently duplicated)
}
```

**Line count change:** 145 → ~100 (and removes duplication)

---

## Priority 4: MEDIUM-LOW (91-102 lines)

### 4a. `execute_extract_posts_job` - executor.rs:234

**Current:** 102 lines

**Extract:**

```rust
// Extract: Validate inputs and load dependencies
async fn prepare_extract_job(
    website_id: WebsiteId,
    deps: &ServerDeps,
) -> Result<(Website, &OpenAIExtractionService)> {
    // ~25 lines
}

// Extract: Build result from extraction
fn build_extract_result(
    job_id: Uuid,
    extraction: &DomainExtractionResult,
) -> JobExecutionResult {
    // ~20 lines
}
```

**Line count change:** 102 → ~55

---

### 4b. `execute_sync_posts_job` - executor.rs:343

**Current:** 91 lines

**Extract:**

```rust
// Extract: Choose and run sync strategy
async fn run_sync_strategy(
    website_id: WebsiteId,
    posts: Vec<ExtractedPost>,
    use_llm: bool,
    deps: &ServerDeps,
) -> Result<SyncResult> {
    // ~35 lines
}
```

**Line count change:** 91 → ~50

---

## Summary

### Files to Create

| New File | Purpose | Est. Lines |
|----------|---------|------------|
| `posts/actions/sync_operations.rs` | Individual sync operations | ~120 |

### Files to Modify

| File | Functions to Extract | Current → After |
|------|---------------------|-----------------|
| `posts/actions/llm_sync.rs` | `prepare_sync_inputs`, `log_sync_diagnostics`, `log_sync_operations` | 470 → ~300 |
| `website_approval/actions/mod.rs` | `fetch_homepage_content`, `find_fresh_research`, `load_research_data`, `generate_assessment_embedding`, `execute_and_store_search` | 661 → ~450 |
| `crawling/actions/post_extraction.rs` | `enrich_posts_with_investigation` | 695 → ~650 |
| `crawling/jobs/executor.rs` | `prepare_extract_job`, `build_extract_result`, `run_sync_strategy` | 495 → ~400 |

### Max Function Size

| Metric | Before | After |
|--------|--------|-------|
| Largest function | 270 lines | ~50 lines |
| Functions >100 lines | 7 | 0 |
| Functions >50 lines | 9 | ~2 |

---

## Implementation Order

### Phase 1: Critical (apply_sync_operations)
1. Create `posts/actions/sync_operations.rs`
2. Extract `apply_insert`, `apply_update`, `apply_delete`, `apply_merge`
3. Simplify dispatcher in `llm_sync.rs`
4. Test sync operations independently

### Phase 2: High (llm_sync_posts)
1. Extract `prepare_sync_inputs`
2. Extract logging helpers (conditional compilation)
3. Simplify orchestrator
4. Test

### Phase 3: Medium (website_approval + post_extraction)
1. Extract website_approval helpers
2. Extract shared `enrich_posts_with_investigation`
3. Test

### Phase 4: Medium-Low (executor.rs)
1. Extract job preparation helpers
2. Simplify job executors
3. Test

---

## Acceptance Criteria

- [ ] No function exceeds 50 lines
- [ ] Each function has single responsibility
- [ ] All existing tests pass
- [ ] `cargo check --package server` compiles
- [ ] Functions are independently testable

---

## Relationship to Event Chaining

This refactor is a **prerequisite** for the event-chaining refactor:

```
BEFORE (can't event-chain):
┌─────────────────────────────────────────┐
│ fat_function() {                        │
│   step1();  // Can't emit event here    │
│   step2();  // Can't make this a job    │
│   step3();  // All inline               │
│ }                                       │
└─────────────────────────────────────────┘

AFTER (ready for event-chaining):
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│ step1()      │ ──► │ step2()      │ ──► │ step3()      │
│ returns data │     │ returns data │     │ returns data │
└──────────────┘     └──────────────┘     └──────────────┘
       │                    │                    │
       ▼                    ▼                    ▼
   Event₁               Event₂               Event₃
```

Once functions are split, the event-chaining refactor can wrap each in an effect that emits events.

---

## References

- Event chaining plan: `docs/plans/2026-02-04-refactor-crawling-cascade-event-chaining-plan.md`
- CLAUDE.md rule: "Effects Must Be Ultra-Thin (<50 lines)"
- Fat function analysis: Deep analysis performed 2026-02-04
