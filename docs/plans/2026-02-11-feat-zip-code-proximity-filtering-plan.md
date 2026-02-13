---
title: "feat: Composable Zip Code Proximity Filtering"
type: feat
date: 2026-02-11
---

# Composable Zip Code Proximity Filtering

## Overview

Add optional zip code + radius parameters to the existing `Posts.list` Restate service so that proximity becomes a composable filter dimension alongside status, post_type, source, and search. When active, posts are sorted by distance and include a `distance_miles` field. When absent, behavior is unchanged.

Also fix a latent pagination bug: `ListPostsRequest` is missing an `offset` field that the admin UI already sends.

## Problem Statement

The backend has full proximity search infrastructure (`haversine_distance`, `zip_codes` table, `find_near_zip` method) but it's isolated in a standalone endpoint (`Posts.search_nearby`). Admins reviewing posts can't filter by geography, and the public directory has no location-based search. Zip proximity needs to compose with all existing filters — not live as a separate feature.

Additionally, the admin UI uses `useOffsetPagination` which sends `{ first, offset }`, but `ListPostsRequest` only has `{ first, after, last, before }`. The `offset` is silently dropped, meaning pagination beyond page 1 is broken today.

## Proposed Solution

Compose zip/radius into the existing `Posts.list` pipeline: request types, model queries, activity layer, Restate handler, and admin UI. Two parallel query paths in the model layer — one with proximity joins (when zip is present), one without (existing behavior). The response type gains an optional `distance_miles` field with `skip_serializing_if`, so it's absent from JSON when not filtering by location.

## Technical Approach

### Architecture

```
Admin UI (zip input + radius dropdown)
    ↓
Posts.list { ...existing filters, zip_code?, radius_miles?, offset? }
    ↓
activities::get_posts_paginated (branches on zip presence)
    ↓
┌─ zip absent ─→ Post::find_paginated (existing, now also supports offset)
└─ zip present ─→ Post::find_paginated_near_zip (new method)
    ↓
PostListResult { posts: Vec<PostResult>, ... }
    (PostResult.distance_miles is Some when zip active, absent from JSON otherwise)
```

### Key Design Decisions

1. **`distance_miles: Option<f64>` on `PostResult`** with `#[serde(skip_serializing_if = "Option::is_none")]`. Follows the existing pattern on `PostResult` (see `published_at`, `tags`, `organization_id`). The field is literally absent from JSON when not filtering by zip. Only `distance_miles` — no `zip_code` or `location_city` for v1 (YAGNI; the existing `location` text field provides context).

2. **Separate model method, not conditional SQL.** Create `Post::find_paginated_near_zip()` rather than injecting conditional JOINs into `find_paginated()`. The SQL shapes are fundamentally different (INNER JOINs on locations/zip_codes, CROSS JOIN on center CTE, GROUP BY for dedup, distance-based ordering). Two clean methods > one messy conditional builder. Follows the codebase pattern of distinct query methods.

3. **Fix offset pagination for both paths.** Add `offset: Option<i32>` to `ListPostsRequest`. The admin UI already sends offset via `useOffsetPagination` — currently silently dropped. The non-zip path uses offset when provided (falling back to cursor pagination if `after`/`before` are sent instead). The zip path always uses offset (distance ordering is incompatible with UUID cursors).

4. **`GROUP BY` with `MIN(distance)` for multi-location dedup.** A post with 3 locations could match the radius on 2 of them. Instead of `DISTINCT ON` + wrapping query (two sorts), use `GROUP BY p.id` with `MIN(haversine_distance(...))` — deduplicates, picks closest distance, and sorts correctly in a single pass.

5. **`COUNT(*) OVER()` window function for total count.** Eliminates the need for a separate count query. The window function runs before LIMIT/OFFSET and counts grouped results correctly.

6. **Bounding box pre-filter before haversine.** Filter zip codes by a rectangular lat/lng range before the expensive trig computation. Eliminates 99%+ of zip codes from consideration. Requires a new index on `zip_codes(latitude, longitude)`.

