---
title: "feat: Clean Up Org Posts Workflow"
type: feat
date: 2026-02-11
brainstorm: docs/brainstorms/2026-02-11-clean-up-org-posts-brainstorm.md
---

# Clean Up Org Posts Workflow

## Overview

A new Restate workflow that cleans up duplicate and rejected posts for an organization. Triggered manually from the admin org page, it serves as a safety net for LLM extraction imperfections — catching duplicates that GPT-5 Mini missed during extraction by running a cleanup pass with GPT-5 full.

## Problem Statement

Duplicate published posts appear in the admin org view, especially from social media sources. The same resource gets extracted multiple times across runs, or the same content appears on both a website and Instagram with different wording. The existing deduplication workflow only catches duplicates at the **pending** stage. Once posts are published, `apply_dedup_results` in `deduplication.rs:240-248` explicitly skips active posts. There is no mechanism to clean up:

1. Duplicate active posts within the same source
2. Duplicate active posts across sources (website + social media)
3. Stale rejected posts cluttering the database

## Proposed Solution

Reuse the existing `detect_cross_source_duplicates` and `stage_cross_source_dedup` functions with two changes: (1) parameterize the model so we can pass `GPT_5` instead of `GPT_5_MINI`, and (2) remove the single-source-type early return so same-source duplicates are caught too. Then add a simple rejected-post purge. One LLM call, one batch, one purge.

### Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Dedup approach | Parameterize existing `detect_cross_source_duplicates` + `stage_cross_source_dedup` | Avoids ~250 lines of duplicated proposal-building logic |
| LLM calls | Single call with all active + pending posts | `CROSS_SOURCE_DEDUP_SYSTEM_PROMPT` already handles mixed statuses and source priority |
| Batching | Single sync batch for all dedup proposals | Avoids batch collision bug where Phase N expires Phase N-1's batch via `find_stale` |
| Dedup model | GPT-5 full | Catches what GPT-5 Mini missed; safety net, not hot path |
| Merge reason model | GPT-5 Mini (via existing `generate_merge_reason`) | Short user-facing text doesn't need the big model |
| Same-source dedup | Remove the `source_types.len() < 2` early return gate | Current code at `deduplication.rs:461-473` skips orgs with one source type; cleanup should catch same-source dupes too |
| Rejected purge | Direct soft-delete, no proposals | Already rejected = safe to auto-clean |
| Concurrent runs | Restate workflow keyed by org_id | Prevents two cleanups racing on same org |
| Service routing | Route through `OrganizationsService` | Matches `extract_org_posts` pattern; handles auth via `require_admin` |
| Result counts | Use `i32` | Matches all existing workflow result types |

## Acceptance Criteria

- [x] Admin can trigger "Clean Up Posts" from org detail page
- [x] Workflow deduplicates all org posts (pending + active, same-source + cross-source) in one LLM pass
- [x] Workflow soft-deletes rejected posts automatically
- [x] All dedup proposals go through a single sync batch for admin review
- [x] Progress status visible in UI during workflow execution
- [x] Handles orgs with zero posts gracefully (early return)
- [x] Uses GPT-5 for duplicate detection

## Technical Approach

### Modified Files

#### 1. `packages/server/src/domains/posts/activities/deduplication.rs`

**Add `model: &str` parameter to two existing functions:**

`detect_cross_source_duplicates` (line 446):
```rust
// BEFORE
pub async fn detect_cross_source_duplicates(
    org_id: Uuid,
    ai: &OpenAi,
    pool: &PgPool,
) -> Result<DuplicateAnalysis>

// AFTER
pub async fn detect_cross_source_duplicates(
    org_id: Uuid,
    model: &str,
    ai: &OpenAi,
    pool: &PgPool,
) -> Result<DuplicateAnalysis>
```

- Change `GPT_5_MINI` to `model` at line 498
- **Remove the single-source-type early return** at lines 461-473 (the `source_types.len() < 2` gate). Cleanup needs to catch same-source duplicates too. The LLM prompt handles single-source scenarios fine.

`stage_cross_source_dedup` (line 520):
```rust
// BEFORE
pub async fn stage_cross_source_dedup(
    org_id: Uuid,
    ai: &OpenAi,
    pool: &PgPool,
) -> Result<StageCrossSourceResult>

// AFTER
pub async fn stage_cross_source_dedup(
    org_id: Uuid,
    model: &str,
    ai: &OpenAi,
    pool: &PgPool,
) -> Result<StageCrossSourceResult>
```

- Passes `model` through to `detect_cross_source_duplicates`

