---
title: "feat: Schedule-Aware Post Filtering"
type: feat
date: 2026-02-11
---

# Schedule-Aware Post Filtering

## Overview

Add schedule-aware filtering to public post queries so posts with all-expired schedules are automatically hidden. Posts without schedules remain visible (evergreen). A daily Restate sweep keeps `status` truthful.

**Core rule:** The schedule IS the signal. Has schedules → temporal. No schedules → evergreen.

**Brainstorm:** `docs/brainstorms/2026-02-11-schedule-aware-post-filtering-brainstorm.md`

## Proposed Solution

Two components:

1. **SQL predicate** added to all public-facing queries — filters out posts whose schedules have all expired
2. **Daily Restate service handler** — sweeps and sets `status = 'expired'` for data honesty

## Implementation

### 1. Define the SQL predicate as a constant

**File:** `packages/server/src/domains/posts/models/post.rs`

Add a constant on `Post` that contains the reusable WHERE clause fragment:

```rust
impl Post {
    /// SQL predicate: filters out posts with all-expired schedules.
    /// Posts without schedules (evergreen) always pass.
    /// Posts with at least one active schedule pass.
    ///
    /// Schedule "active" means:
    /// - One-off event: dtend (or dtstart if no dtend) is in the future
    /// - Recurring/operating hours: valid_to is NULL (open-ended) or in the future
    const SCHEDULE_ACTIVE_FILTER: &'static str = r#"
        AND (
            NOT EXISTS (
                SELECT 1 FROM schedules s
                WHERE s.schedulable_type = 'post' AND s.schedulable_id = p.id
            )
            OR EXISTS (
                SELECT 1 FROM schedules s
                WHERE s.schedulable_type = 'post' AND s.schedulable_id = p.id
                AND (
                    (NULLIF(s.rrule, '') IS NULL AND COALESCE(s.dtend, s.dtstart) > NOW())
                    OR (NULLIF(s.rrule, '') IS NOT NULL AND (s.valid_to IS NULL OR s.valid_to >= CURRENT_DATE))
                )
            )
        )
    "#;
}
```

Notes:
- `NULLIF(s.rrule, '')` guards against empty-string rrule being treated as recurring
- Uses existing index `idx_schedules_schedulable(schedulable_type, schedulable_id)`
- `TIMESTAMPTZ` comparison for one-off events, `DATE` comparison for recurring — types match schema

### 2. Add predicate to public-facing queries

**File:** `packages/server/src/domains/posts/models/post.rs`

Each of these methods needs the predicate. Since the queries use string literals today, switch to `format!()` to interpolate the constant:

| Method | Line | Change |
|--------|------|--------|
| `find_public_filtered()` | ~1502 | Add `{}` placeholder before ORDER BY, use `format!()` |
| `count_public_filtered()` | ~1536 | Add `{}` placeholder, use `format!()` |
| `find_near_zip()` | ~1266 | Add `{}` placeholder, use `format!()` |
| `search_by_similarity()` | ~1057 | Add `{}` placeholder, use `format!()` |
| `search_by_similarity_with_location()` | ~1097 | Add `{}` placeholder, use `format!()` |
| `find_by_organization_id()` | ~596 | Add `{}` placeholder, use `format!()` |
| `find_by_type()` | ~533 | Add `{}` placeholder, use `format!()` |
| `find_by_category()` | ~554 | Add `{}` placeholder, use `format!()` |
| `find_by_capacity()` | ~575 | Add `{}` placeholder, use `format!()` |

**Do NOT add to:**
- `find_paginated()` — admin list with explicit status filter
- `find_all_by_organization_id()` — admin view
- `find_by_ids()` — internal batch loader
- `find_by_status()` — admin status browsing

Example transformation for `find_public_filtered()`:

```rust
pub async fn find_public_filtered(
    post_type: Option<&str>,
    category: Option<&str>,
    limit: i64,
    offset: i64,
    pool: &PgPool,
) -> Result<Vec<Self>> {
    let sql = format!(
        r#"
        SELECT DISTINCT p.* FROM posts p
        LEFT JOIN taggables tg_pt ON tg_pt.taggable_type = 'post' AND tg_pt.taggable_id = p.id
        LEFT JOIN tags t_pt ON t_pt.id = tg_pt.tag_id AND t_pt.kind = 'post_type'
        LEFT JOIN taggables tg_cat ON tg_cat.taggable_type = 'post' AND tg_cat.taggable_id = p.id
        LEFT JOIN tags t_cat ON t_cat.id = tg_cat.tag_id AND t_cat.kind = 'service_offered'
        WHERE p.status = 'active'
          AND p.deleted_at IS NULL
          AND p.revision_of_post_id IS NULL
          AND p.translation_of_id IS NULL
          AND ($1::text IS NULL OR t_pt.value = $1)
          AND ($2::text IS NULL OR t_cat.value = $2)
          {}
        ORDER BY p.created_at DESC
        LIMIT $3 OFFSET $4
        "#,
        Self::SCHEDULE_ACTIVE_FILTER
    );
    sqlx::query_as::<_, Self>(&sql)
        .bind(post_type)
        .bind(category)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
}
```