7. **No `SCHEDULE_ACTIVE_FILTER` in the admin query.** Matches `find_paginated` behavior — admins see all posts regardless of schedule state.

8. **Validate zip, return error for non-MN.** Check zip against `zip_codes` table. Unknown zips return a `TerminalError` with message: "Zip code '{zip}' not found. Only Minnesota zip codes are currently supported."

9. **Leave `find_near_zip` and `search_nearby` alone.** Different use case (public proximity search), different response shape, different contract. No refactor for refactor's sake.

## Acceptance Criteria

- [x] `ListPostsRequest` accepts optional `zip_code`, `radius_miles`, and `offset`
- [x] When zip is provided: posts are filtered by haversine distance, sorted by proximity, and include `distance_miles` in response
- [x] When zip is absent: existing behavior is completely unchanged
- [x] Zip filter composes with all existing filters (status, source_type, source_id, agent_id, search)
- [x] Posts without locations are excluded when zip filter is active (correct behavior)
- [x] Posts with multiple locations appear once at their closest matching location's distance
- [x] Invalid/non-MN zip codes return a clear error message
- [x] Default radius is 25 miles when zip provided without radius
- [x] `total_count` reflects the proximity-filtered count when zip is active
- [x] Offset pagination works for both zip and non-zip paths
- [x] Admin UI: zip text input + radius dropdown (5/10/25/50 mi) in filter bar
- [x] Admin UI: distance column visible when zip filter is active
- [x] Admin UI: filter chip shows "Near: 55401 (25 mi)" with clear button
- [x] Admin UI: pagination resets to page 1 when zip filter changes

## Implementation Plan

### Phase 0: Migration — Bounding Box Index

**New migration file** (next sequential number)

```sql
CREATE INDEX idx_zip_codes_lat_lng ON zip_codes(latitude, longitude);
```

One line. Enables the bounding box pre-filter to use an index range scan instead of scanning all 54k zip codes.

### Phase 1: Fix Offset Pagination on `ListPostsRequest`

**File:** `packages/server/src/domains/posts/restate/services/posts.rs`

Add `offset` to request type:

```rust
pub struct ListPostsRequest {
    pub status: Option<String>,
    pub source_type: Option<String>,
    pub source_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub search: Option<String>,
    pub zip_code: Option<String>,
    pub radius_miles: Option<f64>,
    pub first: Option<i32>,
    pub offset: Option<i32>,
    pub after: Option<String>,
    pub last: Option<i32>,
    pub before: Option<String>,
}
```

Update the `list` handler to pass `offset` through. When `offset` is provided, use it for pagination instead of cursor args.

### Phase 2: Model Layer — `find_paginated_near_zip`

**File:** `packages/server/src/domains/posts/models/post.rs`

Add `published_at` and `updated_at` fields to `PostWithDistance`:

```rust
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct PostWithDistance {
    // ...existing fields...
    pub published_at: Option<DateTime<Utc>>,  // add
    pub updated_at: DateTime<Utc>,             // add
}
```

Create `find_paginated_near_zip`:

```rust
pub async fn find_paginated_near_zip(
    center_zip: &str,
    radius_miles: f64,
    status: Option<&str>,
    source_type: Option<&str>,
    source_id: Option<Uuid>,
    agent_id: Option<Uuid>,
    search: Option<&str>,
    limit: i32,
    offset: i32,
    pool: &PgPool,
) -> Result<(Vec<PostWithDistance>, i32)>
```

Returns `(results, total_count)` — total count via `COUNT(*) OVER()`.

SQL:

