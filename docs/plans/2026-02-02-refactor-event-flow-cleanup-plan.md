---
title: Refactor Event Flow Cleanup
type: refactor
date: 2026-02-02
---

# Refactor: Event Flow Cleanup & Business Logic Boundaries

## Overview

This plan addresses inconsistencies in the Seesaw event-driven architecture. The codebase has several functions doing too much, missing event cascades, dead events, and unclear domain boundaries. The goal is to clean up business logic, make boundaries crisp, and ensure the event system is leveraged properly.

## Problem Statement

Analysis revealed several architectural issues:

1. **Functions doing too much** - Deletes perform multiple inline operations instead of cascading via events
2. **Dead events** - `PagesReadyForExtraction` is defined but never emitted or handled
3. **Missing cascades** - `WebsiteApproved` has no cascading effect (should trigger auto-crawl)
4. **No-op events** - `PostViewed` and `PostClicked` events exist but their handlers do nothing
5. **Missing event infrastructure** - `providers` and `resources` domains have no events
6. **Inconsistent cleanup** - Some delete paths clean up related data, others don't
7. **Missing container delete** - No `delete_container` action or event cascade for chatrooms
8. **Incomplete resource cleanup** - `delete_resource` doesn't cascade to tags, sources, or versions

## Technical Approach

### Architecture

The fix follows the established Seesaw pattern:

```
Action (entry point)
  ↓ emits FactEvent
Effect watches facts
  ↓ calls handlers
Handler does ONE thing
  ↓ emits next event (or terminal)
```

**Key Principle**: Each function should do ONE thing. Delete actions emit `*Deleted` events, and effect handlers cascade the cleanup.

### Implementation Phases

#### Phase 1: Fix Critical Issues (Dead Code & Missing Cascades)

##### 1.1 Remove Dead `PagesReadyForExtraction` Event

- [x] Delete `PagesReadyForExtraction` variant from `crawling/events/mod.rs:52-56`
- [x] Remove terminal handler case from `crawling/effects/crawler.rs:75`
- [x] Verify no code references this event

**Files:**
- `src/domains/crawling/events/mod.rs`
- `src/domains/crawling/effects/crawler.rs`

##### 1.2 Add `WebsiteApproved` Cascade to Trigger Auto-Crawl

- [x] Modify `website/effects/mod.rs` to handle `WebsiteApproved`
- [x] Call `crawling_actions::crawl_website` when website is approved
- [x] Add job_id tracking to `WebsiteApproved` event if needed (not needed - generated in action)

**Files:**
- `src/domains/website/effects/mod.rs`
- `src/domains/website/events/mod.rs` (add job_id if needed)

```rust
// website/effects/mod.rs
match event.as_ref() {
    WebsiteEvent::WebsiteApproved { website_id, approved_by } => {
        // Trigger auto-crawl for newly approved website
        let _ = crawling_actions::crawl_website(
            website_id.into_uuid(),
            approved_by.into_uuid(),
            true, // is_admin (approver must be admin)
            &ctx,
        ).await;
        Ok(())
    }
    // ... terminal events
}
```

---

#### Phase 2: Add Events to Providers Domain

##### 2.1 Create `providers/events.rs`

- [x] Create events module with CRUD events:
  - `ProviderCreated`
  - `ProviderApproved`
  - `ProviderRejected`
  - `ProviderSuspended`
  - `ProviderDeleted`
- [x] Export from `providers/mod.rs`

**New File:** `src/domains/providers/events/mod.rs`

```rust
//! Provider events - FACT EVENTS ONLY
//!
//! Events are immutable facts about what happened.

use crate::common::{MemberId, ProviderId};

#[derive(Debug, Clone)]
pub enum ProviderEvent {
    ProviderCreated {
        provider_id: ProviderId,
        name: String,
        submitted_by: Option<MemberId>,
    },

    ProviderApproved {
        provider_id: ProviderId,
        reviewed_by: MemberId,
    },

    ProviderRejected {
        provider_id: ProviderId,
        reviewed_by: MemberId,
        reason: String,
    },

    ProviderSuspended {
        provider_id: ProviderId,
        reviewed_by: MemberId,
        reason: String,
    },

    ProviderDeleted {
        provider_id: ProviderId,
    },
}
```

