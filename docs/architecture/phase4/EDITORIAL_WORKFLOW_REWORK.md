# Editorial Workflow Rework — Edition Currency Model

> **Status:** Planning
> **Branch:** `feature/phase4-cms-experience`
> **Created:** 2026-03-01
> **Context:** Replaces week-scoped edition viewing with a county-currency model across all admin pages.

---

## Problem

The admin UI currently treats editions as week-scoped artifacts: every page (dashboard, kanban, editions list) asks "which week?" and filters by `periodStart`/`periodEnd`. This creates friction for the core editorial workflow:

1. **Editors don't care about weeks** — they care about whether each county's edition is current, reviewed, and live.
2. **Week navigation is meaningless** — there's only one actionable question: "has this county been reviewed yet?"
3. **Stale editions are invisible** — if an editor skips a week, the old edition silently persists. There's no visual signal that a county is falling behind.
4. **The "Live" column is in the wrong position** — it's first (leftmost), but it's the terminal state. Editorial flow reads left-to-right.

## Solution: County Currency Model

Replace week-scoped views with a **"latest edition per county"** model. Every view answers: **"What's the most recent edition for each county, and how current is it?"**

### Staleness Calculation

```
weeksOld = floor((today - periodEnd) / 7)
```

| `weeksOld` | Label | Visual Treatment |
|---|---|---|
| 0 | "Current" | Normal (stone border) |
| 1 | "1 week old" | Normal (stone border) |
| 2 | "2 weeks old" | Warning (amber border, amber text) |
| 3+ | "N weeks old" | Alert (red border, red text, warning icon) |

Staleness is computed client-side from `periodEnd`. No backend change needed for the calculation itself.

---

## Backend: `latestEditions` Query

### Why a new query

The existing `EditionsListQuery` filters by `periodStart`/`periodEnd` — it returns all editions for a time range. We need the **single most recent edition per county** regardless of when it was generated.

The existing `currentEdition(countyId)` resolver returns one county at a time — calling it 87 times is wasteful.

### Rust model method

```rust
// In packages/server/src/domains/editions/models/edition.rs
impl Edition {
    /// Returns the most recent edition for every county (one row per county).
    pub async fn latest_per_county(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT DISTINCT ON (county_id) *
             FROM editions
             ORDER BY county_id, period_start DESC, created_at DESC"
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}
```

### Restate handler

Add `latest_editions` to the existing `Editions` service — thin wrapper calling the model method.

### GraphQL schema addition

```graphql
type Query {
  latestEditions: [Edition!]!   # One per county, most recent
}
```

Resolver calls Restate `Editions/latest_editions`. No variables needed.

### Frontend query

```typescript
export const LatestEditionsQuery = graphql(`
  query LatestEditions {
    latestEditions {
      id
      county { id name }
      periodStart
      periodEnd
      status
      publishedAt
      rows { id }
      createdAt
    }
  }
`);
```

---

## Page-by-Page Changes

### 1. Review Board (Kanban) — `workflow/page.tsx`

**Priority: High** — this is the primary editorial workspace.

#### Column reorder

Current: `Live | Ready for Review | In Review | Approved`
New: `Ready for Review | In Review | Approved | Live`

Left-to-right follows the editorial flow. "Live" is the terminal/done state.

```typescript
const COLUMNS = [
  { id: "draft",      label: "Ready for Review", status: "draft",      color: "bg-kanban-draft-bg" },
  { id: "in_review",  label: "In Review",        status: "in_review",  color: "bg-kanban-review-bg" },
  { id: "approved",   label: "Approved",          status: "approved",   color: "bg-kanban-published-bg" },
  { id: "published",  label: "Live",              status: "published",  color: "bg-green-50" },
] as const;
```

#### Header

- Remove: week navigation (`< This Week >`, prev/next buttons, `weekOffset` state)
- Remove: `getWeekBounds()`, `formatWeekLabel()` helper functions
- Replace subtitle with progress: **"23 of 87 reviewed"** (count of approved + published vs total)
- Keep: "Review Board" title

#### Data source

- Replace: `EditionsListQuery` with `periodStart`/`periodEnd` variables
- With: `LatestEditionsQuery` (no variables, returns 87 editions)

#### Card content

- Replace: `formatPeriod(periodStart, periodEnd)` → "Feb 23 – Mar 1"
- With: staleness label → "Current", "1 week old", "3 weeks old"
- Add: visual escalation (amber/red border + text for stale editions)
- Keep: county name (primary), drag handle, Edit link

#### Sort order within columns

- Sort by staleness descending (most stale first), then alphabetically by county name
- Editors naturally triage the worst cases first

#### Batch action

- Keep: "Publish All" on Approved column (batch approve → live)
- Consider: "Review All" button to batch-move drafts to in_review

### 2. Dashboard (Cockpit) — `dashboard/page.tsx`

**Priority: High** — first thing editors see.

#### Header

- Remove: "Week of Feb 23 – Mar 1, 2026"
- Replace with: **"Edition Cockpit"** (already the h1) with subtitle **"23 of 87 counties up to date"**
- "Up to date" = status is `published` AND `weeksOld <= 1`

#### Stats cards

Current: 4 cards (Live, Ready for Review, In Review, Approved) showing counts for one week.
New: Same 4 cards but counting against latest-per-county data:

| Card | Counts | Color |
|---|---|---|
| Ready for Review | Drafts needing attention | Amber dot |
| In Review | Currently being checked | Blue dot |
| Approved | Reviewed, awaiting publish | Green dot |
| Live | Currently published | Green dot |

Add a 5th row or inline indicator: **"N counties stale (2+ weeks)"** if any exist — acts as a warning.

#### Edition table

- Remove: period date filter
- Replace with: table of latest editions sorted by staleness (most stale first)
- Columns: County | Status | Staleness | Rows | Actions
- Staleness column uses color-coded labels

#### Data source

- Replace: `EditionsListQuery` + `EditionKanbanStatsQuery` with period filters
- With: `LatestEditionsQuery` (derive stats client-side from the 87 results)

### 3. Editions List — `editions/page.tsx`

**Priority: Medium** — administrative/management view.

This page serves a different purpose than the kanban — it's for browsing, creating, and managing editions. It should keep more flexibility.

#### Changes

- Add: staleness label alongside the period date range (not replacing — both are useful here)
- Add: "Stale" filter option in status dropdown (client-side filter: `weeksOld >= 2`)
- Keep: county filter, status filter, batch generate form, create form
- Keep: period dates in create/batch forms (needed for generation)
- Consider: default sort by staleness instead of created date, with option to toggle

#### No data source change

Keep using `EditionsListQuery` — this page may need historical editions, not just the latest per county.

### 4. Edition Detail — `editions/[id]/page.tsx`

**Priority: Low** — context is already a single edition.

#### Changes

- Add: staleness badge next to the period dates in the header (e.g., "Feb 23 – Mar 1 · **3 weeks old**")
- Add: visual warning if edition is stale (amber/red banner at top)
- Keep: all existing functionality (broadsheet editor, widgets, status actions)

### 5. Posts — `posts/page.tsx`, `posts/[id]/page.tsx`

**Priority: None** — posts aren't tied to the edition currency model. No changes needed.

### 6. Sidebar — `AdminSidebar.tsx`

**Priority: Low** — minor enhancement.

- Consider: badge on "Review Board" nav item showing count of unreviewed editions (draft + in_review)
- Gives editors a persistent signal without opening the page

---

## Implementation Sequence

### Phase A: Backend (latestEditions query)

1. Add `Edition::latest_per_county()` model method
2. Add `latest_editions` Restate handler + activity
3. Add `latestEditions` GraphQL resolver
4. Add `LatestEditionsQuery` to frontend GraphQL definitions
5. Verify with test

### Phase B: Kanban rework (highest editorial impact)

1. Reorder columns (left-to-right editorial flow)
2. Replace data source (`LatestEditionsQuery`)
3. Remove week picker and helpers
4. Update header with progress subtitle
5. Add staleness calculation + labels to cards
6. Add visual escalation (amber/red styling)
7. Sort by staleness within columns
8. Verify drag-and-drop still works with new data source

### Phase C: Dashboard rework

1. Replace data source
2. Update stats cards to use latest-per-county counts
3. Replace week label with progress subtitle
4. Add staleness warning indicator
5. Update edition table with staleness column + sort

### Phase D: Editions list + detail polish

1. Add staleness labels to editions list
2. Add staleness badge to edition detail header
3. Add "Stale" filter to editions list

---

## Staleness Helper (shared)

```typescript
// packages/admin-app/lib/staleness.ts

export type StalenessLevel = "current" | "recent" | "warning" | "alert";

export function getWeeksOld(periodEnd: string): number {
  const end = new Date(periodEnd + "T23:59:59");
  const now = new Date();
  const diffMs = now.getTime() - end.getTime();
  return Math.max(0, Math.floor(diffMs / (7 * 24 * 60 * 60 * 1000)));
}

export function getStalenessLevel(weeksOld: number): StalenessLevel {
  if (weeksOld === 0) return "current";
  if (weeksOld === 1) return "recent";
  if (weeksOld === 2) return "warning";
  return "alert";
}

export function getStalenessLabel(weeksOld: number): string {
  if (weeksOld === 0) return "Current";
  if (weeksOld === 1) return "1 week old";
  return `${weeksOld} weeks old`;
}

export const STALENESS_STYLES: Record<StalenessLevel, { border: string; text: string; bg: string }> = {
  current: { border: "border-l-stone-300", text: "text-stone-500",  bg: "" },
  recent:  { border: "border-l-stone-300", text: "text-stone-500",  bg: "" },
  warning: { border: "border-l-amber-400", text: "text-amber-600",  bg: "bg-amber-50" },
  alert:   { border: "border-l-red-400",   text: "text-red-600",    bg: "bg-red-50" },
};
```

---

## Open Questions

1. **Should "Live" editions be draggable?** Probably not — once live, the only action is replacing with a newer edition. The Live column is read-only display.
2. **Archive flow:** When a new edition is approved→published, does the old published edition auto-archive? This likely already happens in the `publish` Restate handler but should be verified.
3. **Counties with no edition:** If a county has never had an edition generated, it won't appear in `latestEditions`. Should there be a "Missing" indicator? Or does batch generate always cover all 87?
4. **Batch generate interaction:** After running batch generate, all 87 counties get new draft editions. The kanban should immediately reflect this. Verify URQL cache invalidation with `additionalTypenames: ["Edition"]` works for the new query.
