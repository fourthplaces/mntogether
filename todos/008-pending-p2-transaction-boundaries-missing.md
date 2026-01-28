---
status: pending
priority: p2
issue_id: "008"
tags: [code-review, data-integrity, transactions, database]
dependencies: []
---

# Multi-Step Operations Without Transaction Boundaries

## Problem Statement

Several database operations perform multiple updates without transaction wrappers, creating windows for data inconsistency if one step fails after another succeeds.

## Findings

**Location 1**: `/packages/server/src/domains/organization/effects/need_operations.rs:74-106`

**Vulnerable Code**:
```rust
pub async fn update_and_approve_need(...) -> Result<()> {
    // Step 1: Update content
    OrganizationNeed::update_content(...).await?;

    // Step 2: Set status to active
    OrganizationNeed::update_status(need_id, "active", pool).await?;

    Ok(())
}
```

**Risk**: If status update fails after content update succeeds, need remains in `pending_approval` with modified content - inconsistent state.

**Location 2**: `/packages/server/src/domains/organization/effects/need_operations.rs:109-182`

```rust
pub async fn create_post_for_need(...) -> Result<Post> {
    // Create post
    let mut post = Post::create_and_publish(...).await?;

    // Later: Update with outreach copy
    post = Post::update_outreach_copy(post.id, outreach_copy, pool).await?;
}
```

**Risk**: If outreach copy update fails, post exists without AI-generated copy.

**From Data Integrity Guardian**: "Multi-step operations without transactions create windows for data inconsistency if one step fails after another succeeds."

## Proposed Solutions

### Option 1: Wrap in Database Transactions (Recommended)
**Pros**: ACID guarantees, prevents partial updates
**Cons**: Slight performance overhead, longer locks
**Effort**: Small (30 minutes per operation)
**Risk**: Low

```rust
pub async fn update_and_approve_need(...) -> Result<()> {
    let mut tx = pool.begin().await?;

    OrganizationNeed::update_content(..., &mut *tx).await?;
    OrganizationNeed::update_status(need_id, "active", &mut *tx).await?;

    tx.commit().await?;
    Ok(())
}
```

### Option 2: Use Saga Pattern for Compensating Transactions
**Pros**: Better for distributed operations
**Cons**: Much more complex, overkill for single DB
**Effort**: Large (4+ hours)
**Risk**: High

### Option 3: Make Operations Idempotent
**Pros**: Can safely retry
**Cons**: Doesn't prevent inconsistent intermediate states
**Effort**: Medium (2 hours)
**Risk**: Medium

## Recommended Action

**Option 1** - Wrap multi-step operations in database transactions. This is the standard solution for maintaining data consistency.

## Technical Details

**Affected Operations**:
1. `update_and_approve_need()` - 2 steps (update content + update status)
2. `create_post_for_need()` - 2 steps (create post + update outreach copy)
3. `sync_needs()` - Multiple steps (query + create/update + mark disappeared)

**Model Methods to Update**:
Many model methods need transaction parameter variants:
```rust
impl OrganizationNeed {
    // Add transaction-aware variants
    pub async fn update_content_tx(
        id: Uuid,
        title: String,
        description: String,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<Self> { ... }
}
```

## Acceptance Criteria

- [ ] `update_and_approve_need()` wrapped in transaction
- [ ] `create_post_for_need()` decision made (transaction or async generation)
- [ ] `sync_needs()` wrapped in transaction
- [ ] Model methods accept `&mut Transaction` parameter
- [ ] Tests verify rollback on partial failure
- [ ] Documentation updated with transaction patterns
- [ ] Error handling preserves transaction semantics

## Work Log

*Empty - work not started*

## Resources

- **PR/Issue**: N/A - Found in code review
- **Related Code**:
  - `/packages/server/src/domains/organization/effects/need_operations.rs:74-182`
  - `/packages/server/src/domains/organization/effects/utils/sync_utils.rs:40-104`
- **Documentation**:
  - [SQLx Transactions](https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html)
  - [Database Transaction Best Practices](https://www.postgresql.org/docs/current/tutorial-transactions.html)