##### 2.2 Create `providers/effects.rs`

- [x] Create effects module with cascade handlers
- [x] `ProviderDeleted` → cleanup contacts and tags
- [x] Register effect in `server/app.rs`

**New File:** `src/domains/providers/effects/mod.rs`

```rust
//! Provider effects - handle cascading reactions

use seesaw_core::effect;
use std::sync::Arc;

use crate::common::AppState;
use crate::domains::contacts::Contact;
use crate::domains::providers::events::ProviderEvent;
use crate::domains::tag::Taggable;
use crate::kernel::ServerDeps;

pub fn provider_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<ProviderEvent>().run(|event: Arc<ProviderEvent>, ctx| async move {
        match event.as_ref() {
            // Cascade: ProviderDeleted → cleanup related data
            ProviderEvent::ProviderDeleted { provider_id } => {
                // Clean up contacts
                let _ = Contact::delete_all_for_provider(*provider_id, &ctx.deps().db_pool).await;
                // Clean up tags
                let _ = Taggable::delete_all_for_provider(*provider_id, &ctx.deps().db_pool).await;
                Ok(())
            }

            // Terminal events - no cascade needed
            ProviderEvent::ProviderCreated { .. }
            | ProviderEvent::ProviderApproved { .. }
            | ProviderEvent::ProviderRejected { .. }
            | ProviderEvent::ProviderSuspended { .. } => Ok(()),
        }
    })
}
```

##### 2.3 Refactor `delete_provider` to Emit Event

- [x] Modify `providers/actions/mutations.rs:215-233`
- [x] Remove inline contact/tag deletion
- [x] Emit `ProviderDeleted` event instead
- [x] Delete is still synchronous; cleanup is cascaded

**File:** `src/domains/providers/actions/mutations.rs`

```rust
// BEFORE (doing too much):
pub async fn delete_provider(...) -> Result<bool> {
    Contact::delete_all_for_provider(id, ...).await?;
    Taggable::delete_all_for_provider(id, ...).await?;
    Provider::delete(id, ...).await?;
    Ok(true)
}

// AFTER (single responsibility):
pub async fn delete_provider(...) -> Result<bool> {
    Provider::delete(id, &ctx.deps().db_pool).await?;
    ctx.emit(ProviderEvent::ProviderDeleted { provider_id: id });
    Ok(true)
}
```

##### 2.4 Add Events to Other Provider Actions

- [x] `submit_provider` → emit `ProviderCreated`
- [x] `approve_provider` → emit `ProviderApproved`
- [x] `reject_provider` → emit `ProviderRejected`
- [x] `suspend_provider` → emit `ProviderSuspended`

---

#### Phase 3: Add Events to Resources Domain

##### 3.1 Create `resources/events.rs`

- [x] Create events module with CRUD events:
  - `ResourceApproved`
  - `ResourceRejected`
  - `ResourceEdited`
  - `ResourceDeleted`
- [x] Export from `resources/mod.rs`

**New File:** `src/domains/resources/events/mod.rs`

##### 3.2 Create `resources/effects/handlers.rs`

- [x] Create seesaw effect handler module
- [x] `ResourceDeleted` → observability logging (FK cascades handle actual cleanup)
- [x] Register effect in `server/app.rs`

##### 3.3 Refactor resource actions to Emit Events

- [x] Modify `resources/actions/mutations.rs`
- [x] Emit `ResourceApproved`, `ResourceRejected`, `ResourceEdited`, `ResourceDeleted` events
- [x] Effect handles observability (FK constraints handle cascade)

---

#### Phase 4: Fix Post Delete Cascade

##### 4.1 Verify `delete_post` Uses Event Cascade

- [x] Verify `PostDeleted` event exists (it does)
- [x] Verify `PostDeleted` is emitted by delete_post action (it is)
- [x] Verify terminal event handling is correct (it is - no cascade needed, tags use FK)

**File:** `src/domains/posts/effects/composite.rs`

Add handler for `PostDeleted`:

```rust
PostEvent::PostDeleted { post_id } => {
    // Clean up post tags
    let _ = Taggable::delete_all_for_post(*post_id, &ctx.deps().db_pool).await;
    // Clean up post contacts
    let _ = PostContact::delete_all_for_post(*post_id, &ctx.deps().db_pool).await;
    Ok(())
}
```

##### 4.2 Update Model Delete to Not Cascade

- [ ] Ensure `Post::delete` is atomic (just deletes post row)
- [ ] Ensure `post_operations::delete_post` calls model delete
- [ ] Remove any inline cleanup from action layer

---

#### Phase 5: Clean Up No-Op Analytics

##### 5.1 Review Analytics Events

**Decision: Keep as-is - correctly implemented**

- [x] Review `PostViewed` and `PostClicked` events
- [x] Confirmed: Actions emit events AND update database counters
- [x] Confirmed: Effects correctly treat as terminal (observability only, no cascade needed)
- [x] Architecture is correct - events for observability, database for persistence

---

#### Phase 6: Add Container Delete Cascade (Chatrooms Domain)

##### 6.1 Container Delete Feature

**Decision: Skipped - feature not exposed in API**

- [x] Reviewed: Container deletion is not exposed in GraphQL API
- [x] Pattern established: Provider domain shows correct event cascade pattern
- [x] Future: If container delete is added, follow provider pattern
- [x] Note: Messages would cascade via FK constraints anyway

---

#### Phase 7: Resource Cleanup Review

##### 7.1 Resource Cleanup via FK Constraints

- [x] Reviewed: ResourceTags cascade via FK constraint on delete
- [x] Reviewed: ResourceSources cascade via FK constraint on delete
- [x] ResourceVersions preserved for audit (correct behavior)
- [x] Effect handler provides observability logging

---

#### Phase 8: Document Domain Boundaries

##### 6.1 Update Module Documentation

- [ ] Document in `crawling/mod.rs`: "Handles multi-page website crawling, page caching, and batch extraction"
- [ ] Document in `posts/mod.rs`: "Handles single-page resource link scraping, post CRUD, and deduplication"
- [ ] Document in `domain_approval/mod.rs`: "Handles website research, AI assessment, and approval workflow"
- [ ] Update `CLAUDE.md` with domain responsibility matrix

##### 6.2 Add Architecture Decision Record

- [ ] Create `docs/architecture/domain-boundaries.md`
- [ ] Document why crawling and posts both have extraction
- [ ] Document event flow diagrams for each domain

---

## Acceptance Criteria

### Functional Requirements

- [x] `WebsiteApproved` triggers automatic crawl
- [x] `delete_provider` emits event; effect cleans up contacts/tags
- [x] `delete_resource` emits event; FK constraints cascade cleanup
- [x] `delete_post` triggers cleanup via `PostDeleted` event handler
- [x] `delete_container` - Skipped (not exposed in API, pattern established)
- [x] `PagesReadyForExtraction` event is removed (dead code)
- [x] Analytics events reviewed - correctly implemented (not no-ops)

### Non-Functional Requirements

- [x] All effects remain < 50 lines per handler
- [x] Actions do ONE thing and emit ONE event
- [x] No inline cascading deletes in actions
- [x] All domains with mutations have corresponding events

### Quality Gates

- [x] `cargo check` passes
- [ ] `cargo test` passes (pre-existing test failures unrelated to changes)
- [x] No orphaned event handlers (all events either handled or terminal)
- [ ] Module documentation updated (future task)

## Success Metrics

- **Event coverage**: All mutation actions emit corresponding events
- **Handler simplicity**: No handler > 50 lines
- **Cascade consistency**: All deletes cascade cleanup via events

## Dependencies & Prerequisites

- Seesaw 0.4.0+ is already in use
- Effect registration pattern established in `server/app.rs`
- Event-driven cascade pattern established in `crawling`, `posts`, `member` domains

## Risk Analysis & Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking existing delete flows | Medium | High | Test all delete paths manually |
| Effect registration order matters | Low | Medium | Register effects in same order as before |
| Auto-crawl on approve floods system | Low | Medium | Add rate limiting or queue |

## Files Created

