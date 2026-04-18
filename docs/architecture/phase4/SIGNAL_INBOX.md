# Phase 4.3: Signal Inbox (Mock API)

> **Pre-migration design doc.** Written when the plan routed all backend
> work through Restate. Restate was removed on 2026-03-17 (see
> `../ARCHITECTURE_DECISIONS.md` Decision 4). References below to
> "Restate handlers / services" and `domains/*/restate/` directories
> correspond to Axum HTTP handlers in `src/api/routes/{domain}.rs` in
> the current codebase. Architectural intent is preserved.

**Status:** Plan
**Priority:** 3 of 4
**Depends on:** Phase 4.1 (Story Editor) — "Edit & Approve" flow requires the post editor

---

## Context

Root Signal is a separate system that will eventually deliver organized community content to Root Editorial for publication. Today, there is no concept of an "inbox" or content triage queue — posts only enter the system through admin creation (or the now-removed scraping pipeline).

This subproject builds the triage UI with mock data so the editorial workflow is ready when Root Signal's API is defined. The real integration will come later (Phase 4 of the pivot doc lists this as blocked on Craig's input), but the admin interface should be built now.

---

## Architecture Decisions

### 1. Signal items are posts, not a separate entity

Signal items fundamentally become posts when approved. Creating a separate `signal_items` table would duplicate the post schema and require conversion logic. Instead, signal content is stored as posts with `submission_type = 'signal'` and `status = 'pending_approval'`.

The inbox page is a filtered view: `WHERE submission_type = 'signal' AND status = 'pending_approval'`.

This reuses all existing post infrastructure (approval, rejection, tags, contacts, organization linking) with zero new tables.

### 2. The `submission_type` column accepts any text value

The CHECK constraint on `submission_type` was already dropped in migration `000163_add_revision_to_submission_type_check.sql`. The column accepts any text. No migration is needed to add the `'signal'` value.

### 3. Mock data as seeded posts, not a mock HTTP server

Since signal items are just posts, we seed them via the existing seed infrastructure. A `data/signal_items.json` file provides realistic mock content that gets inserted as posts with `submission_type = 'signal'`. No fake API server is needed — the data exercises the real code path.

### 4. Card-based triage UI, not a table

Triage requires quickly reading content, making a decision, and moving on. A card layout with content preview, metadata badges, and action buttons is more efficient than a table for this workflow. The existing `PostReviewCard` component (used on the posts page) is the reference pattern.

### 5. Inbox gets its own route, not a filter on the posts page

The inbox has a fundamentally different UX goal than the posts list. The posts page is for managing all content; the inbox is for triaging incoming content. Separate routes allow purpose-built UX without complicating the existing posts page.

### 6. Signal integration will be via webhook

> **See [ARCHITECTURE_DECISIONS.md](../ARCHITECTURE_DECISIONS.md), Decision 5.**

When Root Signal's API is ready, integration will be a simple webhook — not a shared Restate service or service mesh. Signal calls a webhook endpoint on Editorial's backend, which validates a shared secret and inserts a post with `submission_type='signal', status='pending_approval'`. The mock data approach (seeded `signal_items.json`) is correct for building the triage UI now; the real integration swaps the data source without changing the UI.

---

## Database Changes

**No migration needed.** The `submission_type` column already exists and accepts any text value.

### Signal-to-Type Mapping for Mock Data

