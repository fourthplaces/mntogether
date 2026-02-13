---
title: Architectural Audit - SOLID Compliance & Technical Debt Cleanup
type: refactor
date: 2026-02-01
---

# Architectural Audit - SOLID Compliance & Technical Debt Cleanup

## Overview

Comprehensive architectural audit of the Rust server codebase identifying SOLID principle violations, anti-patterns, technical debt, and providing a prioritized remediation plan. The codebase follows a well-designed DDD/event-driven architecture with seesaw-rs, but has accumulated technical debt requiring attention.

## Executive Summary

| Principle | Status | Key Issue |
|-----------|--------|-----------|
| **Single Responsibility** | ⚠️ Violations | MatchingEffect fat, PostMachine god object |
| **Open/Closed** | ✅ Compliant | Good composite pattern usage |
| **Liskov Substitution** | ✅ Compliant | Trait objects properly used |
| **Interface Segregation** | ✅ Compliant | Small, focused traits |
| **Dependency Inversion** | ⚠️ Partial | ServerDeps location, TwilioService concrete |

**Total Issues Found:** 7 architectural issues, 10 TODO items, 1,026 terminology inconsistencies

---

## Problem Statement

The codebase has accumulated technical debt during rapid development:

1. **MatchingEffect** contains 170+ lines of inline business logic, violating the CLAUDE.md rule that effects must be thin orchestration layers
2. **PostMachine** is a god object with 563 lines handling 50+ event variants across 7 different workflows
3. **ServerDeps** is defined in `posts` domain but used by all domains, creating unnecessary coupling
4. **TwilioService** is concrete in ServerDeps while a `BaseTwilioService` trait exists but is unused
5. **Listing/Post terminology** migration is incomplete (1,026 occurrences of "listing" across 53 files)
6. **Stale code** exists (duplicate ServerDeps in organization domain, test harness imports dead module)
7. **10 TODO comments** represent unfinished work

---

## Proposed Solution

A phased remediation approach, ordered from lowest to highest risk:

### Phase 1: Pre-Refactor Cleanup (Zero Risk)
Clean up stale code before making structural changes.

### Phase 2: ServerDeps Migration (Medium Risk)
Move shared infrastructure to proper location.

### Phase 3: MatchingEffect Refactor (Medium Risk)
Apply thin orchestration pattern.

### Phase 4: Terminology Migration (Medium Risk, Large Scope)
Complete listing→post rename.

### Phase 5: PostMachine Decomposition (High Risk)
Break god object into focused machines.

---

## Technical Approach

### Phase 1: Pre-Refactor Cleanup

**1.1 Delete Stale organization/effects/deps.rs**

There are two `ServerDeps` definitions:
- `packages/server/src/domains/posts/effects/deps.rs` (canonical - has search_service, pii_detector)
- `packages/server/src/domains/organization/effects/deps.rs` (stale - missing fields)

```rust
// DELETE: packages/server/src/domains/organization/effects/deps.rs
// This file is stale and should be removed
```

**1.2 Update Test Harness Imports**

The test harness still imports from dead `listings` module:

```rust
// File: packages/server/tests/common/harness.rs
// BEFORE:
use server_core::domains::listings::{
    commands::ListingCommand,
    effects::{ListingCompositeEffect, ServerDeps},
    machines::ListingMachine,
};

// AFTER:
use server_core::domains::posts::{
    commands::PostCommand,
    effects::{PostCompositeEffect, ServerDeps},
    machines::PostMachine,
};
```

---

### Phase 2: ServerDeps Migration

**2.1 Create New Location**

```rust
// NEW FILE: packages/server/src/kernel/deps.rs
use crate::kernel::{
    BaseAI, BaseEmbeddingService, BasePiiDetector,
    BasePushNotificationService, BaseSearchService,
    BaseTwilioService, BaseWebScraper,
};
use sqlx::PgPool;
use std::sync::Arc;

pub struct ServerDeps {
    pub db_pool: PgPool,
    pub web_scraper: Arc<dyn BaseWebScraper>,
    pub ai: Arc<dyn BaseAI>,
    pub embedding_service: Arc<dyn BaseEmbeddingService>,
    pub push_service: Arc<dyn BasePushNotificationService>,
    pub twilio: Arc<dyn BaseTwilioService>,  // Changed from concrete
    pub search_service: Arc<dyn BaseSearchService>,
    pub pii_detector: Arc<dyn BasePiiDetector>,
    pub test_identifier_enabled: bool,
    pub admin_identifiers: Vec<String>,
}
```

**2.2 Update Module Exports**

```rust
// File: packages/server/src/kernel/mod.rs
pub mod deps;
pub use deps::ServerDeps;
```

**2.3 Update All Import Paths (20 files)**