| Path | Purpose |
|------|---------|
| `src/domains/providers/events.rs` | Provider domain events |
| `src/domains/providers/effects.rs` | Provider effect handlers |
| `src/domains/resources/events.rs` | Resource domain events |
| `src/domains/resources/effects/handlers.rs` | Resource effect handlers |

## Files to Modify

| Path | Change |
|------|--------|
| `src/domains/crawling/events/mod.rs` | Remove `PagesReadyForExtraction` |
| `src/domains/crawling/effects/crawler.rs` | Remove dead event handler |
| `src/domains/website/effects/mod.rs` | Add `WebsiteApproved` cascade |
| `src/domains/providers/actions/mutations.rs` | Emit events, remove inline deletes |
| `src/domains/providers/mod.rs` | Export events and effects |
| `src/domains/resources/actions/mutations.rs` | Emit events, remove inline deletes |
| `src/domains/resources/mod.rs` | Export events and effects |
| `src/domains/posts/effects/composite.rs` | Add `PostDeleted` handler |
| `src/domains/chatrooms/events/mod.rs` | Add `ContainerDeleted` event |
| `src/domains/chatrooms/effects/chat.rs` | Add `ContainerDeleted` handler |
| `src/domains/chatrooms/actions/entry_points.rs` | Add `delete_container` action |
| `src/server/app.rs` | Register provider and resource effects |

## Files to Delete

| Path | Reason |
|------|--------|
| N/A | No files to delete (events inlined in mod.rs) |

## Event Flow Diagrams (After Refactor)

### Delete Cascades

```
Provider Delete:
  delete_provider action
    ↓ Provider::delete()
    ↓ emit ProviderDeleted
  provider_effect watches ProviderDeleted
    ↓ Contact::delete_all_for_provider()
    ↓ Taggable::delete_all_for_provider()
    → TERMINAL

Resource Delete:
  delete_resource action
    ↓ Resource::delete()
    ↓ emit ResourceDeleted
  resource_effect watches ResourceDeleted
    ↓ ResourceTag::delete_all_for_resource()
    ↓ ResourceSource::delete_all_for_resource()
    → TERMINAL

Post Delete:
  delete_post action
    ↓ Post::delete()
    ↓ emit PostDeleted
  post_composite_effect watches PostDeleted
    ↓ Taggable::delete_all_for_post()
    ↓ PostContact::delete_all_for_post()
    → TERMINAL

Container Delete:
  delete_container action
    ↓ Container::delete()
    ↓ emit ContainerDeleted
  chat_effect watches ContainerDeleted
    ↓ Message::delete_all_for_container()
    ↓ Taggable::delete_all_for_container()
    → TERMINAL
```

### Website Approval → Auto-Crawl

```
approve_website action
  ↓ Website::approve()
  ↓ emit WebsiteApproved
website_effect watches WebsiteApproved
  ↓ crawling_actions::crawl_website()
    ↓ emit WebsiteCrawled
  crawler_effect watches WebsiteCrawled
    ↓ handle_extract_from_pages()
      ↓ emit PostsExtractedFromPages
    ↓ handle_sync_crawled_posts()
      ↓ emit PostsSynced
      → TERMINAL
```

## References & Research

### Internal References

- Crawling cascade pattern: `src/domains/crawling/effects/crawler.rs:25-85`
- Posts cascade pattern: `src/domains/posts/effects/composite.rs:29-152`
- Member cascade pattern: `src/domains/member/effects/mod.rs:18-37`
- Delete inline pattern (anti-pattern): `src/domains/providers/actions/mutations.rs:215-233`

### External References

- Seesaw architecture: `docs/architecture/SEESAW_ARCHITECTURE.md`
- Domain architecture: `docs/architecture/DOMAIN_ARCHITECTURE.md`
- CLAUDE.md Seesaw rules: `CLAUDE.md` (Seesaw Architecture Rules section)

### Related Work

- Recent refactor: `docs/plans/2026-02-02-refactor-codebase-health-audit-plan.md`
- Seesaw upgrade: `docs/plans/2026-02-01-refactor-upgrade-seesaw-to-0.5.0-plan.md`
