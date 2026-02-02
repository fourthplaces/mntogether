---
title: Codebase Health Audit & Remediation
type: refactor
date: 2026-02-02
---

# Codebase Health Audit & Remediation Plan

## Overview

Comprehensive audit of the server codebase identified architectural debt from the seesaw 0.5.0 migration. The codebase compiles cleanly and library tests pass (96/96), but contains dead code, unnecessary indirection, and thick effects that violate CLAUDE.md rules.

## Critical Finding: Dead Internal Edges Pattern

### The Problem

Internal edges exist in 4 domains but serve no purpose:

```
src/domains/crawling/edges/internal.rs
src/domains/chatrooms/edges/internal.rs
src/domains/website/edges/internal.rs
src/domains/domain_approval/edges/internal.rs
```

These were designed for the **old machine architecture** where:
- Machines were separate stateful observers
- They listened to "fact events" and decided what "request events" to emit
- This decoupled "what happened" from "what should happen next"

**In seesaw 0.5.0, this is unnecessary indirection.** Effects already know what should happen next. Instead of:

```
Effect → emits FactEvent → edge transforms → RequestEvent → Effect handles
```

Just do:

```
Effect → calls next action directly (or emits next request event)
```

### Solution

**Delete all internal edges** and have actions emit events directly.

**Key architectural decision:** Actions take `ctx` and emit events themselves. This keeps event emission close to the business logic.

```rust
// Effect is ultra-thin - just delegates to action
async fn handle(&mut self, event: CrawlEvent, ctx: EffectContext<...>) -> Result<()> {
    match event {
        CrawlEvent::ExtractPostsRequested { website_id, pages, .. } => {
            actions::extract_posts(website_id, pages, &ctx).await?;
            Ok(())
        }
        ...
    }
}

// Action does work AND emits events
pub async fn extract_posts(
    website_id: WebsiteId,
    pages: Vec<Page>,
    ctx: &EffectContext<ServerDeps, RequestState>,
) -> Result<()> {
    let posts = do_extraction(pages, ctx.deps()).await?;

    // Action emits events directly - no returning to effect
    ctx.emit(CrawlEvent::PostsExtracted { website_id, posts: posts.clone() });

    // Chain to next step directly
    sync_posts(website_id, posts, ctx).await?;

    Ok(())
}
```

**Benefits:**
- Events emitted at the point of business logic, not marshaled back through effect
- Natural chaining - one action can call the next
- Effect stays ultra-thin (just a match statement dispatching to actions)
- No intermediate "fact" events that need transformation

---

## Issues Summary

| Priority | Issue | Action |
|----------|-------|--------|
| **P1** | 51 emit calls in effects (0 in actions) | Move ALL emitting to actions |
| **P1** | Dead internal edges pattern | Delete all `edges/internal.rs` files |
| **P1** | Dead machine code in `posts/extraction/` | Delete entire directory |
| **P1** | Duplicate extraction in `posts/effects/extraction/` | Delete directory |
| **P2** | Existing actions return events instead of emitting | Refactor to take ctx and emit |
| **P2** | Thick effects (1027 lines in `post.rs`) | Extract to actions |
| **P3** | Docs say 0.3.0, actual is 0.5.0 | Update comments |
| **P3** | Deprecated announcement functions | Remove or mark deprecated |

---

## Scope: Event Emission Migration

**All 51 emit calls must move from effects to actions.**

### Current State by Domain

| Domain | Actions Exist? | Emits in Effects | Actions Pattern |
|--------|---------------|------------------|-----------------|
| **posts** | Stub only | 28 emits | Need full migration |
| **crawling** | Yes (7 files) | 9 emits | Actions return events, need refactor |
| **chatrooms** | Yes (4 files) | 5 emits | Actions return events, need refactor |
| **website** | Stub only | 4 emits | Need full migration |
| **domain_approval** | Stub only | 3 emits | Need full migration |
| **member** | Yes (3 files) | 3 emits | Actions return events, need refactor |
| **auth** | Yes (2 files) | 2 emits | Actions return events, need refactor |

### Migration Pattern