```sql
WITH center AS (
    SELECT latitude, longitude FROM zip_codes WHERE zip_code = $1
)
SELECT p.id, p.title, p.description,
       p.description_markdown, p.summary,
       p.post_type, p.category, p.status, p.urgency,
       p.location, p.submission_type, p.source_url,
       p.created_at, p.published_at, p.updated_at,
       MIN(haversine_distance(c.latitude, c.longitude, z.latitude, z.longitude)) as distance_miles,
       COUNT(*) OVER() as total_count
FROM posts p
INNER JOIN post_locations pl ON pl.post_id = p.id
INNER JOIN locations l ON l.id = pl.location_id
INNER JOIN zip_codes z ON l.postal_code = z.zip_code
LEFT JOIN agents a ON a.member_id = p.submitted_by_id
LEFT JOIN post_sources ps ON ps.post_id = p.id
CROSS JOIN center c
WHERE p.deleted_at IS NULL
  AND p.revision_of_post_id IS NULL
  AND p.translation_of_id IS NULL
  AND ($2::text IS NULL OR p.status = $2)
  AND ($3::text IS NULL OR ps.source_type = $3)
  AND ($4::uuid IS NULL OR ps.source_id = $4)
  AND ($5::uuid IS NULL OR a.id = $5)
  AND ($6::text IS NULL OR p.title ILIKE $6 OR p.description ILIKE $6)
  -- Bounding box pre-filter (rectangle containing the radius circle)
  AND z.latitude BETWEEN c.latitude - ($7 / 69.0)
                       AND c.latitude + ($7 / 69.0)
  AND z.longitude BETWEEN c.longitude - ($7 / (69.0 * cos(radians(c.latitude))))
                        AND c.longitude + ($7 / (69.0 * cos(radians(c.latitude))))
GROUP BY p.id, p.title, p.description, p.description_markdown, p.summary,
         p.post_type, p.category, p.status, p.urgency, p.location,
         p.submission_type, p.source_url, p.created_at, p.published_at, p.updated_at,
         c.latitude, c.longitude
HAVING MIN(haversine_distance(c.latitude, c.longitude, z.latitude, z.longitude)) <= $7
ORDER BY distance_miles ASC
LIMIT $8 OFFSET $9
```

Key points:
- `GROUP BY p.id, ...` deduplicates multi-location posts
- `MIN(haversine_distance(...))` picks the closest location's distance
- `HAVING` filters by radius after grouping (so `MIN` is used, not per-row distance)
- `COUNT(*) OVER()` gives total matching groups for pagination
- Bounding box pre-filter eliminates distant zip codes before trig computation
- No `SCHEDULE_ACTIVE_FILTER` — this is an admin query
- Single pass: dedup + sort + count, no wrapping query needed

Note: The return type needs a wrapper struct to capture `total_count` from the window function:

```rust
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PostWithDistanceAndCount {
    // ...all PostWithDistance fields...
    pub total_count: i64,
}
```

### Phase 3: Activity Layer

**File:** `packages/server/src/domains/posts/activities/core.rs`

Update `get_posts_paginated` to accept optional `zip_code`, `radius_miles`, and `offset`. Branch:

```rust
if let Some(zip) = zip_code {
    let _center = ZipCode::find_by_code(zip, pool).await?
        .ok_or_else(|| anyhow!("Zip code '{}' not found. Only Minnesota zip codes are currently supported.", zip))?;

    let radius = radius_miles.unwrap_or(25.0).min(100.0);
    let limit = first.unwrap_or(20);
    let off = offset.unwrap_or(0);

    let (results, total_count) = Post::find_paginated_near_zip(
        zip, radius, status, source_type, source_id, agent_id, search,
        limit, off, pool
    ).await?;

    // Map PostWithDistance → PostEdge, build PostConnection
    // has_next_page = (off + limit) < total_count
    // has_previous_page = off > 0
} else {
    // Existing cursor-based path (unchanged)
    // OR: if offset is provided, use offset-based path too (fixes latent bug)
}
```

### Phase 4: Restate Handler Update

**File:** `packages/server/src/domains/posts/restate/services/posts.rs`

Pass `zip_code`, `radius_miles`, and `offset` from request to activity. When zip path returns, map `PostWithDistance` to `PostResult` with `distance_miles: Some(distance)`.

**File:** `packages/server/src/domains/posts/restate/virtual_objects/post.rs`

