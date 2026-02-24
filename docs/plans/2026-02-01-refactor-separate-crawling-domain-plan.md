---
title: Separate Crawling from Posts Domain
type: refactor
date: 2026-02-01
---

# Separate Crawling from Posts Domain

## Overview

The `posts` domain has grown to encompass two distinct responsibilities that should be separate:
1. **Crawling**: Multi-page website discovery, page fetching, and content caching
2. **Post extraction**: Transforming cached content into structured posts

This creates leaky abstractions where the `posts` domain directly manipulates `Website` entities (crawl status, page counts) and contains 1600+ lines of crawling logic that has nothing to do with posts themselves.

## Problem Statement

### Current Architecture Issues

**1. Mixed Responsibilities in PostCommand**

The `PostCommand` enum (425 lines) contains two completely different concerns:

```
Post lifecycle commands:          Crawling commands:
- CreatePostEntry                 - CrawlWebsite
- UpdatePostStatus                - ExtractPostsFromPages
- CreatePost                      - RetryWebsiteCrawl
- DeletePost                      - MarkWebsiteNoPosts
- CreateReport                    - SyncCrawledPosts
- ResolveReport                   - RegeneratePosts
                                  - RegeneratePageSummaries
```

**2. Posts Domain Directly Manipulates Website Entities**

`crawler.rs` makes 15+ direct calls to Website model methods:
- `Website::find_by_id()`
- `Website::start_crawl()`
- `Website::complete_crawl()`
- `Website::update_pages_crawled()`
- `Website::fail_crawl()`

This violates domain boundaries - posts shouldn't know how to manage crawl state.

**3. Confusing `scraping` Domain Facade**

The `scraping` domain (`domains/scraping/models/mod.rs`) is just a re-export layer:

```rust
// Re-export website models from the website domain for backward compatibility
pub use crate::domains::website::models::{
    CrawlStatus, Website, WebsiteAssessment, WebsiteResearch, ...
};
```

This creates import path confusion - some code imports from `scraping`, some from `website`.

**4. Model Ownership Split**

| Model | Currently Lives In | Should Be In |
|-------|-------------------|--------------|
| `Website` | `website/models/` | `website/` (correct) |
| `WebsiteSnapshot` | `website/models/` | `crawling/` (new) |
| `PageSnapshot` | `scraping/models/` | `crawling/` (new) |
| `PageSummary` | `scraping/models/` | `crawling/` (new) |

**5. `CrawlerEffect` is 1600+ Lines**

This massive effect in `posts/effects/crawler.rs` handles:
- Firecrawl API calls
- Link prioritization (HIGH_PRIORITY_KEYWORDS, SKIP_KEYWORDS)
- Page snapshot storage
- AI summarization
- Post synthesis
- Retry logic

This should be split across domains with clear boundaries.

## Proposed Solution

### New Domain Structure

```
domains/
├── website/                    # Website entity management (approval, status)
│   ├── models/
│   │   └── website.rs          # Website entity (KEEP, but remove crawl methods)
│   ├── commands/
│   ├── events/
│   └── effects/
│
├── crawling/                   # NEW: Page discovery and caching
│   ├── models/
│   │   ├── page_snapshot.rs    # MOVE from scraping
│   │   ├── page_summary.rs     # MOVE from scraping
│   │   └── website_snapshot.rs # MOVE from website
│   ├── commands/
│   │   └── mod.rs              # CrawlCommand enum
│   ├── events/
│   │   └── mod.rs              # CrawlEvent enum
│   ├── effects/
│   │   └── crawler.rs          # Crawling logic (MOVE from posts)
│   └── machines/
│       └── mod.rs              # Crawl workflow state machine
│
├── posts/                      # Post lifecycle ONLY
│   ├── models/
│   │   ├── post.rs             # Keep
│   │   └── post_report.rs      # Keep
│   ├── commands/
│   │   └── mod.rs              # PostCommand (remove crawling commands)
│   ├── events/
│   │   └── mod.rs              # PostEvent (remove crawling events)
│   ├── effects/
│   │   ├── ai.rs               # AI extraction
│   │   ├── scraper.rs          # Single-page scraping
│   │   ├── sync.rs             # Post sync
│   │   └── post.rs             # Post lifecycle
│   └── machines/
│
└── scraping/                   # DELETE - was just a facade
```

### Domain Communication Pattern

Domains communicate via events, not direct model calls:

```
┌─────────────┐     CrawlComplete     ┌──────────────┐
│  crawling   │ ──────────────────▶   │    posts     │
│   domain    │     (event bus)       │    domain    │
└─────────────┘                       └──────────────┘
       │                                     │
       │ owns                                │ owns
       ▼                                     ▼
  PageSnapshot                             Post
  PageSummary                           PostReport
  WebsiteSnapshot
```

### New `CrawlCommand` Enum

```rust
// domains/crawling/commands/mod.rs
pub enum CrawlCommand {
    /// Start crawling a website
    StartCrawl {
        website_id: WebsiteId,
        job_id: JobId,
        requested_by: MemberId,
        max_depth: i32,
        max_pages: i32,
    },

    /// Continue crawling with discovered links
    CrawlPage {
        website_id: WebsiteId,
        job_id: JobId,
        url: String,
        depth: i32,
    },

    /// Summarize crawled pages using AI
    SummarizePages {
        website_id: WebsiteId,
        job_id: JobId,
        page_ids: Vec<PageSnapshotId>,
    },

    /// Mark crawl as complete
    CompleteCrawl {
        website_id: WebsiteId,
        job_id: JobId,
        pages_crawled: i32,
    },

    /// Retry failed crawl
    RetryCrawl {
        website_id: WebsiteId,
        job_id: JobId,
    },

    /// Mark crawl as failed (terminal)
    FailCrawl {
        website_id: WebsiteId,
        job_id: JobId,
        reason: String,
    },
}
```

### New `CrawlEvent` Enum

```rust
// domains/crawling/events/mod.rs
pub enum CrawlEvent {
    /// Crawl started for a website
    CrawlStarted {
        website_id: WebsiteId,
        job_id: JobId,
    },

    /// Page successfully crawled and cached
    PageCrawled {
        website_id: WebsiteId,
        job_id: JobId,
        page_snapshot_id: PageSnapshotId,
        url: String,
        is_new: bool,
    },

    /// Pages summarized by AI
    PagesSummarized {
        website_id: WebsiteId,
        job_id: JobId,
        summary_ids: Vec<PageSummaryId>,
    },

    /// Crawl completed successfully
    CrawlCompleted {
        website_id: WebsiteId,
        job_id: JobId,
        pages_crawled: i32,
        new_pages: i32,
    },

    /// Crawl failed (will retry)
    CrawlRetrying {
        website_id: WebsiteId,
        job_id: JobId,
        attempt: i32,
        reason: String,
    },

    /// Crawl failed terminally
    CrawlFailed {
        website_id: WebsiteId,
        job_id: JobId,
        reason: String,
    },
}
```

### Simplified `PostCommand` Enum (after refactor)

```rust
// domains/posts/commands/mod.rs - AFTER refactor
pub enum PostCommand {
    // =========================================================================
    // Post Lifecycle Commands ONLY
    // =========================================================================

    /// Create a new post entry
    CreatePostEntry { ... },

    /// Update post status
    UpdatePostStatus { ... },

    /// Approve and publish post
    CreatePost { ... },

    /// Create custom post (admin)
    CreateCustomPost { ... },

    /// Repost existing post
    RepostPost { ... },

    /// Expire post
    ExpirePost { ... },

    /// Archive post
    ArchivePost { ... },

    /// Delete post
    DeletePost { ... },

    /// Analytics
    IncrementPostView { ... },
    IncrementPostClick { ... },

    // =========================================================================
    // Extraction Commands (triggered by crawling events)
    // =========================================================================

    /// Extract posts from crawled pages (triggered by CrawlCompleted event)
    ExtractPostsFromCrawl {
        website_id: WebsiteId,
        job_id: JobId,
        page_snapshot_ids: Vec<PageSnapshotId>,
    },

    /// Sync extracted posts to database
    SyncPosts { ... },

    // =========================================================================
    // Report Commands
    // =========================================================================
    CreateReport { ... },
    ResolveReport { ... },
    DismissReport { ... },

    // =========================================================================
    // Deduplication
    // =========================================================================
    DeduplicatePosts { ... },
}
```

## Technical Considerations

### Migration Strategy

**Phase 1: Create `crawling` domain structure**
- Create `domains/crawling/` with commands, events, effects, models directories
- Define `CrawlCommand` and `CrawlEvent` enums
- Create `CrawlerEffect` in new location

