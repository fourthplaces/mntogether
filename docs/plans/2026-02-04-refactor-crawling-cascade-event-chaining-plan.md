---
title: Refactor ALL Domains to Proper Event Chaining
type: refactor
date: 2026-02-04
deepened: 2026-02-04
scope: crawling, chatrooms, posts, website_approval
---

# Refactor ALL Domains to Proper Event Chaining

## Enhancement Summary

**Deepened on:** 2026-02-04
**Research agents used:** architecture-strategist, code-simplicity-reviewer, pattern-recognition-specialist, performance-oracle, best-practices-researcher, repo-research-analyst

### Domains Covered

| Domain | Effect LOC | Anti-Pattern | Priority |
|--------|-----------|--------------|----------|
| **crawling** | 474 | handlers.rs + 3x duplicate inline chains | High |
| **chatrooms** | 361 | handlers.rs + 2x inline chains | High |
| **posts** | 2237 | composite.rs + deeply nested chains | High |
| **website_approval** | 198 | 2x inline handler chains | Medium |

**Total Effect LOC to refactor: 3,270 lines**

### Key Decision: Atomized Event Chains

Each step must emit an event that triggers the next effect. This enables:
1. **Any step can become a job** - Just enqueue instead of executing inline
2. **Independent retry** - Failed steps can be retried without re-running previous steps
3. **Observability** - Every state transition is visible in the event log
4. **Testability** - Each effect can be tested in isolation

**Pattern:**
```
Event₁ → Effect₁ → returns Event₂
Event₂ → Effect₂ → returns Event₃
Event₃ → Effect₃ → returns Event₄ (terminal)
```

**NOT this (consolidated action):**
```rust
// WRONG: Can't make step 2 a job without re-architecting
pub async fn process_all(deps) -> Result<()> {
    let step1 = do_step1().await?;
    let step2 = do_step2(step1).await?;  // What if this needs to be a job?
    let step3 = do_step3(step2).await?;
    Ok(())
}
```

---

## Domain 1: Crawling (474 LOC)

### Current Anti-Pattern

**File:** `domains/crawling/effects/crawler.rs:40-135`

```rust
// ANTI-PATTERN: 3 duplicate match arms (90 lines)
CrawlEvent::WebsiteIngested { website_id, job_id, .. } => {
    match handlers::handle_enqueue_extract_posts(*website_id, *job_id, ctx.deps()).await {
        Ok((posts, page_results)) => {
            // Inline chain to sync
            if let Err(e) = handlers::handle_enqueue_sync_posts(...).await {
                error!(error = %e, "Sync posts failed");
            }
        }
        Err(e) => error!(error = %e, "Extract posts failed"),
    }
    Ok(()) // Terminal
}
// Same pattern repeated for WebsitePostsRegenerated and WebsitePagesDiscovered
```

**Problems:**
- 3 identical 30-line match arms (WebsiteIngested, WebsitePostsRegenerated, WebsitePagesDiscovered)
- `PostsExtractedFromPages` event never emitted - invisible state
- handlers.rs is just pass-through to job executors

### Refactor Plan: Atomized Event Chain

**Event Chain:**
```
WebsiteIngested / WebsitePostsRegenerated / WebsitePagesDiscovered
  → Effect: extract
    → returns PostsExtractedFromPages (or WebsiteCrawlNoListings)

PostsExtractedFromPages
  → Effect: sync
    → returns PostsSynced (terminal)

WebsiteCrawlNoListings
  → Effect: mark
    → returns WebsiteMarkedNoListings (terminal)
```

**Step 1:** Each effect arm does ONE thing and returns ONE event

