---
title: Upgrade Seesaw to 0.7.2
type: refactor
date: 2026-02-04
---

# Upgrade Seesaw to 0.7.2

## Overview

Migrate from Seesaw 0.6.2 to 0.7.2, a significant architectural change that removes `ctx.emit()` in favor of returning events directly from effects. This enforces explicit event chaining with no hidden side effects.

**Scope:** 49 `ctx.emit()` calls across 24 files in 8 domains.

## Problem Statement / Motivation

The current 0.6.2 architecture has hidden event emission through `ctx.emit()` calls scattered throughout handlers and actions. This makes event flows implicit and harder to trace. Seesaw 0.7.2 enforces:

1. **Explicit event returns** - Effects return `Ok(Event)` instead of calling `ctx.emit()`
2. **Forced chaining** - Event flows are declared at the effect registration level
3. **Reducers** - Pure state transformations that run before effects
4. **`on!` macro** - Ergonomic multi-variant event matching

## Proposed Solution

A phased migration from the current `.run()` pattern to the new `.extract().then()` and `on!` macro patterns, starting with the simplest domains and progressively migrating complex event chains.

### Current Pattern (0.6.2):

```rust
// Effect registration with .run() and manual match
effect::on::<CrawlEvent>().run(|event: Arc<CrawlEvent>, ctx| async move {
    match event.as_ref() {
        CrawlEvent::WebsiteIngested { website_id, job_id, .. } => {
            handle_enqueue_extract_posts(*website_id, *job_id, &ctx).await
        }
        CrawlEvent::PostsExtractedFromPages { website_id, posts, .. } => {
            handle_enqueue_sync_posts(*website_id, posts.clone(), &ctx).await
        }
        _ => Ok(()),
    }
})

// Handler calls ctx.emit() - HIDDEN SIDE EFFECT
async fn handle_enqueue_extract_posts(..., ctx: &EffectContext<...>) -> Result<()> {
    let result = execute_extract_posts_job(job, ctx).await?;
    ctx.emit(CrawlEvent::PostsExtractedFromPages { ... });  // Hidden emit!
    Ok(())
}
```

### Target Pattern (0.7.2):

#### Option A: `on!` Macro (Preferred for Multi-Variant)

```rust
use seesaw_core::on;

// Match-like syntax with Event::Variant patterns
// Returns Vec<Effect<S, D>> - fold into engine
let effects = on! {
    // Multiple variants with | - same handler
    CrawlEvent::WebsiteIngested { website_id, job_id, .. } |
    CrawlEvent::WebsitePostsRegenerated { website_id, job_id, .. } => |ctx| async move {
        ctx.deps().jobs.enqueue(ExtractPostsJob {
            website_id,
            parent_job_id: job_id,
        }).await?;
        Ok(CrawlEvent::ExtractJobEnqueued { website_id })
    },

    // Single variant
    CrawlEvent::PostsExtractedFromPages { website_id, posts, .. } => |ctx| async move {
        ctx.deps().jobs.enqueue(SyncPostsJob { website_id, posts }).await?;
        Ok(CrawlEvent::SyncJobEnqueued { website_id })
    },
};

// Add to engine
let engine = effects.into_iter().fold(engine, |e, eff| e.with_effect(eff));
```

#### Option B: `.extract().then()` (For Single Variant)

```rust
// Filter + extract fields in one step
effect::on::<OrderEvent>()
    .extract(|e| match e {
        OrderEvent::Placed { order_id, .. } => Some(*order_id),
        _ => None,
    })
    .then(|order_id, ctx| async move {
        ctx.deps().shipping_api.ship(order_id).await?;
        Ok(OrderEvent::Shipped { order_id })
    })
```

#### Option C: Observer (Dispatch Nothing)

```rust
// Return Ok(()) to dispatch nothing
effect::on::<OrderEvent>().then(|event, ctx| async move {
    ctx.deps().logger.log(&event);
    Ok(())  // No event dispatched
})

// Or observe ALL events
effect::on_any().then(|event, ctx| async move {
    ctx.deps().metrics.track(event.type_id);
    Ok(())
})
```

### Job Execution Modes

Jobs are accessed via `ctx.deps()` - effects decide execution mode:

```rust
// INLINE - do it now, return completion event
CrawlEvent::WebsiteIngestRequested { url, .. } => |ctx| async move {
    let result = ctx.deps().crawler.ingest(&url).await?;
    Ok(CrawlEvent::WebsiteIngested { website_id: result.id })
},

// ENQUEUE - return immediately, job worker emits completion later
CrawlEvent::ExtractPostsRequested { website_id, .. } => |ctx| async move {
    let job_id = ctx.deps().jobs.enqueue(ExtractPostsJob { website_id }).await?;
    Ok(CrawlEvent::ExtractJobEnqueued { website_id, job_id })
},

// SCHEDULE - run at specific time
CrawlEvent::SendDigestRequested { user_id, send_at, .. } => |ctx| async move {
    ctx.deps().jobs.schedule(SendDigestJob { user_id }, send_at).await?;
    Ok(CrawlEvent::DigestScheduled { user_id })
},
```

## Technical Considerations

### API Changes Summary

| 0.6.2 | 0.7.2 |
|-------|-------|
| `effect::on::<E>().run(\|event, ctx\| ...)` | `effect::on::<E>().extract(\|e\| ...).then(\|data, ctx\| ...)` |
| `ctx.emit(Event)` | `Ok(Event)` as return value |
| Hidden emission in actions | Effects return events, actions are pure |
| `engine.activate(state).process(\|ctx\| action(ctx))` | `handle.run(\|ctx\| Ok(Event))` or `handle.process(\|ctx\| async { Ok(Event) })` |
| N/A | `on! { Event::Variant { .. } => \|ctx\| async move { Ok(Event) } }` |
| N/A | `reducer::on::<E>().run(\|state, event\| new_state)` |
| N/A | `.transition(\|prev, next\| bool)` - state change guards |

### Key Architectural Shifts

1. **`on!` macro for multi-variant** - Clean match-like syntax, returns `Vec<Effect>`
2. **`.extract()` replaces `.filter_map()`** - Filter + extract fields in one step
3. **Effects return events** - `Ok(Event)` to dispatch, `Ok(())` for nothing
4. **Reducers for state** - Pure state transforms run before effects
5. **Jobs via deps** - `ctx.deps().jobs.enqueue()` - effects decide execution mode
6. **State access** - `ctx.prev_state()`, `ctx.next_state()`, `ctx.curr_state()`

### Breaking Changes

1. **`ctx.emit()` removed** - All 49 call sites must return events instead
2. **`.run()` → `.then()`** - Different callback signature
3. **`.filter_map()` → `.extract()`** - Renamed method
4. **Actions decouple from EffectContext** - Take `&ServerDeps` directly
5. **Entry point changes** - `handle.run(\|ctx\| Ok(Event))` + `handle.settled().await`

## Acceptance Criteria

### Phase 1: Infrastructure & Simple Domains

- [ ] Update `seesaw_core` dependency to `0.7.2` in `packages/server/Cargo.toml`
- [ ] Update `Engine` setup in `server/app.rs` to new builder pattern
- [ ] Migrate `auth` domain (2 files, 3 emit calls) - simplest, terminal events only
- [ ] Migrate `providers` domain (1 file, 6 emit calls) - CRUD, terminal events only
- [ ] Migrate `website` domain (1 file, 4 emit calls) - CRUD, terminal events only
- [ ] All tests pass for migrated domains

### Phase 2: Single-Cascade Domains

- [ ] Migrate `member` domain (3 files, 4 emit calls) - one cascade: MemberRegistered → EmbeddingGenerated
- [ ] Migrate `website_approval` domain (3 files, 3 emit calls) - linear cascade: ResearchCreated → SearchesCompleted → AssessmentCompleted
- [ ] All tests pass for migrated domains

### Phase 3: Multi-Cascade Domains

- [ ] Migrate `posts` domain (8 files, 20 emit calls) - complex cascade with branches
- [ ] Migrate `chatrooms` domain (2 files, 5 emit calls) - agent response cascade
- [ ] All tests pass for migrated domains

### Phase 4: Job System & Crawling

- [ ] Migrate `crawling` domain (4 files, 6 emit calls) - job executors + cascades
- [ ] Refactor job executors to return events instead of emitting
- [ ] All tests pass for migrated domains
- [ ] Full integration test passes

### Quality Gates