### 3. Create the daily sweep activity

**File:** `packages/server/src/domains/posts/activities/expire_scheduled_posts.rs` (new)

```rust
/// Expire posts whose schedules have all passed.
/// Called by the sweep service handler on a daily schedule.
pub async fn expire_scheduled_posts(deps: &ServerDeps) -> Result<u64> {
    Post::expire_by_schedule(&deps.db_pool).await
}
```

**File:** `packages/server/src/domains/posts/models/post.rs`

Add the model method:

```rust
/// Mark posts as expired when all their schedules have passed.
/// Only affects posts that have schedules (evergreen posts are untouched).
pub async fn expire_by_schedule(pool: &PgPool) -> Result<u64> {
    let result = sqlx::query(
        r#"
        UPDATE posts SET status = 'expired', updated_at = NOW()
        WHERE status = 'active'
          AND EXISTS (
            SELECT 1 FROM schedules s
            WHERE s.schedulable_type = 'post' AND s.schedulable_id = posts.id
          )
          AND NOT EXISTS (
            SELECT 1 FROM schedules s
            WHERE s.schedulable_type = 'post' AND s.schedulable_id = posts.id
            AND (
              (NULLIF(s.rrule, '') IS NULL AND COALESCE(s.dtend, s.dtstart) > NOW())
              OR (NULLIF(s.rrule, '') IS NOT NULL AND (s.valid_to IS NULL OR s.valid_to >= CURRENT_DATE))
            )
          )
        "#,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}
```

### 4. Create the Restate service handler for the sweep

**File:** `packages/server/src/domains/posts/restate/services/posts.rs`

Add a new handler to the existing `PostsService`:

```rust
// In the trait definition:
async fn expire_stale_posts(req: ExpireStalePostsRequest) -> Result<ExpireStalePostsResult, HandlerError>;

// Request/response types (in the same file or types module):
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpireStalePostsRequest {}
impl_restate_serde!(ExpireStalePostsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpireStalePostsResult {
    pub expired_count: u64,
}
impl_restate_serde!(ExpireStalePostsResult);

// In the impl:
async fn expire_stale_posts(
    &self,
    ctx: Context<'_>,
    _req: ExpireStalePostsRequest,
) -> Result<ExpireStalePostsResult, HandlerError> {
    let expired_count = ctx
        .run(|| async {
            activities::expire_scheduled_posts::expire_scheduled_posts(&self.deps)
                .await
                .map_err(Into::into)
        })
        .await?;

    info!(expired_count, "Sweep: expired posts by schedule");

    Ok(ExpireStalePostsResult { expired_count })
}
```

**Invocation:** External cron (systemd timer, Kubernetes CronJob, or simple cron) calls:

```bash
curl -X POST http://restate:9070/Posts/expire_stale_posts \
  -H 'content-type: application/json' \
  -d '{}'
```

This is the simplest approach — no new workflow type needed. The existing `PostsService` gets a new handler. An external scheduler triggers it daily.

### 5. Register the activity module

**File:** `packages/server/src/domains/posts/activities/mod.rs`

Add:
```rust
pub mod expire_scheduled_posts;
```

No server.rs changes needed — the handler is on the existing `PostsService` which is already registered.

## Acceptance Criteria

- [x] Public queries (`find_public_filtered`, `find_near_zip`, `search_by_similarity`, etc.) exclude posts with all-expired schedules
- [x] Posts without schedules remain visible in all public queries
- [x] Posts with at least one active schedule remain visible
- [x] `expire_stale_posts` handler marks schedule-expired posts as `status = 'expired'`
- [x] `cargo check` passes clean
- [x] Existing `upcoming_events` behavior unchanged

## Files Changed

| File | Change |
|------|--------|
| `packages/server/src/domains/posts/models/post.rs` | Add `SCHEDULE_ACTIVE_FILTER` const, add to 9 public queries, add `expire_by_schedule()` |
| `packages/server/src/domains/posts/activities/expire_scheduled_posts.rs` | New — thin activity calling model |
| `packages/server/src/domains/posts/activities/mod.rs` | Add `pub mod expire_scheduled_posts` |
| `packages/server/src/domains/posts/restate/services/posts.rs` | Add `expire_stale_posts` handler + request/response types |

## What's NOT Changing

- No new database tables or migrations
- No front-end changes (tag filtering already works)
- No changes to admin queries
- No changes to `upcoming_events` activity
- No new Restate workflow — just a new handler on existing service