```rust
use seesaw_core::on;

let effects = on! {
    // Step 1: Extract - triggers on any "ready to extract" event
    // Returns PostsExtractedFromPages or WebsiteCrawlNoListings
    CrawlEvent::WebsiteIngested { website_id, job_id, .. } |
    CrawlEvent::WebsitePostsRegenerated { website_id, job_id, .. } |
    CrawlEvent::WebsitePagesDiscovered { website_id, job_id, .. } => |ctx| async move {
        let result = actions::extract_posts_for_website(website_id, job_id, ctx.deps()).await?;

        if result.posts.is_empty() {
            Ok(CrawlEvent::WebsiteCrawlNoListings { website_id, job_id })
        } else {
            Ok(CrawlEvent::PostsExtractedFromPages {
                website_id,
                job_id,
                posts: result.posts,
                page_results: result.page_results,
            })
        }
    },

    // Step 2: Sync - triggers on extraction complete
    // Returns PostsSynced (terminal)
    CrawlEvent::PostsExtractedFromPages { website_id, job_id, posts, .. } => |ctx| async move {
        let result = actions::sync_posts(website_id, posts, ctx.deps()).await?;
        Ok(CrawlEvent::PostsSynced {
            website_id,
            job_id,
            new_count: result.new_count,
            updated_count: result.updated_count,
            unchanged_count: result.unchanged_count,
        })
    },

    // Step 3: Mark no listings - triggers on empty extraction
    // Returns WebsiteMarkedNoListings (terminal)
    CrawlEvent::WebsiteCrawlNoListings { website_id, job_id, .. } => |ctx| async move {
        actions::mark_website_no_listings(website_id, ctx.deps()).await?;
        Ok(CrawlEvent::WebsiteMarkedNoListings { website_id, job_id })
    },

    // Terminal events - chain complete
    CrawlEvent::PostsSynced { .. } |
    CrawlEvent::WebsiteMarkedNoListings { .. } => |_ctx| async move {
        Ok(())
    },
};
```

**Why this matters:** Any step can become a job:

```rust
// CURRENT: Inline execution
CrawlEvent::PostsExtractedFromPages { website_id, posts, .. } => |ctx| async move {
    let result = actions::sync_posts(website_id, posts, ctx.deps()).await?;
    Ok(CrawlEvent::PostsSynced { ... })
},

// FUTURE: Background job execution
CrawlEvent::PostsExtractedFromPages { website_id, job_id, posts, .. } => |ctx| async move {
    ctx.deps().jobs.enqueue(SyncPostsJob { website_id, job_id, posts }).await?;
    Ok(CrawlEvent::SyncJobEnqueued { website_id, job_id })
    // Job worker will emit PostsSynced when done
},
```

**Step 2:** Delete `handlers.rs` (111 lines) - handlers just wrap job executors

**Step 3:** Actions return data, effects build events

### Files to Modify

| File | Action | LOC Change |
|------|--------|------------|
| `effects/crawler.rs` | Collapse 3 arms → 1, remove dead code | -90 lines |
| `effects/handlers.rs` | **DELETE** | -111 lines |
| `effects/mod.rs` | Remove handlers export | -3 lines |

**Estimated reduction:** 195 → ~70 lines (64% reduction)

---

## Domain 2: Chatrooms (361 LOC)

### Current Anti-Pattern

**File:** `domains/chatrooms/effects/chat.rs:24-87`

```rust
// Pattern 1: ContainerCreated → generate greeting
ChatEvent::ContainerCreated { container, with_agent } => {
    if let Some(agent_config) = with_agent {
        match handlers::handle_generate_greeting(container.id, agent_config.clone(), ctx.deps()).await {
            Ok(message) => info!(message_id = %message.id, "Greeting generated"),
            Err(e) => error!(error = %e, "Failed to generate greeting"),
        }
    }
    Ok(()) // Terminal
}

// Pattern 2: MessageCreated → generate reply (if user message + has agent)
ChatEvent::MessageCreated { message } => {
    if message.role != "user" { return Ok(()); }
    if let Some(_) = handlers::get_container_agent_config(message.container_id, &pool).await {
        match handlers::handle_generate_reply(message.id, message.container_id, ctx.deps()).await {
            Ok(reply) => info!(reply_id = %reply.id, "Reply generated"),
            Err(e) => error!(error = %e, "Failed to generate reply"),
        }
    }
    Ok(()) // Terminal
}
```

**Problems:**
- handlers.rs contains business logic (264 lines) that should be in actions
- No events emitted for GreetingGenerated or ReplyGenerated - invisible cascade

### Refactor Plan

**Step 1:** Move handler logic to actions

```rust
// actions/ai_responses.rs (new)
pub async fn generate_greeting(container_id: ContainerId, agent_config: &str, deps: &ServerDeps) -> Result<Message> { ... }
pub async fn generate_reply(message_id: MessageId, container_id: ContainerId, deps: &ServerDeps) -> Result<Message> { ... }
```

**Step 2:** Simplify effect to call actions directly