**Phase 2: Move models**
- Move `PageSnapshot` from `scraping/models/` to `crawling/models/`
- Move `PageSummary` from `scraping/models/` to `crawling/models/`
- Move `WebsiteSnapshot` from `website/models/` to `crawling/models/`
- Update all imports

**Phase 3: Extract crawling logic**
- Move handler functions from `posts/effects/crawler.rs` to `crawling/effects/crawler.rs`
- Update to use new `CrawlCommand` and `CrawlEvent`
- Remove crawling commands from `PostCommand`

**Phase 4: Add event-driven communication**
- Posts domain listens to `CrawlCompleted` events
- When crawl completes, posts domain triggers extraction

**Phase 5: Cleanup**
- Delete `domains/scraping/` (was just a facade)
- Remove crawl methods from `Website` model
- Update all imports across codebase

### Affected Files

| File | Action |
|------|--------|
| `domains/posts/effects/crawler.rs` | MOVE to `domains/crawling/effects/crawler.rs` |
| `domains/posts/effects/composite.rs` | REMOVE routing to CrawlerEffect |
| `domains/posts/commands/mod.rs` | REMOVE crawling commands |
| `domains/posts/events/mod.rs` | REMOVE crawling events |
| `domains/scraping/models/page_snapshot.rs` | MOVE to `domains/crawling/models/` |
| `domains/scraping/models/page_summary.rs` | MOVE to `domains/crawling/models/` |
| `domains/scraping/mod.rs` | DELETE entire domain |
| `domains/website/models/website.rs` | REMOVE crawl state methods |
| `domains/website/models/website_snapshot.rs` | MOVE to `domains/crawling/models/` |

### Import Updates Required

All files importing from `crate::domains::scraping::models` need updating:
- `domains/posts/effects/crawler.rs`
- `domains/posts/effects/scraper.rs`
- `domains/posts/effects/syncing.rs`
- `domains/domain_approval/effects/research.rs`

### Database Schema Impact

**No schema changes required.** Tables remain the same:
- `page_snapshots`
- `page_summaries`
- `website_snapshots`
- `websites`

Only Rust code organization changes.

## Acceptance Criteria

### Functional Requirements

- [ ] New `crawling` domain exists with proper Seesaw structure
- [ ] `CrawlCommand` enum defines all crawling operations
- [ ] `CrawlEvent` enum defines all crawling facts
- [ ] `CrawlerEffect` lives in `crawling/effects/`
- [ ] `posts` domain no longer contains crawling logic
- [ ] `scraping` domain is deleted
- [ ] Posts domain listens to crawl events for extraction trigger
- [ ] All existing functionality works unchanged

### Non-Functional Requirements

- [ ] No performance regression
- [ ] All tests pass
- [ ] Import paths are consistent (no facade layers)
- [ ] CLAUDE.md patterns followed (thin effects, no query_as! macro)

### Quality Gates

- [ ] `cargo build` succeeds
- [ ] `cargo test` passes
- [ ] `cargo clippy` has no new warnings
- [ ] No circular dependencies between domains

## Success Metrics

- `posts/effects/crawler.rs` deleted (0 lines in posts domain for crawling)
- `PostCommand` enum reduced from ~30 variants to ~20
- Clear domain boundaries: crawling knows nothing about posts
- Single source of truth for page/snapshot models

## Dependencies & Prerequisites

- Understand Seesaw event bus for cross-domain communication
- Existing architecture documentation in `docs/architecture/`

## Risk Analysis & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Broken imports | Build failures | Run `cargo build` after each file move |
| Circular dependencies | Build failures | Use events for cross-domain communication |
| Lost functionality | Runtime errors | Keep existing tests passing |
| Performance regression | Slower crawls | Profile before/after |

## Future Considerations

- Consider adding a `CrawlMachine` for complex crawl workflow states
- Event-driven architecture enables future async/parallel crawling
- Clean boundaries enable independent scaling of crawl workers

## References

### Internal References

- Current crawling logic: `packages/server/src/domains/posts/effects/crawler.rs`
- Current commands: `packages/server/src/domains/posts/commands/mod.rs:190-267`
- Seesaw architecture: `docs/architecture/SEESAW_ARCHITECTURE.md`
- Domain architecture: `docs/architecture/DOMAIN_ARCHITECTURE.md`
- Effect pattern rules: `CLAUDE.md` (thin orchestrators)

### Related Work

- Domain approval workflow: `docs/architecture/domain-approval-workflow.md` (shows separation pattern)
