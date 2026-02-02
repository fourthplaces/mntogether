# Institutional Learnings & Historical Context

This document captures documented learnings, gotchas, and architectural decisions from the codebase. It serves as a knowledge base for avoiding repeated mistakes and understanding why certain patterns exist.

---

## 1. Seesaw Architecture Evolution (Critical Context)

### Current Status: Seesaw 0.3.0 Upgrade in Progress
- **Branch**: `refactor/seesaw-0.3.0-upgrade`
- **Previous Version**: 0.1.1 (with Machines + Commands)
- **Target Version**: 0.3.0 (simplified Event → Effect → Event flow)

### Major Architecture Changes (0.1.1 → 0.3.0)

| Aspect | 0.1.1 | 0.3.0 | Why Changed |
|--------|-------|-------|------------|
| **Input to Effects** | Command | Event | Simpler mental model |
| **Effect Self** | `&self` | `&mut self` | Enable in-effect state tracking |
| **Method** | `execute()` | `handle()` | Clearer semantic for event handling |
| **Routing Layer** | Machines | Direct registration | Reduced boilerplate |
| **Entry Points** | Functions + EventBus | Edges trait | Type-safe, structured initiation |

### Key Insight: Effects Must Be Ultra-Thin
**HARD RULE from CLAUDE.md:**
- Effect handlers: <50 lines max
- Responsibility: Auth check → action call → event return
- All business logic: Lives in `domains/*/actions/` modules
- Pattern violation: MatchingEffect (170+ lines) - identified as debt to fix

**Example of Correct Pattern:**
```rust
async fn handle_event(
    event: Event,
    ctx: &EffectContext<ServerDeps>,
) -> Result<Event> {
    // 1. Auth check
    check_authorization(&ctx)?;

    // 2. Delegate to action
    let result = actions::do_business_logic(&ctx).await?;

    // 3. Return event
    Ok(Event::Complete { result })
}
```

### State Management Pattern (for 0.3.0)
**Option A (Recommended for Simple State):**
- Move HashMap/HashSet state into Effect struct
- Use `&mut self` to mutate state during event handling
- Example: `MemberEffect` tracks pending registrations

**Option B (For State Shared Across Effects):**
- Use Reducers for pure state transformations
- Effects remain immutable, reducer produces new state
- More functional, better for testing

**Example (Option A - MatchingEffect currently violates this):**
```rust
pub struct DomainApprovalEffect {
    requesters: HashMap<Uuid, MemberId>,  // State here
}

async fn handle(&mut self, event, ctx) -> Result<Event> {
    match event {
        Event::Start { job_id, requester } => {
            self.requesters.insert(job_id, requester);  // Mutate state
            actions::work(...).await
        }
    }
}
```

---

## 2. Domain Architecture & Separation Patterns

### Current Issue: Posts Domain is Too Large
**Status**: Plan created (2026-02-01) to separate `crawling` from `posts`

**Problem:**
- Posts domain conflates two unrelated responsibilities:
  1. **Crawling**: Multi-page website discovery, page fetching, content caching
  2. **Post Extraction**: Transform cached content into structured posts
- `CrawlerEffect` in posts domain: 1600+ lines of monolithic logic
- `PostCommand` enum: 425 lines with mixed concerns
- Posts domain directly manipulates `Website` entities (violates domain boundaries)

**Solution (Planned):**
```
Current:                          Target:
posts/                            posts/
├── crawler.rs (1600L)    →       ├── (small, post-only)
├── commands (425L)       →
                                  crawling/ (NEW)
                                  ├── models/page_snapshot, page_summary, website_snapshot
                                  ├── commands/crawl operations
                                  ├── events/crawl facts
                                  └── effects/crawler.rs

                                  scraping/ → DELETE (was just a facade)
```

**Key Lesson**: When domain responsibilities grow, watch for:
- One domain directly calling methods on another domain's models
- Mixed command/event enums handling unrelated workflows
- Effects > 500 lines (signal of multiple workflows crammed together)
- Domain models leaking across boundaries (e.g., posts manipulating website state)