- [ ] `cargo check --package server` compiles without warnings
- [ ] All effect handlers use `on!` macro or `.extract().then()` pattern
- [ ] No `ctx.emit()` calls remain in codebase
- [ ] All event chains explicitly declared in effect registration
- [ ] CLAUDE.md updated with 0.7.2 patterns

## Success Metrics

- All 49 `ctx.emit()` calls replaced with returned events
- 8 effect registration files migrated to new pattern
- Event flow is traceable by reading effect registrations alone
- No hidden side effects in action functions

## Dependencies & Risks

### Dependencies

- Seesaw 0.7.2 crate published and available (crcn/seesaw-rs)
- Job system accessible via `ctx.deps().jobs`

### Risks

| Risk | Mitigation |
|------|------------|
| Job executors need restructure | Effects build events from job results, executors return data only |
| Multiple events from single handler | Use `on!` macro with `\|` for multiple variant triggers |
| No-op for unhandled events | Unmatched variants in `on!` pass through (no chain break) |
| Fan-out convergence | Use `.transition(\|prev, next\|)` guards with reducer state |

## Implementation Details

### Files to Modify by Domain

#### Auth Domain (Phase 1)

| File | Emit Calls | Pattern |
|------|------------|---------|
| `domains/auth/actions/send_otp.rs` | 2 | Terminal → return event |
| `domains/auth/actions/verify_otp.rs` | 1 | Terminal → return event |

#### Providers Domain (Phase 1)

| File | Emit Calls | Pattern |
|------|------------|---------|
| `domains/providers/actions/mutations.rs` | 6 | Terminal CRUD → return events |

#### Website Domain (Phase 1)

| File | Emit Calls | Pattern |
|------|------------|---------|
| `domains/website/actions/mod.rs` | 4 | Terminal CRUD → return events |

#### Member Domain (Phase 2)

| File | Emit Calls | Pattern |
|------|------------|---------|
| `domains/member/actions/register_member.rs` | 2 | Cascade: MemberRegistered |
| `domains/member/actions/update_status.rs` | 1 | Terminal |
| `domains/member/effects/mod.rs` | 1 | Chain: → EmbeddingGenerated |

#### Website Approval Domain (Phase 2)

| File | Emit Calls | Pattern |
|------|------------|---------|
| `domains/website_approval/actions/mod.rs` | 1 | Cascade start |
| `domains/website_approval/effects/search.rs` | 1 | Chain link |
| `domains/website_approval/effects/assessment.rs` | 1 | Terminal |

#### Posts Domain (Phase 3)

| File | Emit Calls | Pattern |
|------|------------|---------|
| `domains/posts/actions/core.rs` | 10 | Mixed terminal + cascade |
| `domains/posts/actions/deduplication.rs` | 2 | Conditional emission |
| `domains/posts/actions/reports.rs` | 3 | Terminal |
| `domains/posts/actions/scraping.rs` | 2 | Cascade start |
| `domains/posts/effects/scraper.rs` | 1 | Chain link |
| `domains/posts/effects/ai.rs` | 1 | Chain link |
| `domains/posts/effects/post.rs` | 1 | Chain link |
| `domains/posts/effects/composite.rs` | 0 | Rewrite entirely |

#### Chatrooms Domain (Phase 3)

| File | Emit Calls | Pattern |
|------|------------|---------|
| `domains/chatrooms/actions/entry_points.rs` | 3 | Mixed terminal + cascade |
| `domains/chatrooms/effects/handlers.rs` | 2 | Agent response chain |

#### Crawling Domain (Phase 4)

| File | Emit Calls | Pattern |
|------|------------|---------|
| `domains/crawling/actions/mod.rs` | 2 | Cascade start |
| `domains/crawling/actions/ingest_website.rs` | 1 | Cascade start |
| `domains/crawling/jobs/executor.rs` | 2 | Job completion events |
| `domains/crawling/effects/handlers.rs` | 1 | Chain link |

### Migration Pattern: Terminal Events

**Before (0.6.2):**
```rust
pub async fn approve_post(
    post_id: Uuid,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Post> {
    post_operations::update_post_status(post_id, "active", &ctx.deps().db_pool).await?;
    ctx.emit(PostEvent::PostApproved { post_id });
    Post::find_by_id(post_id, &ctx.deps().db_pool).await
}
```

