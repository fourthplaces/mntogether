---
status: pending
priority: p2
issue_id: "007"
tags: [code-review, simplification, refactoring, duplication]
dependencies: ["003"]
---

# Duplicate Dependency Containers (ServerKernel + ServerDeps)

## Problem Statement

The codebase has TWO nearly identical structs holding the same dependencies: `ServerKernel` (infrastructure) and `ServerDeps` (domain wrapper). This is pure duplication that adds unnecessary complexity and confusion.

## Findings

**Locations**:
- `/packages/server/src/kernel/server_kernel.rs` (ServerKernel - 47 lines)
- `/packages/server/src/domains/organization/effects/deps.rs` (ServerDeps - 37 lines)

**Both contain**:
- `db_pool: PgPool`
- `web_scraper: Arc<dyn BaseWebScraper>`
- `ai: Arc<dyn BaseAI>`
- `embedding_service: Arc<dyn BaseEmbeddingService>`
- `push_service: Arc<dyn BasePushNotificationService>`

**From Code Simplicity Reviewer**: "You have TWO nearly identical structs holding the same dependencies. This is pure duplication. Effects use ServerDeps, but you already have ServerKernel. You've created a redundant abstraction layer."

**Why it exists**: Refactoring created domain-specific wrapper around kernel, but this adds no value.

## Proposed Solutions

### Option 1: Delete ServerDeps, Use ServerKernel Everywhere (Recommended)
**Pros**: Removes 40 LOC, eliminates confusion, single source of truth
**Cons**: Need to update Effect trait bounds
**Effort**: Medium (1 hour)
**Risk**: Low

```rust
// DELETE: packages/server/src/domains/organization/effects/deps.rs (entire file)

// UPDATE: Effect implementations
impl Effect<OrganizationCommand, ServerKernel> for AIEffect {
    //                            ^^^^^^^^^^^^^ was ServerDeps
    type Event = OrganizationEvent;

    async fn execute(&self, cmd: OrganizationCommand, ctx: EffectContext<ServerKernel>)
        -> Result<OrganizationEvent>
    {
        let ai = ctx.deps().ai.as_ref();
        // ...
    }
}
```

### Option 2: Make ServerDeps Type Alias
**Pros**: Minimal code changes, backward compatible
**Cons**: Doesn't remove abstraction layer
**Effort**: Small (15 minutes)
**Risk**: Low

```rust
// In organization/effects/deps.rs
pub type ServerDeps = ServerKernel;

// No other changes needed
```

### Option 3: Keep Both, Document Rationale
**Pros**: No code changes
**Cons**: Continues technical debt, confusing for developers
**Effort**: None
**Risk**: None

## Recommended Action

**Option 1** - Delete `ServerDeps` entirely and use `ServerKernel` everywhere. The extra abstraction adds zero value and just creates confusion about which to use.

## Technical Details

**Files to Update**:
1. **DELETE**: `/packages/server/src/domains/organization/effects/deps.rs`
2. **UPDATE**: All effect implementations to use `ServerKernel`:
   - `/packages/server/src/domains/organization/effects/mod.rs`
   - `/packages/server/src/domains/matching/effects/mod.rs`
   - `/packages/server/src/domains/member/effects/mod.rs`
   - `/packages/server/src/domains/auth/effects/mod.rs`

**Impact**:
- ~40 LOC removed
- Clearer codebase structure
- One less concept for new developers to learn

## Acceptance Criteria

- [ ] `ServerDeps` struct deleted
- [ ] All effects use `ServerKernel` as dependency type
- [ ] All imports updated (`use crate::kernel::ServerKernel`)
- [ ] Compilation successful
- [ ] All tests pass
- [ ] Documentation updated to mention only ServerKernel

## Work Log

*Empty - work not started*

## Resources

- **PR/Issue**: N/A - Found in code review
- **Related Code**:
  - `/packages/server/src/kernel/server_kernel.rs` (keep this)
  - `/packages/server/src/domains/organization/effects/deps.rs` (delete this)
- **Related Findings**: #003 (kernel circular dependency) should be fixed first