### Domain Communication Pattern
**MUST USE**: Event-driven communication, NOT direct model calls

**BAD (posts domain doing):**
```rust
Website::find_by_id(id).await?
Website::start_crawl(...).await?
Website::complete_crawl(...).await?
```

**GOOD (after refactor):**
```rust
// crawling domain emits CrawlCompleted event
// posts domain listens to CrawlCompleted
// Each domain owns its models exclusively
```

---

## 3. SQL Query Rules (CRITICAL)

### HARD RULE: Never use `sqlx::query_as!` macro
**Always use function version: `sqlx::query_as::<_, Type>`**

**Why:**
- Macro requires compile-time database access (fragile)
- Type inference issues with nullable fields (Option<Option<T>>)
- Slower compilation
- Less flexible for complex queries

**Correct Pattern:**
```rust
#[derive(FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub metadata: Option<serde_json::Value>,
}

impl User {
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM users WHERE id = $1"
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }
}
```

### HARD RULE: NEVER Modify Existing Migration Scripts
**ALWAYS create a new migration file instead.**

**Why:**
- SQLx checksums migrations
- Modifying applied migrations breaks checksum
- Causes: "migration was previously applied but has been modified" errors
- Recovery requires manual database intervention
- **Can cause data loss in production**

**Example:**
```rust
// WRONG:
// Edit 000057_add_users.sql to add missing column

// CORRECT:
// Create 000058_add_user_email.sql
ALTER TABLE users ADD COLUMN email TEXT;
```

---

## 4. Database Schema Design (CRITICAL)

### HARD RULE: Avoid JSONB Columns
**Normalize data into proper relational tables instead.**

**Why JSONB is bad:**
- Harder to query: `WHERE search_results->>'title' = ...` (awkward syntax)
- No foreign key constraints
- No type safety in Rust models
- Can't add proper indexes on nested fields
- Requires JSON parsing in application code

**Anti-Pattern (DO NOT USE):**
```sql
CREATE TABLE domains (
    id UUID PRIMARY KEY,
    search_results JSONB  -- AVOID!
);
```

**Correct Pattern (Normalized):**
```sql
CREATE TABLE search_queries (
    id UUID PRIMARY KEY,
    domain_id UUID REFERENCES domains(id),
    query TEXT NOT NULL
);

CREATE TABLE search_results (
    id UUID PRIMARY KEY,
    query_id UUID REFERENCES search_queries(id),
    title TEXT NOT NULL,
    url TEXT NOT NULL,
    score DECIMAL(3,2)
);
```

**When JSONB is Acceptable (Rare):**
1. Truly unstructured data with unknown schema (arbitrary metadata)
2. External API responses you don't control (audit trail only)
3. Configuration objects with frequent schema changes

**Current Codebase Status:**
- ✅ Mostly normalized (posts, page_snapshots, etc.)
- ⚠️ Some optional JSONB fields remain (metadata columns)

---

## 5. Terminology & Naming Consistency

### Large Migration in Progress: Listing → Post
**Status**: Plan created (2026-02-01) for comprehensive rename
**Scope**: 1,026 occurrences across 53 files

**Impact Areas:**
- Event variants: `ListingApproved` → `PostApproved`
- Command variants: `CreateListing` → `CreatePost`
- Function names: ~200 references
- Variable names: ~250 references
- Comments: ~500 occurrences
- Error messages: ~50 occurrences

**Key Lesson**: Terminology inconsistencies compound over time. Started with "listing" in early codebase, pivoted to "post" semantics, but migration never completed. Creates confusion for new developers.

**Migration Strategy (ordered by risk):**
1. Types/Structs (least used)
2. Event/Command variants (moderate use)
3. Function names (high use)
4. Variables (highest use)
5. Comments (no runtime impact)

---

## 6. Architectural Debt Identified & Prioritized

### Phase 1: Pre-Refactor Cleanup (ZERO RISK)
**Status**: Planned but not yet executed

**Issues to fix:**
1. Delete stale `organization/effects/deps.rs` (duplicate ServerDeps)
2. Update test harness imports from dead `listings` module