**After (0.7.2):**
```rust
// Action becomes pure - no emit, takes deps directly
pub async fn approve_post(post_id: Uuid, deps: &ServerDeps) -> Result<Post> {
    post_operations::update_post_status(post_id, "active", &deps.db_pool).await?;
    Post::find_by_id(post_id, &deps.db_pool).await
}

// GraphQL mutation returns event via handle.process()
let handle = engine.activate(State::default());
handle.process(|ctx| async {
    let post = approve_post(post_id, ctx.deps()).await?;
    Ok(PostEvent::PostApproved { post_id: post.id })
}).await?;
```

### Migration Pattern: Cascading Events with `on!` Macro

**Before (0.6.2):**
```rust
// Composite effect with match arms and hidden emits
effect::on::<CrawlEvent>().run(|event: Arc<CrawlEvent>, ctx| async move {
    match event.as_ref() {
        CrawlEvent::WebsiteIngested { website_id, job_id, .. } => {
            handle_enqueue_extract_posts(*website_id, *job_id, &ctx).await
        }
        CrawlEvent::PostsExtractedFromPages { website_id, posts, .. } => {
            handle_enqueue_sync_posts(*website_id, posts.clone(), &ctx).await
        }
        _ => Ok(()),
    }
})
```

**After (0.7.2):**
```rust
use seesaw_core::on;

// on! macro - clean match-like syntax, returns Vec<Effect>
let effects = on! {
    // Multiple triggers with | syntax
    CrawlEvent::WebsiteIngested { website_id, job_id, .. } |
    CrawlEvent::WebsitePostsRegenerated { website_id, job_id, .. } => |ctx| async move {
        ctx.deps().jobs.enqueue(ExtractPostsJob { website_id, parent_job_id: job_id }).await?;
        Ok(CrawlEvent::ExtractJobEnqueued { website_id })
    },

    CrawlEvent::PostsExtractedFromPages { website_id, posts, .. } => |ctx| async move {
        ctx.deps().jobs.enqueue(SyncPostsJob { website_id, posts }).await?;
        Ok(CrawlEvent::SyncJobEnqueued { website_id })
    },

    // Unmatched variants (PostsSynced, etc.) pass through - no handler needed
};

// Add to engine
let engine = effects.into_iter().fold(engine, |e, eff| e.with_effect(eff));
```

### Migration Pattern: Job Executors

**Before (0.6.2):**
```rust
// Job executor takes EffectContext and emits
pub async fn execute_sync_posts_job(
    job: SyncPostsJob,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<JobExecutionResult> {
    let result = sync_posts(job.website_id, job.posts, ctx.deps()).await?;
    ctx.emit(CrawlEvent::PostsSynced { website_id, count: result.inserted });
    Ok(JobExecutionResult { job_id, status: "succeeded" })
}
```

**After (0.7.2):**
```rust
// Job executor returns data only - no emit, takes deps directly
pub async fn execute_sync_posts_job(
    job: SyncPostsJob,
    deps: &ServerDeps,
) -> Result<SyncJobResult> {
    let result = sync_posts(job.website_id, job.posts, deps).await?;
    Ok(SyncJobResult {
        job_id: job.id,
        website_id: job.website_id,
        inserted: result.inserted,
        updated: result.updated,
    })
}

// Effect builds event from job result
CrawlEvent::SyncJobEnqueued { website_id, job_id } => |ctx| async move {
    let job = SyncPostsJob::find(job_id, ctx.deps()).await?;
    let result = execute_sync_posts_job(job, ctx.deps()).await?;
    Ok(CrawlEvent::PostsSynced {
        website_id: result.website_id,
        new_count: result.inserted,
        updated_count: result.updated,
    })
},
```

### Engine Setup Changes

**Before (0.6.2):**
```rust
fn build_engine(server_deps: ServerDeps) -> AppEngine {
    Engine::with_deps(server_deps)
        .with_effect(auth_effect())
        .with_effect(crawler_effect())
        // ...
}

// GraphQL mutation
engine.activate(app_state).process(|ctx| {
    actions::ingest_website(website_id, &ctx)
}).await
```