Add to `PostResult`:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub distance_miles: Option<f64>,
```

### Phase 5: Admin Web UI

**File:** `packages/web/app/admin/(app)/posts/page.tsx`

Add filter state:

```typescript
const [zipCode, setZipCode] = useState("");
const [radiusMiles, setRadiusMiles] = useState<number>(25);
```

Add to the Restate request:

```typescript
const { data } = useRestate<PostList>("Posts", "list", {
  ...existingFilters,
  zip_code: zipCode || null,
  radius_miles: zipCode ? radiusMiles : null,
  ...pagination.variables,
});
```

UI additions:
- Zip code text input (5-digit, in the filter bar alongside existing filters)
- Radius dropdown: 5, 10, 25, 50 miles (default 25, next to zip input, disabled when no zip entered)
- When zip active: distance column in table (formatted as "X.X mi"), active filter chip "Near: {zip} ({radius} mi)" with X to clear
- Clearing zip resets pagination to page 1

**File:** `packages/web/lib/restate/types.ts`

Add to `PostResult`:

```typescript
export interface PostResult {
  // ...existing...
  distance_miles?: number;
}
```

## Edge Cases

| Case | Behavior |
|------|----------|
| Post has no locations | Excluded when zip filter active, included normally otherwise |
| Post has multiple locations within radius | Appears once at closest location's distance (`MIN` in `GROUP BY`) |
| Location has no postal_code | Excluded (INNER JOIN on zip_codes fails) |
| Location has non-MN postal_code | Excluded (not in zip_codes table) |
| Zip code not in `zip_codes` table | TerminalError: "Only Minnesota zip codes currently supported" |
| Zip provided, radius omitted | Default 25 miles |
| Radius > 100 miles | Capped at 100 |
| Zero results within radius | Empty list, total_count = 0 |
| Zip cleared in UI | Reverts to standard behavior, pagination resets |
| Offset sent without zip | Works (fixes existing pagination bug) |

## Files Changed

| File | Change |
|------|--------|
| New migration | `CREATE INDEX idx_zip_codes_lat_lng ON zip_codes(latitude, longitude)` |
| `packages/server/src/domains/posts/models/post.rs` | Add `published_at`/`updated_at` to `PostWithDistance`, add `PostWithDistanceAndCount`, add `find_paginated_near_zip` |
| `packages/server/src/domains/posts/activities/core.rs` | Branch in `get_posts_paginated` on zip presence, accept offset |
| `packages/server/src/domains/posts/restate/services/posts.rs` | Add `zip_code`, `radius_miles`, `offset` to `ListPostsRequest` |
| `packages/server/src/domains/posts/restate/virtual_objects/post.rs` | Add `distance_miles` to `PostResult` |
| `packages/web/app/admin/(app)/posts/page.tsx` | Add zip input, radius dropdown, distance column, filter chip |
| `packages/web/lib/restate/types.ts` | Add `distance_miles` to `PostResult` |

## What We're NOT Doing

- Not touching `find_near_zip` or `search_nearby` — different use case, leave alone
- Not adding `zip_code`/`location_city` to `PostResult` — YAGNI for v1, existing `location` field suffices
- Not adding `SCHEDULE_ACTIVE_FILTER` to admin query — admins see all posts
- Not using PostGIS — haversine + bounding box is sufficient at current scale (viable to 50k+ posts)
- Not creating a separate count query — `COUNT(*) OVER()` handles it in one pass

## References

- Brainstorm: `docs/brainstorms/2026-02-11-zip-code-proximity-filtering-brainstorm.md`
- Existing `find_near_zip`: `packages/server/src/domains/posts/models/post.rs:1270`
- Existing `find_paginated`: `packages/server/src/domains/posts/models/post.rs:465`
- `PostWithDistance` struct: `packages/server/src/domains/posts/models/post.rs:90`
- `PostResult` struct: `packages/server/src/domains/posts/restate/virtual_objects/post.rs:196`
- `ListPostsRequest`: `packages/server/src/domains/posts/restate/services/posts.rs:29`
- `useOffsetPagination`: `packages/web/lib/hooks/useOffsetPagination.ts`
- Admin posts page: `packages/web/app/admin/(app)/posts/page.tsx`
- Schedule-aware filter plan: `docs/plans/2026-02-11-feat-schedule-aware-post-filtering-plan.md`