**Time estimate**: < 1 hour
**Risk**: None (dead code removal)

### Phase 2: ServerDeps Migration (MEDIUM RISK)
**Status**: Planned

**Issue**: `ServerDeps` defined in posts domain but used by ALL domains
- Creates unnecessary coupling
- Breaks domain independence principle
- Hard to mock in tests

**Solution**: Move to `kernel/deps.rs` where it belongs
- **Impact**: 20 files need import path updates
- **Benefit**: Clear separation of kernel infrastructure
- **Risk**: Broken imports if done incorrectly

**Key files affected:**
```
- server/app.rs
- domains/auth/*
- domains/member/*
- domains/chatrooms/*
- domains/matching/*
- domains/domain_approval/*
- domains/website/*
```

### Phase 3: MatchingEffect Refactor (MEDIUM RISK)
**Status**: Identified, plan ready

**Current violation**: 170+ lines of inline business logic
- Should be < 50 lines
- Should dispatch to action functions

**Solution**: Extract business logic to `domains/matching/actions/`
- `handle_find_matches()` function
- `find_match_candidates()` helper
- `filter_and_notify()` helper

**Impact**: Matching still works, effect becomes thin dispatcher

### Phase 4: Terminology Migration (MEDIUM RISK, LARGE SCOPE)
**Status**: Comprehensive plan ready
**Scope**: 1,026 occurrences across 53 files

**Recommendation**: Do AFTER other refactors to avoid rebasing conflicts

### Phase 5: PostMachine Decomposition (HIGH RISK)
**Status**: Design complete, implementation not started

**Current issue**: God object handling 7 distinct workflows
```
1. Scraping Workflow
2. Post Management
3. Resource Link Submission
4. User Submission
5. Approval Workflows
6. Website Crawl
7. Regeneration
```

**Risk**: Each workflow must continue working, event chains must not break
**Recommendation**: Do LAST after phases 1-4 are stable

---

## 7. Specific Known Issues & Gotchas