**Before (current - actions return events):**
```rust
// crawling/actions/crawl_website.rs
pub async fn crawl_website_pages(
    website: &Website,
    job_id: JobId,
    deps: &ServerDeps,
) -> Result<Vec<CrawledPageInfo>, CrawlEvent> {
    // ... do work ...
    Err(CrawlEvent::WebsiteCrawlFailed { ... })  // RETURNS event
}

// crawling/effects/crawler.rs
let result = actions::crawl_website_pages(&website, job_id, deps).await;
ctx.emit(result);  // Effect emits
```

**After (target - actions emit directly):**
```rust
// crawling/actions/crawl_website.rs
pub async fn crawl_website_pages(
    website: &Website,
    job_id: JobId,
    ctx: &EffectContext<ServerDeps, RequestState>,
) -> Result<()> {
    // ... do work ...
    ctx.emit(CrawlEvent::WebsiteCrawlFailed { ... });  // ACTION emits
    Ok(())
}

// crawling/effects/crawler.rs
actions::crawl_website_pages(&website, job_id, &ctx).await?;  // No emit in effect
```

---

## Phase 1: Delete Dead Code

### 1.1 Delete Internal Edges

**Files to delete:**
- `src/domains/crawling/edges/internal.rs`
- `src/domains/chatrooms/edges/internal.rs`
- `src/domains/website/edges/internal.rs`
- `src/domains/domain_approval/edges/internal.rs`
- `src/domains/member/edges/internal.rs` (if exists)
- `src/domains/posts/edges/internal.rs` (if exists)

**Update `edges/mod.rs`** in each domain to remove the `pub mod internal;` line.

### 1.2 Delete Legacy Machine Code

**Directory to delete:** `src/domains/posts/extraction/`

Contains dead code:
- `machines.rs` - `PostExtractionMachine` implements removed `seesaw_core::Machine` trait
- `commands.rs` - `PostExtractionCommand` never dispatched
- `effects.rs` - `PostExtractionEffect` never registered
- `events.rs` - `PostExtractionEvent` never emitted
- `mod.rs` - re-exports for dead code

### 1.3 Delete Duplicate Extraction Module

**Directory to delete:** `src/domains/posts/effects/extraction/`

Already marked deprecated, imports from `crawling::effects::extraction` anyway.

---

## Phase 2: Migrate Existing Actions to Emit Directly

Refactor existing actions to take `ctx` and emit events directly.

### 2.1 Crawling Domain (9 emits → actions)

**Files to refactor:**
- `actions/crawl_website.rs` - Change `Result<Vec<CrawledPageInfo>, CrawlEvent>` → `Result<()>`, take ctx
- `actions/extract_posts.rs` - Change `Result<ExtractionResult, CrawlEvent>` → `Result<()>`, take ctx
- `actions/sync_posts.rs` - Change return type, take ctx
- `actions/authorization.rs` - Change `Result<(), CrawlEvent>` → emit auth denied directly

**Effect changes:**
- `effects/crawler.rs` - Remove all `ctx.emit(result)` calls, just call actions

### 2.2 Chatrooms Domain (5 emits → actions)

**Files to refactor:**
- `actions/create_container.rs` - Take ctx, emit `ContainerCreated`
- `actions/create_message.rs` - Take ctx, emit `MessageCreated`
- `actions/generate_greeting.rs` - Take ctx, emit result
- `actions/generate_reply.rs` - Take ctx, emit result

### 2.3 Member Domain (3 emits → actions)

**Files to refactor:**
- `actions/register_member.rs` - Take ctx, emit `MemberRegistered`
- `actions/update_status.rs` - Take ctx, emit `MemberStatusUpdated`
- `actions/generate_embedding.rs` - Take ctx, emit `EmbeddingGenerated`

### 2.4 Auth Domain (2 emits → actions)

**Files to refactor:**
- `actions/send_otp.rs` - Take ctx, emit `OtpSent`
- `actions/verify_otp.rs` - Take ctx, emit `OtpVerified`

### 1.4 Inline Edge Logic Into Actions

For any edge logic that was actually needed, inline it into actions that chain directly.

**Crawling domain - actions chain naturally:**