| File | Change |
|------|--------|
| `domains/posts/effects/mod.rs` | Remove deps module, re-export from kernel |
| `domains/matching/effects/mod.rs` | `use crate::kernel::ServerDeps;` |
| `domains/chatrooms/effects/*.rs` | `use crate::kernel::ServerDeps;` |
| `domains/member/effects/mod.rs` | `use crate::kernel::ServerDeps;` |
| `domains/domain_approval/effects/*.rs` | `use crate::kernel::ServerDeps;` |
| `server/app.rs` | `use crate::kernel::ServerDeps;` |

---

### Phase 3: MatchingEffect Refactor

**Current State (Violation):**
```rust
// File: packages/server/src/domains/matching/effects/mod.rs
impl Effect<MatchingCommand, ServerDeps> for MatchingEffect {
    async fn execute(&self, cmd: MatchingCommand, ctx: EffectContext<ServerDeps>) -> Result<MatchingEvent> {
        match cmd {
            MatchingCommand::FindMatches { post_id } => {
                // 170 lines of inline business logic - VIOLATION
            }
        }
    }
}
```

**Target State (Compliant):**
```rust
// File: packages/server/src/domains/matching/effects/mod.rs
impl Effect<MatchingCommand, ServerDeps> for MatchingEffect {
    async fn execute(&self, cmd: MatchingCommand, ctx: EffectContext<ServerDeps>) -> Result<MatchingEvent> {
        match cmd {
            MatchingCommand::FindMatches { post_id } => {
                handle_find_matches(post_id, &ctx).await
            }
        }
    }
}

// ============================================================================
// Handler Functions (Business Logic)
// ============================================================================

async fn handle_find_matches(
    post_id: PostId,
    ctx: &EffectContext<ServerDeps>
) -> Result<MatchingEvent> {
    // All business logic moved here
}

async fn find_match_candidates(
    post: &Post,
    ctx: &EffectContext<ServerDeps>
) -> Result<Vec<MatchCandidate>> {
    // Vector search logic
}

async fn filter_and_notify(
    post: &Post,
    candidates: Vec<MatchCandidate>,
    ctx: &EffectContext<ServerDeps>
) -> Result<(usize, usize)> {
    // Relevance check + notification logic
}
```

---

### Phase 4: Terminology Migration

**4.1 Scope Assessment**

| Category | Count | Files |
|----------|-------|-------|
| Event variants | 18 | `posts/events/mod.rs` |
| Command variants | 15 | `posts/commands/mod.rs` |
| Function names | ~200 | Various |
| Comments | ~500 | Various |
| Error messages | ~50 | Various |
| Variable names | ~250 | Various |

**4.2 Migration Strategy**

Rename in order:
1. Types/Structs (least used)
2. Event/Command variants (moderate use)
3. Function names (high use)
4. Variables (highest use)
5. Comments (no runtime impact)

**4.3 Key Renames**

```rust
// Events (posts/events/mod.rs)
ListingApproved -> PostApproved
ListingCreated -> PostCreated
ListingRejected -> PostRejected
ListingsExtracted -> PostsExtracted
ListingsSynced -> PostsSynced

// Commands (posts/commands/mod.rs)
CreateListing -> CreatePost
DeleteListing -> DeletePost
UpdateListingAndApprove -> UpdatePostAndApprove
ExtractListings -> ExtractPosts
SyncListings -> SyncPosts
```

---

### Phase 5: PostMachine Decomposition

**5.1 Current State (God Object)**

The PostMachine handles 7 distinct workflows:
1. **Scraping Workflow** (lines 37-146)
2. **Post Management** (lines 156-214)
3. **Resource Link Submission** (lines 219-245)
4. **User Submission** (lines 249-283)
5. **Approval Workflows** (lines 288-385)
6. **Website Crawl** (lines 421-496)
7. **Regeneration** (lines 501-560)

Shared state: `pending_scrapes: HashSet<WebsiteId>`

**5.2 Target Architecture**

```
PostMachine (facade)
    |
    +-- ScrapeWorkflowMachine
    |       - handles: ScrapeSource*, ExtractListings*
    |       - owns: pending_scrapes for scrape operations
    |
    +-- CrawlWorkflowMachine
    |       - handles: CrawlWebsite*, PageScraped*, Regenerate*
    |       - owns: pending_crawls HashSet
    |
    +-- ApprovalWorkflowMachine
    |       - handles: Create*, Approve*, Reject*, Delete*
    |       - stateless
    |
    +-- SubmissionWorkflowMachine
            - handles: SubmitResourceLink*, UserSubmitListing*
            - stateless
```

**5.3 Shared State Coordination**