**Update existing callers** (`deduplicate_cross_source_all_orgs` at line 694):
```rust
// Pass GPT_5_MINI to maintain existing behavior
stage_cross_source_dedup(org_id.into_uuid(), GPT_5_MINI, deps.ai.as_ref(), &deps.db_pool)
```

**Add purge activity** (~15 lines):
```rust
/// Soft-delete all rejected posts for an organization.
/// Returns count of posts purged.
pub async fn purge_rejected_posts_for_org(
    org_id: Uuid,
    pool: &PgPool,
) -> Result<usize> {
    let rejected = Post::find_rejected_by_organization(org_id, pool).await?;
    let count = rejected.len();
    for post in &rejected {
        Post::soft_delete(post.id, "Purged by org cleanup (rejected)", pool).await?;
    }
    info!(org_id = %org_id, purged = count, "Purged rejected posts");
    Ok(count)
}
```

#### 2. `packages/server/src/domains/posts/models/post.rs`

**One new query method** (rejected posts for purge):

```rust
pub async fn find_rejected_by_organization(
    organization_id: Uuid,
    pool: &PgPool,
) -> Result<Vec<Self>> {
    sqlx::query_as::<_, Self>(
        r#"
        SELECT DISTINCT ON (p.id) p.*
        FROM posts p
        JOIN post_sources ps ON ps.post_id = p.id
        JOIN sources s ON ps.source_id = s.id
        WHERE s.organization_id = $1
          AND p.status = 'rejected'
          AND p.deleted_at IS NULL
        ORDER BY p.id
        "#,
    )
    .bind(organization_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}
```

No other new model methods needed — `find_active_pending_by_organization_with_source` already loads both active + pending posts for the dedup LLM call.

### New Files

#### 3. `packages/server/src/domains/organization/restate/workflows/clean_up_org_posts.rs`

~80 lines. Follows `extract_org_posts.rs` pattern.

```rust
// Request
pub struct CleanUpOrgPostsRequest {
    pub organization_id: Uuid,
}
impl_restate_serde!(CleanUpOrgPostsRequest);

// Response
pub struct CleanUpOrgPostsResult {
    pub duplicates_found: i32,
    pub proposals_staged: i32,
    pub rejected_purged: i32,
    pub status: String,
}
impl_restate_serde!(CleanUpOrgPostsResult);

// Trait
#[restate_sdk::workflow]
#[name = "CleanUpOrgPostsWorkflow"]
pub trait CleanUpOrgPostsWorkflow {
    async fn run(req: CleanUpOrgPostsRequest) -> Result<CleanUpOrgPostsResult, HandlerError>;
    #[shared]
    async fn get_status(req: EmptyRequest) -> Result<String, HandlerError>;
}
```

**Workflow body** (pseudocode):

```
1. Load org, validate exists → TerminalError if not found
2. ctx.set("status", "Deduplicating posts...")

3. ctx.run: dedup_result = stage_cross_source_dedup(org_id, GPT_5, ai, pool)
   - Single LLM call: loads all active + pending posts for org
   - CROSS_SOURCE_DEDUP_SYSTEM_PROMPT handles mixed statuses, source priority
   - Creates revisions for merged content
   - Stages all proposals in ONE sync batch
   - Returns StageCrossSourceResult { batch_id, proposals_staged }

4. ctx.set("status", "Purging rejected posts...")

5. ctx.run: purged = purge_rejected_posts_for_org(org_id, pool)

6. ctx.set("status", "Completed")
7. Return CleanUpOrgPostsResult {
       duplicates_found: dedup_result.proposals_staged as i32,
       proposals_staged: dedup_result.proposals_staged as i32,
       rejected_purged: purged as i32,
       status: "completed",
   }
```

**Error handling**: Best-effort. If dedup fails, log error and continue to purge. If purge fails, return partial result with error status.

```rust
// Dedup phase — best-effort
let dedup_result = match ctx.run(|| async {
    stage_cross_source_dedup(org_id, GPT_5, ai, pool)
        .await.map_err(Into::into)
}).await {
    Ok(r) => r,
    Err(e) => {
        warn!(org_id = %org_id, error = %e, "Dedup failed, continuing to purge");
        StageCrossSourceResult { batch_id: None, proposals_staged: 0 }
    }
};

// Purge phase — best-effort
let purged = match ctx.run(|| async {
    purge_rejected_posts_for_org(org_id, pool)
        .await.map_err(Into::into)
}).await {
    Ok(n) => n,
    Err(e) => {
        warn!(org_id = %org_id, error = %e, "Purge failed");
        0
    }
};
```

#### 4. `packages/server/src/domains/organization/restate/services/organizations.rs`

**Add `clean_up_org_posts` handler** to `OrganizationsService` trait + impl. Follows exact `extract_org_posts` pattern (lines 678-701):

