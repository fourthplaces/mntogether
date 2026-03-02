# Phase 4.2: CMS UX Rework — Editorial Workflow Orientation

**Date:** 2026-03-02
**Branch:** `feature/phase4-cms-experience`
**Status:** Frontend chunks 1–5 complete, fixes applied. Backend chunks 6–7 deferred.

---

## Problem Statement

The CMS was built bottom-up: data models first, then CRUD pages for each model. But the editorial workflow is top-down:

1. Root Signal generates posts weekly
2. Batch generate creates 87 broadsheet editions (one per county)
3. Editors review posts within each edition
4. Editors review broadsheet layouts
5. Approve, then deliberately publish

The UI reflected the data model, not the workflow. Specific issues:

- **Kanban had a "Live" column** that made publishing feel like a drag gesture rather than a deliberate action. Editors could accidentally drag an edition to "published."
- **"Current" staleness labels on kanban cards** didn't make sense in a workflow context. The kanban is about throughput, not data freshness.
- **Editions page showed raw date ranges** instead of contextual labels ("This week", "3 weeks ago"). No visual indication of staleness.
- **"Draft" status label for editions** confused editors because "draft" means something specific for posts (human-written content). Editions are auto-generated, never "drafted."
- **Posts were a top-level sidebar item** showing ALL posts across ALL editions. Posts are children of editions, not a standalone concern — but they also need to be accessible for statewide posts.
- **Post scoring was broken** ("Scored 0 posts, 123 failed") and obsoleted by Root Signal's relevance scoring.
- **No way to view posts within an edition.** Clicking an edition went straight to the broadsheet layout editor with no way to see its posts in a reviewable grid.
- **Dashboard was data-centric** instead of answering "what do I need to do next?"
- **Sidebar organized by data type** (Posts, Editions, Media) rather than editorial workflow.

---

## What Changed

### Chunk 1: Kanban Rework

**Goal:** Remove Live column, make publishing deliberate, simplify cards.

| File | Change |
|------|--------|
| `workflow/page.tsx` | Removed `published` column from COLUMNS array. 3 columns: Ready for Review, In Review, Approved. Removed `PublishEditionMutation` single-card mutation. Moved "Publish All" to header button. Added workflow guidance messages. |
| `EditionKanbanCard.tsx` | Removed staleness imports/labels/border colors. Changed `EditionCardData` interface: `filledSlots`/`totalSlots` replaced with `rowCount`. Card now shows: drag handle, county name, row count, hover edit link. |
| `EditionKanbanColumn.tsx` | Removed `action` prop (was used for Publish button inside Approved column). |

**Key decision:** Publishing is a header-level batch action ("Publish 12 Approved"), not a drag target. This prevents accidental publishing and makes the kanban purely about editorial review throughput.

### Chunk 2: Remove Post Scoring

**Goal:** Rip out all scoring UI. Root Signal handles relevance now.

| File | Change |
|------|--------|
| `lib/graphql/fragments.ts` | Removed `relevanceScore` and `relevanceBreakdown` from `PostListFields` and `PostDetailFields` fragments. |
| `lib/graphql/posts.ts` | Removed `BatchScorePostsMutation`. |
| `PostReviewCard.tsx` | Removed `relevanceScore` from interface, `getScoreColor` function, score badge display. |
| `posts/page.tsx` | Removed `ScoreFilter` type, score state, `handleScoreAll`, `matchesScoreFilter`, score filter dropdown, "Score All" menu, score result banner, "Review Tips" section, entire "more" menu. |
| `posts/[id]/page.tsx` | Removed relevance score section (score badge, breakdown text). |
| `organizations/[id]/page.tsx` | Removed `BatchScorePostsMutation` import, scoring state/handler, "Score N Unscored" button, score result banner, `relevanceScore` from PostData type and PostRow display. |

### Chunk 3: Sidebar Restructure

**Goal:** Reorganize navigation around editorial workflow.

| Before | After |
|--------|-------|
| Content > Posts (with 7 expandable children), Editions, Media | Editorial > Review Board, Editions, Posts, Media |
| Separate PostStatsQuery + EditionKanbanStatsQuery for badges | Single LatestEditionsQuery for Review Board badge |
| ~500 lines with expandable tree nav | ~280 lines with flat links |

**Key decision:** Posts kept in sidebar (restored after initial removal) for statewide post access. But the primary post review flow is through editions.

### Chunk 4: Editions Page Rework

**Goal:** Contextual period labels, staleness visualization, simplified generation.

| File | Change |
|------|--------|
| `lib/staleness.ts` | Added `formatPeriodLabel()` ("This week", "Last week", "3 weeks ago"). |
| `editions/page.tsx` | Removed Create Edition form entirely. Simplified Batch Generate to one-click "Generate This Week" (auto-calculates Mon-Sun). Updated status filters: All, Ready for Review, In Review, Approved, Published, Archived. Period column uses `formatPeriodLabel()` with staleness-driven text colors (stone, amber, red) and warning icon for 3+ weeks. |

### Chunk 5: Dashboard Rework

**Goal:** Weekly cockpit answering "what do I need to do next?"

| File | Change |
|------|--------|
| `lib/graphql/dashboard.ts` | Removed `pendingPosts` and `allPosts` queries. Dashboard now only queries `latestEditions`. |
| `dashboard/page.tsx` | Renamed "Edition Cockpit" to "Dashboard". Added one-click "Generate This Week" in header. Added workflow guidance banner (action/progress/ready/success tones). Stat cards follow pipeline order: Ready for Review, In Review, Approved, Published. Removed stale editions warning (belongs on Editions page). Removed pending posts panel. Quick actions reduced from 3 to 2. |