> **From [CMS_SYSTEM_SPEC.md §10](../CMS_SYSTEM_SPEC.md#10-root-signal-integration).**

The mock data should reflect the expected mapping from Root Signal categories to post types:

| Root Signal Category | Post Type | Tags | Mock Data Guidance |
|---------------------|-----------|------|--------------------|
| Tension | `story` | topic-derived | LLM-drafted narrative around a community tension |
| Situation | `story` | topic-derived | Narrative wrapping multiple signals |
| Need | `exchange` | `need` + topic | Items/contact/status from signal data |
| Aid | `exchange` | `aid` + topic | Items/contact/status from signal data |
| Notice (low severity) | `notice` | topic | Standard informational post |
| Notice (high severity) | `notice` | `urgent` + topic | High-priority, source attribution |
| Gathering | `event` | topic, maybe `recurring` | Date/time/location from signal data |

The 10-15 mock items should cover all 6 post types with a realistic distribution: heavier on exchanges and notices (the most common signal types), lighter on stories and events.

### Origin Metadata

> **From [CMS_SYSTEM_SPEC.md §10](../CMS_SYSTEM_SPEC.md#10-root-signal-integration).**

Each signal-originated post can track its Root Signal origin for traceability. The inbox cards should display this when available:

```
origin: {
  signal_id:     string?    // Root Signal signal ID
  situation_id:  string?    // Root Signal situation ID
  generated:     boolean    // true if body text was LLM-drafted
  draft_source:  string?    // description of what data the LLM used
}
```

For mock data, populate `generated: true` and a `draft_source` like "Synthesized from 3 community reports and 1 official notice" to exercise the UI. The `signal_id` and `situation_id` can be fake UUIDs for now.

This metadata is informational — it helps the editor understand where the content came from and how much editorial judgment was applied upstream.

### Seed Data: `data/signal_items.json` (new file)

10–15 mock items representing the kinds of content Root Signal would deliver:

```json
[
  {
    "title": "Volunteers Needed: North Minneapolis Food Distribution",
    "description_markdown": "Community Partners is seeking **20 volunteers** for weekly food distribution...",
    "post_type": "exchange",
    "urgency": "high",
    "location": "Minneapolis, MN",
    "category": "food_assistance"
  },
  {
    "title": "City Council Votes to Expand Transit Routes",
    "description_markdown": "The Minneapolis City Council approved expansion of three bus routes...",
    "post_type": "story",
    "location": "Minneapolis, MN",
    "category": "government"
  }
  // ... 8-13 more items covering all 6 post types
]
```

### Seed Script Update: `data/seed.mjs`

Add a section that reads `signal_items.json` and inserts each item as a post with:
- `submission_type = 'signal'`
- `status = 'pending_approval'`
- `weight = 'medium'` (default)
- `priority = 0` (unranked, editor assigns)

---

## Backend Changes

### Model: `packages/server/src/domains/posts/models/post.rs`

**Already handled by Phase 4.1** — the Story Editor adds `submission_type` to `PostFilters` and updates `find_paginated`/`count_by_status` to filter on it.

If Phase 4.1 is not yet implemented, these changes must happen here instead:

Add to `PostFilters`:
```rust
pub submission_type: Option<&'a str>,
```

Update `find_paginated` forward/backward queries to add:
```sql
AND ($N::text IS NULL OR p.submission_type = $N)
```

Update `count_by_status` similarly.

### Restate: `packages/server/src/domains/posts/restate/services/posts.rs`

**Already handled by Phase 4.1** — `ListPostsRequest` gets a `submission_type` field.

No additional backend changes needed beyond Phase 4.1.

---

## GraphQL Changes

### Schema: `packages/shared/graphql/schema.ts`

**Already handled by Phase 4.1:**
- `submissionType: String` added to `posts` query args
- `submissionType: String` added to `Post` type

No additional schema changes needed.

---

## Frontend Changes

### New page: `packages/admin-app/app/admin/(app)/inbox/page.tsx`

The inbox page focused on triage workflow:

```
┌─────────────────────────────────────────────────────────┐
│  Signal Inbox                              14 items     │
│                                                         │
│  ┌─────────────────────────────────────────────────────┐│
│  │ 🔴 HIGH URGENCY                                    ││
│  │ Volunteers Needed: North Minneapolis Food Distrib...││
│  │ exchange · food_assistance · Minneapolis, MN        ││
│  │                                                     ││
│  │ Community Partners is seeking 20 volunteers for     ││
│  │ weekly food distribution at the Northside campus... ││
│  │                                                     ││
│  │         [Approve]  [Edit & Approve]  [Reject]       ││
│  └─────────────────────────────────────────────────────┘│
│                                                         │
│  ┌─────────────────────────────────────────────────────┐│
│  │ City Council Votes to Expand Transit Routes         ││
│  │ story · government · Minneapolis, MN                ││
│  │                                                     ││
│  │ The Minneapolis City Council approved expansion of  ││
│  │ three bus routes serving North and South...         ││
│  │                                                     ││
│  │         [Approve]  [Edit & Approve]  [Reject]       ││
│  └─────────────────────────────────────────────────────┘│
│                                                         │
│  [Load More...]                                         │
└─────────────────────────────────────────────────────────┘
```

**Behavior:**
- Fetches `posts(submissionType: "signal", status: "pending_approval")`
- Cards show: title, type badge, urgency badge (if present), location, truncated markdown preview (first ~150 chars rendered via ReactMarkdown)
- **Approve**: Calls `ApprovePostMutation` → card fades out, count decrements
- **Edit & Approve**: Navigates to `/admin/posts/[id]?edit=true` (edit mode from Phase 4.1)
- **Reject**: Confirmation prompt → calls `RejectPostMutation` → card fades out
- Offset pagination with "Load More" button
- Empty state: "No items in the inbox. New content will appear here when Root Signal delivers it."
- Item count in header updates as items are approved/rejected

### New component: `packages/admin-app/components/admin/InboxCard.tsx`

Compact triage card component:

**Props:**
- `post: InboxPost` — post data from query
- `onApprove: (id: string) => void`
- `onReject: (id: string) => void`
- `onEditApprove: (id: string) => void`

**Layout:**
- White card with rounded corners (stone/amber design system)
- Title as `<h3>` with urgency badge to the right (if present)
- Metadata row: post type badge, category, location
- Content preview: truncated markdown (ReactMarkdown with `max-h-20 overflow-hidden`)
- Action row: three buttons (Approve = green, Edit & Approve = amber, Reject = ghost/danger)
- Loading state on action buttons while mutation is in flight

### New query: `packages/admin-app/lib/graphql/inbox.ts`

```typescript
export const InboxQuery = graphql(`
  query Inbox($limit: Int, $offset: Int) {
    inbox: posts(
      submissionType: "signal"
      status: "pending_approval"
      limit: $limit
      offset: $offset
    ) {
      posts {
        id
        title
        description
        descriptionMarkdown
        summary
        postType
        urgency
        location
        category
        submissionType
        createdAt
      }
      totalCount
      hasNextPage
    }
  }
`);
```

Reuses existing `ApprovePostMutation` and `RejectPostMutation` from `lib/graphql/posts.ts`.

### Sidebar: `packages/admin-app/components/admin/AdminSidebar.tsx`

Add "Inbox" to the "Content" nav group, between "Posts" and "Editions":

```typescript
{
  name: "Inbox",
  href: "/admin/inbox",
  icon: InboxIcon,  // or use an SVG inbox icon
}
```

Optionally show a badge count. This requires a lightweight query or piggybacking on the dashboard data. For simplicity, skip the badge count initially and add it later if useful.

---

## Existing Code to Reuse

| What | Where | How |
|------|-------|-----|
| `PostReviewCard` | `admin-app/components/admin/PostReviewCard.tsx` | Reference pattern for card layout |
| `ApprovePostMutation` | `admin-app/lib/graphql/posts.ts` | Approve action |
| `RejectPostMutation` | `admin-app/lib/graphql/posts.ts` | Reject action |
| `PostListFields` fragment | `admin-app/lib/graphql/fragments.ts` | Post field selection |
| `ReactMarkdown` | `posts/[id]/page.tsx` | Markdown rendering in preview |
| `Badge` component | `admin-app/components/ui/Badge.tsx` | Type/urgency badges |
| `Button` component | `admin-app/components/ui/Button.tsx` | Action buttons |
| `AdminLoader` | `admin-app/components/admin/AdminLoader.tsx` | Loading state |
| `PaginationControls` | `admin-app/components/ui/PaginationControls.tsx` | Load more / pagination |

---

## Implementation Steps

1. **Prerequisite check**: Verify Phase 4.1 added `submissionType` to GraphQL schema and `submission_type` to `PostFilters` / `ListPostsRequest`. If not, do those changes first.
2. **Seed data**: Create `data/signal_items.json` with 10–15 mock items
3. **Seed script**: Update `data/seed.mjs` to insert signal items as posts
4. **Frontend**: Create `InboxCard` component
5. **Frontend**: Create `lib/graphql/inbox.ts` with `InboxQuery`
6. **Frontend**: Create `app/admin/(app)/inbox/page.tsx`
7. **Frontend**: Update `AdminSidebar.tsx` with Inbox nav item
8. **Codegen**: Run `yarn codegen` in admin-app
9. **Seed**: Run `make seed` (or equivalent) to populate mock data

---

## Verification

1. Run seed script — verify 10+ posts appear with `submission_type = 'signal'`
2. Navigate to `/admin/inbox` — mock items should appear as cards
3. Approve an item — card disappears, post appears in `/admin/posts` as active
4. Reject an item — card disappears, post appears in posts list as rejected
5. Click "Edit & Approve" — navigates to post detail in edit mode (requires Phase 4.1)
6. Verify count in header decrements after approve/reject
7. Verify empty state shows when all items are triaged
8. Verify sidebar "Inbox" link is present and active-highlighted