```rust
ChatEvent::ContainerCreated { container, with_agent } => {
    if let Some(agent_config) = with_agent {
        actions::generate_greeting(container.id, &agent_config, ctx.deps()).await?;
    }
    Ok(())
}

ChatEvent::MessageCreated { message } => {
    if message.role == "user" && actions::container_has_agent(message.container_id, &ctx.deps().db_pool).await {
        actions::generate_reply(message.id, message.container_id, ctx.deps()).await?;
    }
    Ok(())
}
```

**Step 3:** Delete handlers.rs, keep helper functions as actions

### Files to Modify

| File | Action | LOC Change |
|------|--------|------------|
| `effects/chat.rs` | Simplify to call actions | -20 lines |
| `effects/handlers.rs` | **DELETE** (move to actions) | -264 lines |
| `actions/ai_responses.rs` | **NEW** (from handlers) | +180 lines |
| `effects/mod.rs` | Remove handlers export | -2 lines |

**Net reduction:** 361 → ~180 lines (50% reduction in effects)

---

## Domain 3: Posts (2237 LOC in effects)

### Current Anti-Pattern

**File:** `domains/posts/effects/composite.rs:29-228`

```rust
// ANTI-PATTERN: Deeply nested inline chaining (228 lines)
PostEvent::WebsiteCreatedFromLink { job_id, url, submitter_contact, .. } => {
    // Step 1: Scrape
    let scraped_event = match handle_scrape_resource_link(...).await {
        Ok(e) => e,
        Err(e) => { error!(...); return Ok(()); }
    };

    // Step 2: Extract (nested inside step 1 success)
    if let PostEvent::ResourceLinkScraped { ... } = scraped_event {
        let extracted_event = match handle_extract_posts_from_resource_link(...).await {
            Ok(e) => e,
            Err(e) => { error!(...); return Ok(()); }
        };

        // Step 3: Create (nested inside step 2 success)
        if let PostEvent::ResourceLinkPostsExtracted { ... } = extracted_event {
            match handle_create_posts_from_resource_link(...).await {
                Ok(PostEvent::PostEntryCreated { title, .. }) => info!(...),
                Err(e) => error!(...),
            }
        }
    }
    Ok(())
}
// Pattern repeated for ResourceLinkScraped and ResourceLinkPostsExtracted
```

**Problems:**
- 3 cascade handlers with deeply nested `if let` chains
- Handlers return events but effect ignores them (returns `Ok(())`)
- Same logic repeated 3 times for different entry points
- Error swallowing with early `return Ok(())`

### Refactor Plan: Atomized Event Chain

**Event Chain:**
```
WebsiteCreatedFromLink
  → Effect: scrape
    → returns ResourceLinkScraped

ResourceLinkScraped
  → Effect: extract
    → returns ResourceLinkPostsExtracted

ResourceLinkPostsExtracted
  → Effect: create
    → returns PostEntryCreated (terminal)
```

**Step 1:** Each effect arm does ONE thing and returns ONE event

```rust
use seesaw_core::on;

let effects = on! {
    // Step 1: Scrape - returns ResourceLinkScraped
    PostEvent::WebsiteCreatedFromLink { job_id, url, submitter_contact, .. } => |ctx| async move {
        let result = actions::scrape_resource_link(job_id, &url, ctx.deps()).await?;
        Ok(PostEvent::ResourceLinkScraped {
            job_id,
            url,
            content: result.content,
            context: result.context,
            submitter_contact,
        })
    },

    // Step 2: Extract - returns ResourceLinkPostsExtracted
    PostEvent::ResourceLinkScraped { job_id, url, content, context, submitter_contact, .. } => |ctx| async move {
        let result = actions::extract_posts_from_content(job_id, &url, &content, ctx.deps()).await?;
        Ok(PostEvent::ResourceLinkPostsExtracted {
            job_id,
            url,
            posts: result.posts,
            context,
            submitter_contact,
        })
    },

    // Step 3: Create - returns PostEntryCreated (terminal)
    PostEvent::ResourceLinkPostsExtracted { job_id, url, posts, context, submitter_contact } => |ctx| async move {
        let result = actions::create_posts(job_id, &url, posts, submitter_contact, ctx.deps()).await?;
        Ok(PostEvent::PostEntryCreated {
            post_id: result.post_id,
            title: result.title,
            organization_name: result.organization_name,
            submission_type: "resource_link".to_string(),
        })
    },

    // Terminal events - no action needed
    PostEvent::PostEntryCreated { .. } | ... => |_ctx| async move {
        Ok(())
    },
};
```