```rust
// Trait addition:
async fn clean_up_org_posts(
    req: RegenerateOrganizationRequest,
) -> Result<CleanUpOrgPostsServiceResult, HandlerError>;

// Impl:
async fn clean_up_org_posts(
    &self,
    ctx: Context<'_>,
    req: RegenerateOrganizationRequest,
) -> Result<CleanUpOrgPostsServiceResult, HandlerError> {
    let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
    let org_id = req.id;

    let workflow_id = format!("cleanup-org-{}-{}", org_id, chrono::Utc::now().timestamp());

    ctx.workflow_client::<CleanUpOrgPostsWorkflowClient>(workflow_id.clone())
        .run(CleanUpOrgPostsRequest { organization_id: org_id })
        .send();

    Ok(CleanUpOrgPostsServiceResult {
        organization_id: Some(org_id.to_string()),
        status: format!("started:{}", workflow_id),
    })
}
```

Reuses `RegenerateOrganizationRequest` (just `{ id: Uuid }`) as input. Returns a simple result with the workflow ID for status polling.

### Wiring

#### 5. `packages/server/src/domains/organization/restate/workflows/mod.rs`

```rust
pub mod clean_up_org_posts;
```

#### 6. `packages/server/src/bin/server.rs`

```rust
.bind(CleanUpOrgPostsWorkflowImpl::with_deps(server_deps.clone()).serve())
```

#### 7. `packages/web/app/admin/(app)/organizations/[id]/page.tsx`

"Clean Up Posts" button following the "Extract Org Posts" pattern:

```typescript
const [cleaningUp, setCleaningUp] = useState(false);

const handleCleanUpPosts = async () => {
    setCleaningUp(true);
    try {
        await callService("Organizations", "clean_up_org_posts", { id: orgId });
        invalidateService("Posts");
        refetchPosts();
    } catch (err: any) {
        alert(err.message || "Failed to clean up posts");
    } finally {
        setCleaningUp(false);
    }
};
```

Routes through `OrganizationsService` (not direct workflow call), matching existing pattern.

#### 8. `packages/web/lib/restate/types.ts`

Add request/response types for the new service handler.

## Edge Cases

| Edge Case | Handling |
|-----------|----------|
| Org with 0 posts | `detect_cross_source_duplicates` returns empty (existing `posts.len() < 2` check) |
| Org with only rejected posts | Dedup returns 0 proposals, purge runs normally |
| LLM fails (timeout, rate limit) | Best-effort: log error, continue to purge, return partial result |
| Post state changes during workflow | No locking; stale proposals may fail on approval (acceptable) |
| Concurrent cleanup for same org | Restate workflow key = org_id prevents concurrent runs |
| Large org (200+ posts) | Accept for now; chunk if needed later (same limitation as existing cross-source dedup) |

## Implementation Order

1. **Model method** — `Post::find_rejected_by_organization` in `post.rs`
2. **Parameterize existing functions** — add `model` param to `detect_cross_source_duplicates` + `stage_cross_source_dedup`, remove single-source gate, add `purge_rejected_posts_for_org`, update existing caller
3. **Workflow** — `clean_up_org_posts.rs`
4. **Service handler** — add to `OrganizationsService`
5. **Registration** — `server.rs` + `mod.rs`
6. **Admin UI** — button + types

## What We're NOT Building

- No new activity file (`org_cleanup.rs`) — reuse existing dedup functions
- No separate LLM calls per phase — one call catches all combinations
- No multiple sync batches — one batch, no collision risk
- No new LLM prompts — `CROSS_SOURCE_DEDUP_SYSTEM_PROMPT` already handles mixed statuses
- No per-phase result counts — `duplicates_found`, `proposals_staged`, `rejected_purged`

## References

- Brainstorm: `docs/brainstorms/2026-02-11-clean-up-org-posts-brainstorm.md`
- Existing dedup (parameterize these): `packages/server/src/domains/posts/activities/deduplication.rs:446-664`
- Existing dedup workflow (pattern for `ctx.run()`): `packages/server/src/domains/posts/restate/workflows/deduplicate_posts.rs`
- Org extraction workflow (pattern for workflow structure): `packages/server/src/domains/organization/restate/workflows/extract_org_posts.rs`
- Service handler pattern: `packages/server/src/domains/organization/restate/services/organizations.rs:678-701`
- Proposal staging: `packages/server/src/domains/sync/activities/proposal_actions.rs`
- Post model: `packages/server/src/domains/posts/models/post.rs`
- Admin org page: `packages/web/app/admin/(app)/organizations/[id]/page.tsx`