```rust
// actions/extract_posts.rs
pub async fn extract_posts(..., ctx: &EffectContext<...>) -> Result<()> {
    let posts = do_extraction(...).await?;
    ctx.emit(CrawlEvent::PostsExtracted { ... });

    // Chain directly - no edge needed
    sync_posts(website_id, posts, ctx).await
}

// actions/handle_no_listings.rs
pub async fn handle_no_listings(should_retry: bool, ..., ctx: &EffectContext<...>) -> Result<()> {
    if should_retry {
        retry_crawl(website_id, ctx).await
    } else {
        mark_website_no_posts(website_id, ctx).await
    }
}
```

**Chatrooms domain - actions chain naturally:**

```rust
// actions/create_message.rs
pub async fn create_message(..., ctx: &EffectContext<...>) -> Result<()> {
    let message = Message::create(...).await?;
    ctx.emit(ChatEvent::MessageCreated { ... });

    // Chain directly if user message needs AI reply
    if message.role == "user" {
        generate_reply(message.id, ctx).await?;
    }
    Ok(())
}

// actions/create_container.rs
pub async fn create_container(with_agent: Option<String>, ..., ctx: &EffectContext<...>) -> Result<()> {
    let container = Container::create(...).await?;
    ctx.emit(ChatEvent::ContainerCreated { ... });

    // Chain directly if agent greeting needed
    if let Some(agent_config) = with_agent {
        generate_greeting(container.id, agent_config, ctx).await?;
    }
    Ok(())
}
```

---

## Phase 3: Create New Actions for Stub Domains

### 3.1 Posts Domain (28 emits → new actions)

**Current state:** `post.rs` is 1027 lines, `actions/mod.rs` is a stub

**Target:** Effect <50 lines, all business logic in `posts/actions/`

**Action pattern:** Actions take `&EffectContext` and emit events directly.

```rust
// posts/actions/approve_post.rs
pub async fn approve_post(
    post_id: PostId,
    approved_by: MemberId,
    ctx: &EffectContext<ServerDeps, RequestState>,
) -> Result<()> {
    let post = Post::find_by_id(post_id, &ctx.deps().db_pool).await?;
    post.approve(approved_by, &ctx.deps().db_pool).await?;

    ctx.emit(PostEvent::PostApproved { post_id, approved_by });
    Ok(())
}
```

**Actions to create (from post.rs emit analysis):**
```
posts/actions/
├── mod.rs
├── approve_post.rs         # PostApproved
├── reject_post.rs          # PostRejected
├── submit_post.rs          # PostSubmitted
├── create_post.rs          # PostCreated
├── scrape_source.rs        # SourceScraped
├── crawl_posts.rs          # PostsCrawled
├── sync_posts.rs           # PostsSynced
├── generate_embedding.rs   # PostEmbeddingGenerated
├── track_view.rs           # PostViewed
├── track_click.rs          # PostClicked
├── report_post.rs          # PostReported
├── resolve_report.rs       # ReportResolved
├── expire_post.rs          # PostExpired
├── archive_post.rs         # PostArchived
├── repost_post.rs          # PostReposted
└── delete_post.rs          # PostDeleted
```

### 3.2 Website Domain (4 emits → new actions)

**Current state:** `effects/mod.rs` has handlers, `actions/mod.rs` is a stub

**Actions to create:**
```
website/actions/
├── mod.rs
├── regenerate_posts.rs         # PostsRegenerated
├── regenerate_summaries.rs     # SummariesRegenerated
├── regenerate_page_summary.rs  # PageSummaryRegenerated
└── regenerate_page_posts.rs    # PagePostsRegenerated
```

### 3.3 Domain Approval Domain (3 emits → new actions)

**Current state:** `effects/mod.rs` has handlers, `actions/mod.rs` is a stub

**Actions to create:**
```
domain_approval/actions/
├── mod.rs
├── generate_assessment.rs  # AssessmentGenerated
├── search_websites.rs      # WebsiteSearchCompleted
└── check_fraud.rs          # FraudCheckCompleted
```

---

## Phase 3: Documentation Cleanup

### 3.1 Update Version References