**Step 2:** Actions become pure functions that return data (not events)

```rust
// actions/scraping.rs - returns data, effect builds event
pub async fn scrape_resource_link(job_id: JobId, url: &str, deps: &ServerDeps) -> Result<ScrapeResult> {
    // ... scraping logic
    Ok(ScrapeResult { content, context })
}

// actions/extraction.rs - returns data, effect builds event
pub async fn extract_posts_from_content(job_id: JobId, url: &str, content: &str, deps: &ServerDeps) -> Result<ExtractionResult> {
    // ... extraction logic
    Ok(ExtractionResult { posts })
}

// actions/creation.rs - returns data, effect builds event
pub async fn create_posts(job_id: JobId, url: &str, posts: Vec<ExtractedPost>, submitter: Option<String>, deps: &ServerDeps) -> Result<CreateResult> {
    // ... creation logic
    Ok(CreateResult { post_id, title, organization_name })
}
```

**Why this matters:** Any step can become a job:

```rust
// CURRENT: Inline execution
PostEvent::WebsiteCreatedFromLink { .. } => |ctx| async move {
    let result = actions::scrape_resource_link(...).await?;
    Ok(PostEvent::ResourceLinkScraped { ... })
},

// FUTURE: Background job execution
PostEvent::WebsiteCreatedFromLink { .. } => |ctx| async move {
    ctx.deps().jobs.enqueue(ScrapeJob { job_id, url }).await?;
    Ok(PostEvent::ScrapeJobEnqueued { job_id })  // Job will emit ResourceLinkScraped when done
},
```

### Files to Modify

| File | Action | LOC Change |
|------|--------|------------|
| `effects/composite.rs` | Rewrite with `on!` macro, each arm returns event | ~0 (restructure) |
| `effects/scraper.rs` | Return `PostEvent` instead of `Result<PostEvent>` | minor |
| `effects/ai.rs` | Return `PostEvent` instead of internal chaining | minor |
| `effects/post.rs` | Return `PostEvent` instead of internal chaining | minor |

**Net change:** Same LOC but atomized - each effect is independent and chainable

---

## Domain 4: Website Approval (198 LOC)

### Current Anti-Pattern

**File:** `domains/website_approval/effects/mod.rs:42-124`

```rust
// Pattern: WebsiteResearchCreated → search → assess (inline chain)
WebsiteApprovalEvent::WebsiteResearchCreated { research_id, website_id, job_id, requested_by, .. } => {
    match handle_conduct_searches(*research_id, *website_id, *job_id, *requested_by, &ctx).await {
        Ok(result) => {
            info!("Searches completed, now generating assessment");
            // INLINE CHAIN to assessment
            if let Err(e) = handle_generate_assessment(*research_id, *website_id, *job_id, *requested_by, &ctx).await {
                error!(error = %e, "Assessment generation failed");
            }
        }
        Err(e) => error!(error = %e, "Search cascade failed"),
    }
    Ok(())
}

// Duplicate handling for ResearchSearchesCompleted
WebsiteApprovalEvent::ResearchSearchesCompleted { ... } => {
    // Same assessment call duplicated
    if let Err(e) = handle_generate_assessment(...).await { ... }
    Ok(())
}
```

**Problems:**
- Inline chaining from search → assessment
- Duplicate assessment call in two match arms
- Handlers take `&EffectContext` (should take `&ServerDeps`)

### Refactor Plan

**Step 1:** Create unified assessment action

```rust
// actions/assessment.rs
pub async fn conduct_research_and_assess(
    research_id: ResearchId,
    website_id: WebsiteId,
    job_id: JobId,
    requested_by: MemberId,
    deps: &ServerDeps,
) -> Result<AssessmentResult> {
    let search_result = search::conduct_searches(research_id, website_id, deps).await?;
    let assessment = assessment::generate_assessment(research_id, website_id, &search_result, deps).await?;
    Ok(assessment)
}
```

**Step 2:** Simplify effect

```rust
WebsiteApprovalEvent::WebsiteResearchCreated { research_id, website_id, job_id, requested_by, .. } => {
    actions::conduct_research_and_assess(*research_id, *website_id, *job_id, *requested_by, ctx.deps()).await?;
    Ok(())
}

// ResearchSearchesCompleted can be removed or simplified if not needed
```