```rust
// File: packages/server/src/domains/posts/machines/coordinator.rs
pub struct PostWorkflowState {
    pub pending_scrapes: HashSet<WebsiteId>,
    pub pending_crawls: HashSet<WebsiteId>,
}

impl PostWorkflowState {
    pub fn start_scrape(&mut self, website_id: WebsiteId) -> bool {
        self.pending_scrapes.insert(website_id)
    }

    pub fn complete_scrape(&mut self, website_id: WebsiteId) {
        self.pending_scrapes.remove(&website_id);
    }
}
```

---

## Acceptance Criteria

### Phase 1
- [ ] `organization/effects/deps.rs` deleted
- [ ] Test harness compiles with updated imports
- [ ] All tests pass

### Phase 2
- [ ] `kernel/deps.rs` contains ServerDeps
- [ ] All 20 files updated to use `crate::kernel::ServerDeps`
- [ ] `twilio` field uses `Arc<dyn BaseTwilioService>`
- [ ] Application starts successfully
- [ ] All tests pass

### Phase 3
- [ ] MatchingEffect execute method < 20 lines
- [ ] Business logic in handler functions
- [ ] Matching still triggers on PostApproved event
- [ ] Push notifications sent correctly
- [ ] Notification records created

### Phase 4
- [ ] No occurrences of "listing" in event/command enums
- [ ] All function names use "post" terminology
- [ ] Comments updated
- [ ] Error messages consistent

### Phase 5
- [ ] PostMachine < 200 lines
- [ ] 4 focused workflow machines created
- [ ] Duplicate scrape prevention still works
- [ ] All 50+ events handled correctly
- [ ] MatchingCoordinatorMachine still triggers matching

---

## Dependencies & Prerequisites

- No external dependencies
- Migration file not needed (schema unchanged)
- Frontend unaffected (GraphQL types unchanged)

---

## Risk Analysis & Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Broken imports after ServerDeps move | High | Low | Run `cargo check` after each file update |
| PostMachine decomposition breaks event flow | Medium | High | Comprehensive integration tests before & after |
| Terminology migration misses occurrences | Medium | Low | Use `rg` to verify no "listing" remains |
| Test harness update incomplete | Low | Medium | Run full test suite |

---

## TODO Items (10 Total)

| File | Line | TODO | Priority |
|------|------|------|----------|
| `matching/effects/mod.rs` | 67 | Generate embedding if not exists | Medium |
| `matching/utils/relevance.rs` | 75 | In production, make AI call | High |
| `chatrooms/commands/mod.rs` | 78 | Switch to Background when job queue ready | Low |
| `chatrooms/effects/messaging.rs` | 68 | Check if author is admin | Medium |
| `chatrooms/effects/messaging.rs` | 79 | Look up or create agent member | Medium |
| `posts/effects/post_operations.rs` | 17 | Store submitted_by_member_id | Low |
| `posts/effects/utils/sync_utils.rs` | 255 | Detect content changes | Medium |
| `posts/effects/post.rs` | 619 | Store contact info | Low |
| `posts/machines/mod.rs` | 392 | Get created_by from context | Low |
| `schema.rs` | 360 | Get IP address from request context | Low |

---

## Implementation Order (Safest → Riskiest)

```
1. Delete stale deps.rs              [Zero risk - dead code removal]
2. Update test harness imports       [Required for any testing]
3. Move ServerDeps to kernel         [Foundation for other work]
4. Use BaseTwilioService trait       [Simple type change]
5. Refactor MatchingEffect           [Isolated domain, easy to test]
6. Complete terminology migration    [Large scope but low risk]
7. Decompose PostMachine             [Highest risk - do last]
```

---

## References

### Internal
- `CLAUDE.md` - Effect thin orchestration rules
- `docs/architecture/SEESAW_ARCHITECTURE.md` - Event-driven patterns
- `docs/architecture/DOMAIN_ARCHITECTURE.md` - Domain layering rules

### Files Modified

**Phase 1:**
- `packages/server/src/domains/organization/effects/deps.rs` (DELETE)
- `packages/server/tests/common/harness.rs`

**Phase 2:**
- `packages/server/src/kernel/mod.rs`
- `packages/server/src/kernel/deps.rs` (NEW)
- `packages/server/src/domains/posts/effects/deps.rs` (DELETE)
- `packages/server/src/domains/posts/effects/mod.rs`
- 18 other files updating imports

**Phase 3:**
- `packages/server/src/domains/matching/effects/mod.rs`

**Phase 4:**
- 53 files with terminology changes

**Phase 5:**
- `packages/server/src/domains/posts/machines/mod.rs`
- `packages/server/src/domains/posts/machines/coordinator.rs` (NEW)
- `packages/server/src/domains/posts/machines/scrape.rs` (NEW)
- `packages/server/src/domains/posts/machines/crawl.rs` (NEW)
- `packages/server/src/domains/posts/machines/approval.rs` (NEW)
- `packages/server/src/domains/posts/machines/submission.rs` (NEW)
