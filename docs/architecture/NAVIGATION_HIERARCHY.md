# Navigation & Dashboard Re-orientation

**Status:** Implemented | **Priority:** Chunk 4 of Phase 4.2-4.3

## Overview

The admin dashboard and sidebar navigation have been re-oriented around the **broadsheet editorial workflow** rather than individual post management.

## Dashboard Changes

The dashboard (`/admin/dashboard`) was renamed to "Edition Cockpit" and redesigned around edition status for the current week.

### Edition Stats Cards (top)
| Card | Source | Color |
|------|--------|-------|
| Live | `editionKanbanStats.published` | Green |
| Ready for Review | `editionKanbanStats.draft` | Yellow |
| In Review | `editionKanbanStats.inReview` | Amber |
| Approved | `editionKanbanStats.approved` | Emerald |

### Alert Banner
When editions need review (draft + in_review > 0), a prominent amber banner links to the Review Board.

### Quick Actions
1. **Review Board** — link to `/admin/workflow`
2. **All Editions** — link to `/admin/editions`
3. **Batch Generate** — calls `batchGenerateEditions` for the current week

### Secondary Content
- **Pending Posts** — list of up to 5 posts needing approval (links to post detail)
- **Content Summary** — total posts, editions this week, published count, pending count

### Query Changes
The `DashboardQuery` now accepts `periodStart`/`periodEnd` variables and fetches `editionKanbanStats` alongside a lightweight post summary (no longer fetches 1000 posts).

## Sidebar Changes

### Review Board Badge
The sidebar's "Review Board" link now shows a badge with the count of editions needing review (draft + in_review for the current week). Uses `EditionKanbanStatsQuery` fetched in the sidebar component.

### Navigation Order (unchanged from Chunk 2)
```
Overview
  Dashboard

Content
  Review Board  [badge: editions needing review]
  Editions
  Posts (expandable: All, Stories, Notices, Exchanges, Events, Spotlights, References)
  Media

Sources
  Organizations

System
  Jobs
  Tags
```

## Files Modified

- `packages/admin-app/app/admin/(app)/dashboard/page.tsx` — Full rewrite
- `packages/admin-app/lib/graphql/dashboard.ts` — Updated query with period vars + edition stats
- `packages/admin-app/components/admin/AdminSidebar.tsx` — Added edition stats query + Review Board badge
