---
title: "refactor: Split Crawl Pipeline into Independent Jobs"
type: refactor
date: 2026-02-04
---

# Split Crawl Pipeline into Independent Jobs

## Overview

Refactor the monolithic crawl pipeline into three independent, retriable jobs that chain via Seesaw events. This enables independent retries, better timeout handling, and clearer progress visibility.

## Problem Statement

The current crawl pipeline runs everything synchronously in a single job:

1. **Ingest Website** → Discover URLs, fetch pages, store in extraction_pages
2. **Extract Posts** → Three-pass LLM extraction (narrative → dedupe → investigate)
3. **Sync Posts** → Write to database (simple delete/replace or LLM diff)

Problems with this approach:
- If step 2 fails after step 1 succeeds, must re-ingest the entire website
- No progress visibility between stages
- Single timeout covers all operations (extraction can take 10+ minutes)
- Sophisticated job infrastructure (retries, DLQ, leases) is underutilized

## Proposed Solution

Split into three independent jobs that chain via Seesaw events:

```
IngestWebsiteJob ──event──► ExtractPostsJob ──event──► SyncPostsJob
     (existing)                  (new)                    (new)
```

### Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Job chaining | Seesaw events | Existing pattern, keeps architecture consistent |
| Extracted posts storage | JSONB in job args | Temporary storage, acceptable for inter-job data |
| Sync strategy | Single job with flag | `use_llm_sync: bool` - simpler than two job types |
| Progress tracking | Job-level only | Status per job, no sub-job granularity needed |

## Technical Approach

### New Job Types

#### ExtractPostsJob

```rust
// domains/crawling/jobs/extract_posts.rs
pub struct ExtractPostsJob {
    pub website_id: Uuid,
    pub parent_job_id: Option<Uuid>,  // For tracking job chains
}

impl CommandMeta for ExtractPostsJob {
    const JOB_TYPE: &'static str = "extract_posts";
    const MAX_RETRIES: i32 = 3;
    const PRIORITY: JobPriority = JobPriority::Normal;
}
```

#### SyncPostsJob

```rust
// domains/crawling/jobs/sync_posts.rs
pub struct SyncPostsJob {
    pub website_id: Uuid,
    pub extracted_posts: Vec<ExtractedPost>,  // JSONB serialized
    pub use_llm_sync: bool,
    pub parent_job_id: Option<Uuid>,
}

impl CommandMeta for SyncPostsJob {
    const JOB_TYPE: &'static str = "sync_posts";
    const MAX_RETRIES: i32 = 3;
    const PRIORITY: JobPriority = JobPriority::Normal;
}
```

### Modified Event Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ BEFORE (Synchronous Cascade)                                                │
├─────────────────────────────────────────────────────────────────────────────┤
│ WebsiteIngested → handle_extract_posts_from_pages() → PostsExtractedFromPages
│                   (runs extraction inline)                                  │
│                                                                             │
│ PostsExtractedFromPages → handle_sync_crawled_posts() → PostsSynced         │
│                           (runs sync inline)                                │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│ AFTER (Job-Based Cascade)                                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│ WebsiteIngested → handle_enqueue_extract_posts() → (enqueues ExtractPostsJob)
│                   (thin handler, just enqueues)                             │
│                                                                             │
│ ExtractPostsJob executes → emits PostsExtractedFromPages                    │
│                                                                             │
│ PostsExtractedFromPages → handle_enqueue_sync_posts() → (enqueues SyncPostsJob)
│                           (thin handler, just enqueues)                     │
│                                                                             │
│ SyncPostsJob executes → emits PostsSynced                                   │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Event Changes

The `PostsExtractedFromPages` event needs to carry extracted posts for the sync job:

```rust
// domains/crawling/events/mod.rs
pub enum CrawlEvent {
    // ... existing variants ...

    PostsExtractedFromPages {
        website_id: WebsiteId,
        job_id: JobId,
        posts: Vec<ExtractedPost>,      // Already exists
        page_results: Vec<PageExtractionResult>,
        // NEW: Add parent job reference for chaining
        extract_job_id: Option<Uuid>,
    },
}
```