Find and replace `seesaw 0.3.0` → `seesaw 0.5.0` in:
- `src/domains/crawling/mod.rs`
- `src/domains/crawling/events/mod.rs`
- `src/domains/member/mod.rs`
- `src/domains/website/mod.rs`
- `src/domains/domain_approval/mod.rs`
- `src/domains/posts/events/mod.rs`

### 3.2 Remove Edge Documentation

Update module docs that reference "internal edges" pattern since it's being removed.

---

## Acceptance Criteria

### Phase 1: Delete Dead Code
- [ ] All `edges/internal.rs` files deleted (6 files)
- [ ] `posts/extraction/` directory deleted (5 files)
- [ ] `posts/effects/extraction/` directory deleted
- [ ] No references to deleted code remain
- [ ] Build passes: `cargo build --all-targets`

### Phase 2: Migrate Existing Actions
- [ ] Crawling actions take ctx and emit directly (9 emits moved)
- [ ] Chatrooms actions take ctx and emit directly (5 emits moved)
- [ ] Member actions take ctx and emit directly (3 emits moved)
- [ ] Auth actions take ctx and emit directly (2 emits moved)
- [ ] **0 emit calls remain in effects for these domains**

### Phase 3: Create New Actions
- [ ] Posts domain: 28 emits moved to new actions, `post.rs` <100 lines
- [ ] Website domain: 4 emits moved to new actions
- [ ] Domain approval: 3 emits moved to new actions
- [ ] **Total: 0 emit calls in any effects directory**

### Final Verification
```bash
# Should return 0
grep -rn "ctx.emit" packages/server/src/domains/*/effects/ | wc -l

# Should return 51+ (all emits now in actions)
grep -rn "ctx.emit" packages/server/src/domains/*/actions/ | wc -l
```

---

## Verification

```bash
# After Phase 1
cargo build --all-targets
cargo test --lib

# Check no internal.rs files remain
find packages/server/src -name "internal.rs" -path "*/edges/*"

# Check no extraction directory in posts
ls packages/server/src/domains/posts/extraction 2>/dev/null && echo "FAIL: directory exists" || echo "OK: deleted"

# Check for stale imports
grep -r "edges::internal" packages/server/src/
grep -r "posts::extraction" packages/server/src/
```

---

## Files Summary

### Phase 1: Files to Delete

```
# Internal edges (dead code - never wired up)
src/domains/crawling/edges/internal.rs
src/domains/chatrooms/edges/internal.rs
src/domains/website/edges/internal.rs
src/domains/domain_approval/edges/internal.rs
src/domains/member/edges/internal.rs
src/domains/posts/edges/internal.rs

# Legacy machine code (dead - seesaw 0.5.0 removed Machine trait)
src/domains/posts/extraction/  (entire directory)

# Duplicate extraction (deprecated)
src/domains/posts/effects/extraction/  (entire directory)
```

### Phases 2-3: Files to Modify

**Effects to thin out (remove all ctx.emit calls):**
- `domains/crawling/effects/crawler.rs` (9 emits → 0)
- `domains/chatrooms/effects/chat.rs` (5 emits → 0)
- `domains/member/effects/mod.rs` (3 emits → 0)
- `domains/auth/effects.rs` (2 emits → 0)
- `domains/posts/effects/post.rs` (20 emits → 0)
- `domains/posts/effects/scraper.rs` (2 emits → 0)
- `domains/posts/effects/ai.rs` (2 emits → 0)
- `domains/posts/effects/sync.rs` (1 emit → 0)
- `domains/website/effects/mod.rs` (4 emits → 0)
- `domains/domain_approval/effects/mod.rs` (3 emits → 0)

**Actions to refactor (add ctx parameter, emit directly):**
- `domains/crawling/actions/*.rs` (7 files)
- `domains/chatrooms/actions/*.rs` (4 files)
- `domains/member/actions/*.rs` (3 files)
- `domains/auth/actions/*.rs` (2 files)

**Actions to create:**
- `domains/posts/actions/*.rs` (~16 new files)
- `domains/website/actions/*.rs` (~4 new files)
- `domains/domain_approval/actions/*.rs` (~3 new files)

---

## References

- `src/domains/crawling/actions/` - Reference pattern for actions
- `CLAUDE.md` - Effect <50 lines rule, actions pattern