### TwilioService Abstraction Underutilized
**Issue**: `ServerDeps` uses concrete `TwilioService` instead of `BaseTwilioService` trait
- `BaseTwilioService` trait exists but isn't used
- Makes testing harder (can't easily mock)
- Breaks dependency inversion principle

**Solution**: Change field type from concrete to trait object
```rust
// BEFORE
pub twilio: Arc<TwilioService>,

// AFTER
pub twilio: Arc<dyn BaseTwilioService>,
```

**Status**: Identified in architectural audit, awaiting Phase 2

### TODO Comments Throughout Codebase
**10 TODOs identified:**

| Priority | File | Item |
|----------|------|------|
| High | `matching/utils/relevance.rs` | Make AI call in production |
| Medium | `matching/effects/mod.rs` | Generate embedding if not exists |
| Medium | `chatrooms/effects/messaging.rs` | Check if author is admin |
| Medium | `chatrooms/effects/messaging.rs` | Look up or create agent member |
| Medium | `posts/effects/utils/sync_utils.rs` | Detect content changes |
| Low | Various | Store submitted_by_member_id, contact info, etc. |

**Pattern**: Many represent incomplete features that need attention when relevant workflow is touched.

---

## 8. API Provider Migrations (Completed)

### OpenAI → Claude + Voyage AI (Completed)
**Date**: Recently completed
**Changes Made:**

1. **AI Completions**
   - Provider: OpenAI GPT-4o → Anthropic Claude 3.5 Sonnet
   - Impact: Changed in `kernel/ai.rs`
   - Cost: 20-50% more expensive but better reasoning

2. **Embeddings**
   - Provider: OpenAI text-embedding-3-small (1536 dims) → Voyage AI voyage-3-large (1024 dims)
   - Impact: Vector dimension change in migration 000022
   - Impact: All existing embeddings invalidated (needed regeneration)
   - Cost: 6x more expensive but better semantic search

**Key Lessons:**
- Provider migrations are painful (embeddings need regeneration)
- Dimension changes cascade (all vector operations affected)
- Cost differences significant (evaluate carefully)
- API response formats differ (OpenAI vs Anthropic vs Voyage)

**Files affected:**
```
- config.rs (API keys)
- kernel/ai.rs
- common/utils/embeddings.rs
- migrations/000022_change_embedding_dimensions.sql
- bin/seed_organizations.rs
- dev-cli/src/main.rs
```

---

## 9. SQL Refactoring Patterns Applied

### Movement of Queries from Effects to Models
**Status**: Completed (recent migration)

**Pattern**: SQLx queries belong in model layer, not effects

**Example:**
```rust
// BEFORE (in effect)
let need = sqlx::query_as::<_, OrganizationNeed>(
    "SELECT * FROM needs WHERE source = $1 AND title = $2"
)
.bind(source)
.bind(title)
.fetch_optional(pool)
.await?;

// AFTER (in model)
impl OrganizationNeed {
    pub async fn find_by_source_and_title(source: &str, title: &str, pool: &PgPool) -> Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM needs WHERE source = $1 AND title = $2"
        )
        .bind(source)
        .bind(title)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }
}
```

**Benefits:**
- Separation of concerns
- Reusability (multiple effects can use same model methods)
- Testability (model methods can be tested independently)
- Centralized query maintenance

**Files migrated:**
```
✅ Organization domain (needs)
✅ Matching domain (notifications, match_candidates)
```

---

## 10. Best Practices Summary

### Do's ✅
- Use `sqlx::query_as::<_, Type>` (function version)
- Keep effects < 50 lines (dispatch only)
- Put business logic in actions module
- Use trait objects for abstraction (Arc<dyn Trait>)
- Normalize database schema (relational tables)
- Communicate between domains via events
- Create new migrations (never modify old ones)
- Test actions independently of effects
- Use Option<T> for nullable fields (not special annotations)
- Define structured errors (not string errors)

### Don'ts ❌
- Use `sqlx::query_as!` macro
- Modify existing migration files
- Put business logic in effects
- Direct model access across domain boundaries
- JSONB for structured data
- Concrete types where traits should be used
- More than 1-2 workflows per machine
- Ignore type system for "convenience"
- String-based error messages
- Duplicated state definitions across domains

---

## 11. How to Use This Document

### Before Starting Work
1. **Read the relevant section** for your feature type
2. **Check "Known Issues"** in that section
3. **Follow the patterns** documented here
4. **Reference the CLAUDE.md** rules for confirmed patterns

### When You Discover Something New
1. **Document it** in the appropriate section
2. **Add to CLAUDE.md** if it's a HARD RULE for the future
3. **Create a plan document** if it's a refactor

### Escalation Criteria
- **High impact** + **high risk** → Create formal plan document
- **Medium impact** → Add TODO comment in code + this document
- **Low impact** → Just document learning

---

## References

### Key Documentation
- **CLAUDE.md** - Hard rules and patterns (MUST READ)
- **docs/plans/2026-02-01-refactor-upgrade-seesaw-to-0.3.0-plan.md** - 0.3.0 architecture details
- **docs/plans/2026-02-01-refactor-separate-crawling-domain-plan.md** - Domain separation pattern
- **docs/plans/2026-02-01-refactor-edge-trait-migration-plan.md** - Edge trait migration
- **docs/plans/2026-02-01-refactor-architectural-audit-cleanup-plan.md** - Debt inventory
- **docs/migrations/SQL_QUERY_REFACTORING.md** - Query pattern migration
- **docs/migrations/MIGRATION_CLAUDE_VOYAGE.md** - Provider migration example

### Code Examples
- `domains/auth/actions/` - Simple actions
- `domains/member/effects/` - Stateful effect pattern
- `domains/matching/models/` - Model-based queries
- `domains/posts/effects/` - Composite effect pattern

### Current Branch
- `refactor/seesaw-0.3.0-upgrade` - In-progress upgrade work

---

**Last Updated**: February 2, 2026
**Document Status**: Active (reflects current codebase state)