## Acceptance Criteria

### Functional Requirements

- [ ] `IngestWebsiteJob` emits `WebsiteIngested` and completes (existing behavior)
- [ ] `WebsiteIngested` event triggers enqueueing of `ExtractPostsJob`
- [ ] `ExtractPostsJob` runs three-pass extraction and emits `PostsExtractedFromPages`
- [ ] `PostsExtractedFromPages` event triggers enqueueing of `SyncPostsJob`
- [ ] `SyncPostsJob` runs sync (simple or LLM) and emits `PostsSynced`
- [ ] Each job can be retried independently without re-running previous stages
- [ ] Job status visible in admin UI (pending → running → succeeded/failed)

### Non-Functional Requirements

- [ ] `ExtractPostsJob` timeout: 10 minutes (accounts for agentic investigation)
- [ ] `SyncPostsJob` timeout: 5 minutes (LLM sync can be slow)
- [ ] Jobs respect existing retry logic (exponential backoff, max 3 retries)
- [ ] Failed jobs go to dead letter queue for manual inspection

## Implementation Plan

### Phase 1: Create New Job Types

**Files to create:**
- `domains/crawling/jobs/extract_posts.rs`
- `domains/crawling/jobs/sync_posts.rs`

**Files to modify:**
- `domains/crawling/jobs/mod.rs` - Add exports

### Phase 2: Create Job Executors

**Files to create:**
- `domains/crawling/jobs/executor.rs` - Add `execute_extract_posts_job()` and `execute_sync_posts_job()`

Move extraction logic from `handlers.rs:handle_extract_posts_from_pages()` into executor.
Move sync logic from `handlers.rs:handle_sync_crawled_posts()` into executor.

### Phase 3: Modify Event Handlers

**Files to modify:**
- `domains/crawling/effects/handlers.rs`

Change handlers to enqueue jobs instead of running logic:

```rust
// BEFORE
pub async fn handle_extract_posts_from_pages(...) -> Result<()> {
    // ... extraction logic ...
    ctx.emit(CrawlEvent::PostsExtractedFromPages { ... });
}

// AFTER
pub async fn handle_enqueue_extract_posts(...) -> Result<()> {
    let job = ExtractPostsJob { website_id, parent_job_id: Some(job_id.into_uuid()) };
    ctx.deps().job_queue.enqueue(job).await?;
    Ok(())
}
```

### Phase 4: Wire Up Job Execution

**Files to modify:**
- `domains/crawling/effects/crawler.rs` - Update effect to call new handlers

### Phase 5: Add Job Status Queries

**Files to modify:**
- `domains/crawling/jobs/executor.rs` - Add `JobInfo::find_chain_for_website()` to query job chain status

## File Changes Summary

| File | Change |
|------|--------|
| `domains/crawling/jobs/extract_posts.rs` | **CREATE** - ExtractPostsJob struct |
| `domains/crawling/jobs/sync_posts.rs` | **CREATE** - SyncPostsJob struct |
| `domains/crawling/jobs/mod.rs` | MODIFY - Add exports |
| `domains/crawling/jobs/executor.rs` | MODIFY - Add executors for new jobs |
| `domains/crawling/effects/handlers.rs` | MODIFY - Change to thin enqueue handlers |
| `domains/crawling/effects/crawler.rs` | MODIFY - Update event routing |
| `domains/crawling/events/mod.rs` | MODIFY - Add extract_job_id to event |

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| JSONB for extracted posts could grow large | Medium | Posts are temporary; cleared after sync job completes |
| Job chain could orphan if middle job fails | Low | Dead letter queue captures failed jobs; admin can retry |
| Breaking change to event structure | Medium | Add field as `Option<Uuid>`, backward compatible |

## Future Considerations

- **Progress tracking**: Could add `progress_pct` field to jobs table for sub-job visibility
- **Parallel extraction**: Could split extraction batches into separate jobs for parallelism
- **Job dashboard**: Admin UI to view job chains and retry failed stages