**Step 3:** Update handlers to take `&ServerDeps` instead of `&EffectContext`

### Files to Modify

| File | Action | LOC Change |
|------|--------|------------|
| `effects/mod.rs` | Simplify, remove duplicate | -40 lines |
| `effects/search.rs` | Change signature to `&ServerDeps` | ~0 |
| `effects/assessment.rs` | Change signature to `&ServerDeps` | ~0 |
| `actions/assessment.rs` | **NEW** (consolidated) | +30 lines |

**Net reduction:** 136 → ~80 lines (41% reduction)

---

## Summary: All Domains

### Architecture Change

**Before:** Effects call handlers inline, chain to next handler inside match arm
**After:** Effects return events, next effect triggers on that event

This enables:
- Any step can become a background job
- Independent retry/observability per step
- Clear event log of all state transitions

### Total Impact

| Domain | Before | After | Pattern |
|--------|--------|-------|---------|
| crawling | 474 | ~150 | 3 events in chain |
| chatrooms | 361 | ~100 | 2 events in chain |
| posts | 2237 | ~200 | 3 events in chain |
| website_approval | 198 | ~100 | 2 events in chain |

### Files to Delete

| File | Lines |
|------|-------|
| `crawling/effects/handlers.rs` | 111 |
| `chatrooms/effects/handlers.rs` | 264 |
| **Total deleted** | **375 lines** |

### New Action Files

| File | Lines | Purpose |
|------|-------|---------|
| `chatrooms/actions/ai_responses.rs` | ~180 | Moved from handlers |
| `posts/actions/resource_link_cascade.rs` | ~80 | Consolidated cascade |
| `website_approval/actions/assessment.rs` | ~30 | Consolidated flow |
| **Total new** | **~290 lines** |

---

## Implementation Order

### Phase 1: Crawling (Simplest)
1. [x] Collapse 3 match arms with `|`
2. [x] Delete handlers.rs (111 lines removed)
3. [x] Test (142 tests pass)

### Phase 2: Chatrooms
1. [x] Move handlers.rs to actions/ai_responses.rs (~180 lines)
2. [x] Simplify chat.rs (87 → 62 lines)
3. [x] Delete handlers.rs (265 lines removed from effects)
4. [x] Test (142 tests pass)

### Phase 3: Website Approval
1. [x] Simplify effects/mod.rs to call actions directly
2. [x] Delete handler files (search.rs, assessment.rs)
3. [x] Test (compiles without errors)

### Phase 4: Posts (Largest)
1. [x] Flatten composite.rs using atomized event chain with `on!` macro
2. [x] Use `effect::group()` to combine 3 individual effects
3. [x] Test (142 tests pass)

---

## Acceptance Criteria

### All Domains

- [x] No `handlers.rs` files in effects directories (crawling, chatrooms, website_approval cleaned)
- [x] No nested `if let` chains deeper than 1 level (posts composite flattened)
- [x] All effects return `Ok(())` or `Ok(Event)` (atomized event chain)
- [x] Each match arm is <20 lines (using `on!` macro)
- [x] Duplicate match arms consolidated with `|` syntax (crawling effects)

### Testing

- [x] All existing tests pass (142 tests)
- [ ] `cargo check --package server` compiles without warnings (some deprecated warnings remain)

---

## References

### Internal
- Seesaw 0.7.2 upgrade plan: `docs/plans/2026-02-04-refactor-upgrade-seesaw-to-0.7.2-plan.md`
- Current crawling: `packages/server/src/domains/crawling/effects/crawler.rs`
- Current chatrooms: `packages/server/src/domains/chatrooms/effects/chat.rs`
- Current posts: `packages/server/src/domains/posts/effects/composite.rs`
- Current website_approval: `packages/server/src/domains/website_approval/effects/mod.rs`

### External Best Practices
- [Solace - Event-Driven Architecture Patterns](https://solace.com/event-driven-architecture-patterns/)
- [Confluent - Event Design Best Practices](https://developer.confluent.io/courses/event-design/best-practices/)
- [Microsoft - Domain Events Design](https://learn.microsoft.com/en-us/dotnet/architecture/microservices/microservice-ddd-cqrs-patterns/domain-events-design-implementation)
