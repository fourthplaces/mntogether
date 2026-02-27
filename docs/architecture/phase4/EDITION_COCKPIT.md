# Phase 4.2: Edition Cockpit Dashboard

**Status:** Plan
**Priority:** 2 of 4
**Depends on:** Phase 4.1 (Story Editor) — the "New Post" quick action links to the editor

---

## Context

The current dashboard at `/admin/dashboard` is a placeholder. It fetches up to 1,000 posts client-side and computes basic stats (total posts, pending count, recent 7-day activity). It says "Overview of your community resources platform" — generic language from the pre-pivot era.

Editors need a dashboard that answers: *What is the current edition? How complete is it? What needs my attention?* This is the "edition cockpit" — the home screen for the weekly publication cycle.

---

## Architecture Decisions

### 1. Server-side aggregation via a new Restate handler

The current approach loads 1,000 posts into the browser and counts them in JavaScript. Edition completeness requires joining editions, rows, slots, and templates — too complex and slow for client-side computation. A new `dashboard_stats` handler in the Editions service computes this in SQL.

### 2. County selector in URL query parameter

Editors typically work on a specific county. A URL-based selector (`?county=UUID`) makes the dashboard bookmarkable. The default view (no county param) shows aggregate stats across all counties for the current period.

### 3. Auto-computed current period

The dashboard auto-computes the current publication week (Monday–Sunday) and uses it to filter editions. The editor doesn't need to pick a date range — they just open the dashboard and see "this week."

### 4. Extend EditionsService rather than creating a new Restate service

Dashboard stats are edition-centric data. Adding `dashboard_stats` to the existing EditionsService avoids registering a new service and keeps related functionality together.

---

## Database Changes

**No migration needed.** The aggregation queries use existing tables: `editions`, `edition_rows`, `edition_slots`, `row_template_slots`, `posts`, `counties`.

---

## Backend Changes

### Model: `packages/server/src/domains/editions/models/edition.rs`

Add a new struct and query:

```rust
/// Aggregated dashboard metrics for editions in a period.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EditionDashboardRow {
    pub edition_id: Uuid,
    pub county_id: Uuid,
    pub county_name: String,
    pub status: String,
    pub total_slots: i64,
    pub filled_slots: i64,
}

impl Edition {
    /// Compute edition dashboard stats for a given period, optionally filtered by county.
    pub async fn dashboard_stats(
        county_id: Option<Uuid>,
        period_start: NaiveDate,
        period_end: NaiveDate,
        pool: &PgPool,
    ) -> Result<Vec<EditionDashboardRow>> {
        sqlx::query_as::<_, EditionDashboardRow>(
            r#"
            SELECT
                e.id as edition_id,
                e.county_id,
                c.name as county_name,
                e.status,
                COALESCE(SUM(rts.count), 0)::bigint as total_slots,
                COUNT(DISTINCT es.id)::bigint as filled_slots
            FROM editions e
            JOIN counties c ON c.id = e.county_id
            LEFT JOIN edition_rows er ON er.edition_id = e.id
            LEFT JOIN row_template_slots rts ON rts.row_template_id = er.row_template_id
            LEFT JOIN edition_slots es ON es.edition_row_id = er.id
            WHERE e.period_start >= $1
              AND e.period_end <= $2
              AND ($3::uuid IS NULL OR e.county_id = $3)
            GROUP BY e.id, e.county_id, c.name, e.status
            ORDER BY c.name
            "#,
        )
        .bind(period_start)
        .bind(period_end)
        .bind(county_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}
```

### Activity: `packages/server/src/domains/editions/activities/dashboard.rs` (new file)

```rust
pub async fn compute_dashboard_stats(
    county_id: Option<Uuid>,
    period_start: NaiveDate,
    period_end: NaiveDate,
    deps: &ServerDeps,
) -> Result<DashboardStats> {
    let rows = Edition::dashboard_stats(county_id, period_start, period_end, &deps.db_pool).await?;
    let pending_count = Post::count_by_status(
        &PostFilters { status: Some("pending_approval"), ..Default::default() },
        &deps.db_pool,
    ).await?;

    // Compute aggregates
    let draft_count = rows.iter().filter(|r| r.status == "draft").count();
    let published_count = rows.iter().filter(|r| r.status == "published").count();
    let total_counties = rows.iter().map(|r| r.county_id).collect::<HashSet<_>>().len();

    Ok(DashboardStats {
        period_start,
        period_end,
        total_counties,
        draft_count,
        published_count,
        empty_count: 87 - total_counties,  // 87 MN counties
        pending_posts_count: pending_count,
        edition_summaries: rows,
    })
}
```

Register in `activities/mod.rs`.

### Restate: `packages/server/src/domains/editions/restate/services/editions.rs`