**After (0.7.2):**
```rust
use seesaw_core::{on, reducer, Engine};

fn build_engine(server_deps: ServerDeps) -> AppEngine {
    let mut engine = Engine::new().with_deps(server_deps);

    // Reducers - pure state transforms (run before effects)
    engine = engine.with_reducer(reducer::on::<CrawlEvent>().run(|state, event| {
        match event {
            CrawlEvent::PostsSynced { new_count, .. } => State {
                total_posts: state.total_posts + new_count,
                ..state
            },
            _ => state.clone(),
        }
    }));

    // Effects via on! macro
    let effects = on! {
        CrawlEvent::WebsiteIngested { website_id, job_id, .. } => |ctx| async move {
            ctx.deps().jobs.enqueue(ExtractPostsJob { website_id, parent_job_id: job_id }).await?;
            Ok(CrawlEvent::ExtractJobEnqueued { website_id })
        },
        // ... more handlers
    };

    effects.into_iter().fold(engine, |e, eff| e.with_effect(eff))
}

// GraphQL mutation - return event to trigger chain
let handle = engine.activate(State::default());
handle.process(|ctx| async {
    let result = ctx.deps().crawler.ingest(&url).await?;
    Ok(CrawlEvent::WebsiteIngested { website_id: result.id, job_id: result.job_id })
}).await?;
// Chain runs: WebsiteIngested → ExtractJobEnqueued → ... → PostsSynced
```

## Future Considerations

1. **Reducers for workflow state** - Track job progress, counts, completion status
2. **Transition guards** - `.transition(|prev, next| bool)` for fan-out convergence
3. **Durable outbox** - Use `seesaw-outbox` crate for events that must survive crashes
4. **Testing** - Use `Engine` directly in tests with mock deps

## References & Research

### Seesaw 0.7.2 Documentation

- **GitHub**: `crcn/seesaw-rs`
- **Key APIs**:
  - `on!` macro - multi-variant event matching
  - `effect::on::<E>().extract().then()` - single variant handling
  - `effect::on_any().then()` - observe all events
  - `reducer::on::<E>().run()` - pure state transforms
  - `.transition(|prev, next| bool)` - state change guards
  - `handle.run()` / `handle.process()` - entry points
  - `handle.settled().await` - wait for chain completion

### Internal References

- Previous upgrade: `docs/plans/2026-02-02-refactor-upgrade-seesaw-to-0.6.0-plan.md`
- Architecture guide: `docs/plans/2026-02-01-refactor-untangle-seesaw-architecture-plan.md`
- Event chain cleanup: `docs/plans/2026-02-03-refactor-event-effect-chain-architecture-plan.md`
- Current engine setup: `packages/server/src/server/app.rs:84-102`
- Effect examples: `packages/server/src/domains/posts/effects/composite.rs`

### Files with ctx.emit() Calls (All 24)

```
packages/server/src/domains/auth/actions/send_otp.rs (2)
packages/server/src/domains/auth/actions/verify_otp.rs (1)
packages/server/src/domains/chatrooms/actions/entry_points.rs (3)
packages/server/src/domains/chatrooms/effects/handlers.rs (2)
packages/server/src/domains/crawling/actions/ingest_website.rs (1)
packages/server/src/domains/crawling/actions/mod.rs (2)
packages/server/src/domains/crawling/effects/handlers.rs (1)
packages/server/src/domains/crawling/jobs/executor.rs (2)
packages/server/src/domains/member/actions/register_member.rs (2)
packages/server/src/domains/member/actions/update_status.rs (1)
packages/server/src/domains/member/effects/mod.rs (1)
packages/server/src/domains/posts/actions/core.rs (10)
packages/server/src/domains/posts/actions/deduplication.rs (2)
packages/server/src/domains/posts/actions/reports.rs (3)
packages/server/src/domains/posts/actions/scraping.rs (2)
packages/server/src/domains/posts/effects/ai.rs (1)
packages/server/src/domains/posts/effects/post.rs (1)
packages/server/src/domains/posts/effects/scraper.rs (1)
packages/server/src/domains/providers/actions/mutations.rs (6)
packages/server/src/domains/website/actions/mod.rs (4)
packages/server/src/domains/website_approval/actions/mod.rs (1)
packages/server/src/domains/website_approval/effects/assessment.rs (1)
packages/server/src/domains/website_approval/effects/search.rs (1)
```