### Fix Pass: Status Labels, Posts Sidebar, Edition Detail Tabs

Three issues identified after initial chunks:

**1. Edition "Draft" label renamed to "Ready for Review"**

Editions are auto-generated, never "drafted." The backend status value stays `draft` but the UI displays "Ready for Review" everywhere:

| File | Change |
|------|--------|
| `editions/page.tsx` | Status filter label and badge label changed. |
| `editions/[id]/page.tsx` | `StatusBadge` labels map updated. |
| `workflow/page.tsx` | Already said "Ready for Review" (no change needed). |
| `dashboard/page.tsx` | Already said "Ready for Review" (no change needed). |

**2. Posts restored to sidebar**

Posts were initially removed entirely from the sidebar. Restored under Editorial group (Review Board, Editions, Posts, Media) for statewide post access.

**3. Layout | Posts tabs added to edition detail**

| File | Change |
|------|--------|
| `editions/[id]/page.tsx` | Refactored: lifted edition query to parent `EditionDetailPage`, which renders a tab bar (Layout / Posts). `BroadsheetEditor` now receives `edition` and `refetchEdition` as props. New `EditionPostsView` component extracts posts from edition slots and renders a 2-column card grid with post type, weight, row template, and status badges. Auto-review effect (draft → in_review on open) moved to parent. |

---

## Architecture Decisions

### 1. Presentation-layer status rename, not backend migration

The backend stores edition status as `draft` because that's what the migration defined. Rather than running a database migration to rename `draft` → `ready_for_review`, we kept the backend value and added a display label mapping in the frontend. This avoids:
- A migration on a live database
- Updating all Rust model methods that check `status == "draft"`
- Updating all GraphQL resolvers and frontend queries
- Risk of breaking the kanban drag-and-drop transitions

### 2. Edition query lifted to parent, not separate route segments

The plan originally called for `layout.tsx` + `page.tsx` + `posts/page.tsx` as separate Next.js route segments. Instead, we used client-side tab state within a single `page.tsx`. This was simpler because:
- The edition data is already loaded (no additional query needed for posts tab)
- Posts come from the edition's slot data, not a separate GraphQL query
- Tab switching is instant (no route navigation, no loading states)
- The broadsheet editor has complex DnD state that would be lost on route change

### 3. Posts kept in sidebar as flat link

The initial plan removed Posts from the sidebar entirely, reasoning that posts are children of editions. But statewide posts (not tied to any county edition) still need a top-level access point. Posts was restored as a flat link under Editorial, alongside Review Board, Editions, and Media.

### 4. Staleness visualization split between pages

Staleness is shown in two places with different purposes:
- **Editions page:** Contextual period labels with color escalation (stone → amber → red). This helps editors find stale editions that need regeneration.
- **Dashboard:** No staleness. The dashboard focuses on current workflow state (how many editions are in each stage). Staleness is a data quality concern, not a workflow concern.

---

## Files Modified (This Session)

| File | Lines | Nature |
|------|-------|--------|
| `app/admin/(app)/workflow/page.tsx` | ~250 | Rewritten (Chunk 1) |
| `components/admin/EditionKanbanCard.tsx` | ~96 | Rewritten (Chunk 1) |
| `components/admin/EditionKanbanColumn.tsx` | ~68 | Edited (Chunk 1) |
| `lib/graphql/fragments.ts` | ~2 fields removed | Edited (Chunk 2) |
| `lib/graphql/posts.ts` | ~1 mutation removed | Edited (Chunk 2) |
| `components/admin/PostReviewCard.tsx` | ~10 lines removed | Edited (Chunk 2) |
| `app/admin/(app)/posts/page.tsx` | ~300 | Rewritten (Chunk 2) |
| `app/admin/(app)/posts/[id]/page.tsx` | ~15 lines removed | Edited (Chunk 2) |
| `app/admin/(app)/organizations/[id]/page.tsx` | ~30 lines removed | Edited (Chunk 2) |
| `components/admin/AdminSidebar.tsx` | ~280 | Rewritten (Chunk 3) |
| `lib/staleness.ts` | +6 lines | Edited (Chunk 4) |
| `app/admin/(app)/editions/page.tsx` | ~320 | Rewritten (Chunk 4) |
| `lib/graphql/dashboard.ts` | ~17 | Simplified (Chunk 5) |
| `app/admin/(app)/dashboard/page.tsx` | ~250 | Rewritten (Chunk 5) |
| `app/admin/(app)/editions/[id]/page.tsx` | +~100 lines | Refactored (Fix pass) |

---

## What's Left (Deferred to Next Session)

### Chunk 6: Post Status Expansion (Backend)

Add review workflow statuses to post model:

```
draft → pending → in_review → approved → rejected
                                  │
                                  └→ active (when edition publishes)
```

Requires: SQL migration, Rust model changes, GraphQL schema update.

### Chunk 7: Edition Detail — Scoped Post Review

The current Posts tab extracts posts from edition slots. The full vision includes:
- Status filter tabs within the edition posts view (Pending, In Review, Approved, Rejected)
- Approve/reject actions per post within the edition context
- A new `editionPosts` GraphQL query that includes unplaced posts eligible for the county
- Post status transitions that work within the edition context (requires Chunk 6)

---

## Verification

- `npx graphql-codegen` — passes
- `npx tsc --noEmit` — 0 errors
- Preview server — no console errors, no server errors, no failed network requests
- All pages manually verified: Dashboard, Editions, Edition Detail (both tabs), Workflow/Kanban, Posts