Add request/response types:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStatsRequest {
    pub county_id: Option<Uuid>,
    pub period_start: String,  // "2026-02-24"
    pub period_end: String,    // "2026-03-02"
}
impl_restate_serde!(DashboardStatsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStatsResult {
    pub period_start: String,
    pub period_end: String,
    pub total_counties: i32,
    pub draft_count: i32,
    pub published_count: i32,
    pub empty_count: i32,
    pub pending_posts_count: i64,
    pub edition_summaries: Vec<EditionSummaryResult>,
}
impl_restate_serde!(DashboardStatsResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditionSummaryResult {
    pub edition_id: Uuid,
    pub county_id: Uuid,
    pub county_name: String,
    pub status: String,
    pub total_slots: i64,
    pub filled_slots: i64,
    pub completeness: f64,  // filled_slots / total_slots
}
impl_restate_serde!(EditionSummaryResult);
```

Add `dashboard_stats` to `EditionsService` trait and impl. Parse date strings to `NaiveDate`, call the activity, return results.

---

## GraphQL Changes

### Schema: `packages/shared/graphql/schema.ts`

Add types:

```graphql
type EditionDashboard {
  periodStart: String!
  periodEnd: String!
  totalCounties: Int!
  draftCount: Int!
  publishedCount: Int!
  emptyCount: Int!
  pendingPostsCount: Int!
  editionSummaries: [EditionSummary!]!
}

type EditionSummary {
  editionId: ID!
  countyId: ID!
  countyName: String!
  status: String!
  totalSlots: Int!
  filledSlots: Int!
  completeness: Float!
}
```

Add to `Query`:
```graphql
editionDashboard(countyId: ID, periodStart: String, periodEnd: String): EditionDashboard!
```

### Resolver: `packages/shared/graphql/resolvers/edition.ts`

Add query resolver:

```typescript
editionDashboard: async (_parent, args, ctx) => {
  return ctx.restate.callService("Editions", "dashboard_stats", {
    county_id: args.countyId,
    period_start: args.periodStart,
    period_end: args.periodEnd,
  });
},
```

---

## Frontend Changes

### Replace: `packages/admin-app/app/admin/(app)/dashboard/page.tsx`

Complete rewrite. The new dashboard layout:

```
┌─────────────────────────────────────────────────────────┐
│  Edition Cockpit         [County: All ▼]  Week of Feb 24│
├─────────────┬──────────────┬──────────────┬─────────────┤
│  87 Counties│  12 Draft    │  3 Published │  72 Empty   │
│  Total      │  Editions    │  Editions    │  Counties   │
├─────────────┴──────────────┴──────────────┴─────────────┤
│  ⚠ 14 Posts Pending Review                    [Review →]│
├─────────────────────────────────────────────────────────┤
│  Quick Actions                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ Generate     │  │ New Post     │  │ View Site    │  │
│  │ Editions     │  │              │  │              │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
├─────────────────────────────────────────────────────────┤
│  Edition Status                                         │
│  ┌─────────────────────────────────────────────────────┐│
│  │ County      │ Status   │ Completeness │ Actions     ││
│  │ Hennepin    │ ● Draft  │ ████████░░ 80%│ [Edit]     ││
│  │ Ramsey      │ ● Published│ ██████████ 100%│ [View]  ││
│  │ Dakota      │ ● Draft  │ ██░░░░░░░░ 20%│ [Edit]    ││
│  └─────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────┘
```

**Components:**
- **Period header**: Auto-computed current week (Mon–Sun), display date range
- **County selector**: Dropdown from `CountiesQuery`, defaults to "All Counties"
- **Stats cards**: Total counties, drafts, published, empty — colored badges
- **Pending alert**: Amber banner with count and link to `/admin/posts?status=pending_approval`
- **Quick actions**: Generate Editions (links to `/admin/editions`), New Post (links to `/admin/posts/new`), View Site (external link)
- **Edition table**: Per-county status with completeness bar, link to edit/view edition

### Replace query: `packages/admin-app/lib/graphql/dashboard.ts`

```typescript
export const EditionDashboardQuery = graphql(`
  query EditionDashboard($countyId: ID, $periodStart: String!, $periodEnd: String!) {
    editionDashboard(countyId: $countyId, periodStart: $periodStart, periodEnd: $periodEnd) {
      periodStart
      periodEnd
      totalCounties
      draftCount
      publishedCount
      emptyCount
      pendingPostsCount
      editionSummaries {
        editionId
        countyId
        countyName
        status
        totalSlots
        filledSlots
        completeness
      }
    }
  }
`);
```

Reuse `CountiesQuery` from `lib/graphql/editions.ts` for the county dropdown.

---

## Existing Code to Reuse

| What | Where | How |
|------|-------|-----|
| `Edition::list` + `EditionFilters` | `edition.rs:96` | Pattern for filtered queries |
| `Post::count_by_status` | `post.rs:1098` | Pending posts count |
| `CountiesQuery` | `admin-app/lib/graphql/editions.ts` | County dropdown data |
| `EditionsListQuery` | `admin-app/lib/graphql/editions.ts` | Pattern for edition queries |
| `AdminLoader` | `admin-app/components/admin/AdminLoader.tsx` | Loading state |
| `Badge` component | `admin-app/components/ui/Badge.tsx` | Status badges |
| `PaginationControls` | `admin-app/components/ui/PaginationControls.tsx` | If edition table needs paging |
| Edition resolver patterns | `resolvers/edition.ts` | `callService("Editions", ...)` |

---

## Implementation Steps

1. **Model**: Add `EditionDashboardRow` struct and `Edition::dashboard_stats` query
2. **Activity**: Create `activities/dashboard.rs` with `compute_dashboard_stats`
3. **Activity**: Register in `activities/mod.rs`
4. **Restate**: Add request/response types and `dashboard_stats` handler to `EditionsService`
5. **GraphQL**: Add `EditionDashboard`, `EditionSummary` types and `editionDashboard` query to `schema.ts`
6. **GraphQL**: Add resolver in `resolvers/edition.ts`
7. **Frontend**: Replace `lib/graphql/dashboard.ts` with new query
8. **Frontend**: Rewrite `dashboard/page.tsx` with cockpit layout
9. **Codegen**: Run `yarn codegen` in admin-app
10. **Rebuild**: `docker compose up -d --build server`

---

## Verification

1. Dashboard loads without errors, shows current week dates
2. Stats cards show correct counts (compare against `/admin/editions` page)
3. County selector filters the edition table to one county
4. "All Counties" shows aggregate view
5. Pending posts count matches actual pending posts
6. Quick action links navigate correctly
7. Completeness bars reflect actual slot fill rates
8. "Edit" links on draft editions navigate to `/admin/editions/[id]`
