# Broadsheet Layout Editor

> **Pre-migration design doc.** Written when the plan routed all backend
> work through Restate. Restate was removed on 2026-03-17 (see
> `ARCHITECTURE_DECISIONS.md` Decision 4). References below to
> "Restate handlers / services" and `domains/*/restate/` directories
> correspond to Axum HTTP handlers in `src/api/routes/{domain}.rs` in
> the current codebase. Architectural intent is preserved.

**Status:** Implemented | **Priority:** Chunk 3 of Phase 4.2-4.3

## Overview

The edition detail page (`/admin/editions/[id]`) has been rewritten from a read-only list view into a full **drag-and-drop broadsheet layout editor**. Editors can visually arrange posts within a CSS grid that mirrors the broadsheet's row/slot structure.

## Layout Architecture

Each edition has **rows**, and each row has a **row template** that defines slot positions with weights:

```
Row: "Hero with Sidebar"
┌────────────────────────┬─────────┐
│ heavy (span 2)         │ light   │
│ [Feature Post]         │ [Post]  │
│                        │ [Post]  │  ← count=2 (stacked)
└────────────────────────┴─────────┘

Row: "Three Column"
┌──────────┬──────────┬──────────┐
│ medium   │ medium   │ medium   │
│ [Post]   │ [Post]   │ [Post]   │
└──────────┴──────────┴──────────┘
```

### CSS Grid

- 3-column grid: `grid-template-columns: repeat(3, 1fr)`
- Weight → column span: `heavy=2`, `medium=1`, `light=1`
- Each template slot is a droppable grid cell
- Posts stack vertically within a cell (up to `count` limit)

## Drag-and-Drop

Uses `@dnd-kit/core` with `useDraggable` (post cards) and `useDroppable` (grid cells + remove zone).

### DnD IDs

| Element | ID Format | Example |
|---------|-----------|---------|
| Draggable post | Slot UUID | `a1b2c3d4-...` |
| Droppable cell | `drop-{rowId}-{slotIndex}` | `drop-abc123-0` |
| Remove zone | `remove-zone` | `remove-zone` |

### Actions on Drop

- **Cell → Cell**: Calls `moveSlot(slotId, targetRowId, slotIndex)` to reposition a post
- **Cell → Remove Zone**: Calls `removePostFromEdition(slotId)` to unplace a post

## Auto-Review Transition

When an editor opens a `draft` edition, the page auto-calls `reviewEdition` to transition it to `in_review`. This uses a `useRef` guard to prevent double-firing in React strict mode.

## Row Management (non-DnD)

- **Reorder**: Up/down arrow buttons call `reorderEditionRows`
- **Delete**: X button calls `deleteEditionRow`
- **Add**: "+ Add Row" dropdown shows available row templates, calls `addEditionRow`

## Post Template Picker

Each slot card has a `<Select>` dropdown for changing the post's visual template (feature, gazette, bulletin, etc.) via `changeSlotTemplate`.

## Status-Aware Actions

| Edition Status | Available Actions |
|----------------|-------------------|
| `draft` / `in_review` | Regenerate, row/slot editing |
| `in_review` | Approve |
| `approved` | Publish |
| `published` | Archive |

## Backend Mutations Used

| Mutation | Purpose |
|----------|---------|
| `moveSlot` | Reposition a post between slots/rows |
| `addPostToEdition` | Place a new post (future: unplaced pool) |
| `addEditionRow` | Add a new row with template |
| `deleteEditionRow` | Remove a row |
| `reorderEditionRows` | Change row order |
| `changeSlotTemplate` | Change post visual template |
| `removePostFromEdition` | Unplace a post from a slot |
| `reviewEdition` | Auto-transition draft → in_review |
| `approveEdition` | Approve for publication |
| `publishEdition` | Go live |

## Files Modified

- `packages/admin-app/app/admin/(app)/editions/[id]/page.tsx` — Full rewrite
- `packages/admin-app/lib/graphql/editions.ts` — New mutation definitions (from Chunk 3 backend)
- `packages/shared/graphql/schema.ts` — New mutations (from Chunk 3 backend)
- `packages/shared/graphql/resolvers/edition.ts` — New resolvers (from Chunk 3 backend)
- `packages/server/src/domains/editions/restate/services/editions.rs` — New handlers (from Chunk 3 backend)

## Future Enhancements

- **Unplaced Posts Pool**: Drag posts from a pool of eligible-but-unplaced posts into slots
- **Slot validation**: Enforce `accepts` constraints (only allow certain post types in certain slots)
- **Widget support**: Non-post content like section headers and weather (Chunk 5)
