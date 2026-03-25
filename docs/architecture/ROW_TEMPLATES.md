# Row Templates — Current Architecture

> Status: Living document. Last updated 2026-03-24.

## Overview

The broadsheet layout is a 6-column CSS grid. Each **row template** defines a grid shape (layout variant), a set of **slots** that posts fill, and rules about which post weights and templates go where. The system has three layers:

1. **Row template** — the grid layout + slot recipe ("Feature + 3 tickers in sidebar")
2. **Layout variant** — the CSS grid shape (shared across templates)
3. **Post template** — the visual treatment of a single post (feature, gazette, ticker, etc.)
4. **Post type** — the content kind (story, notice, event, exchange, spotlight, reference)

---

## Weight System

Weight is the primary constraint axis. It's a strict chain:

```
post.weight → slot.weight → post_template.weight
```

Three tiers:

| Weight | Role | Body limits | Examples |
|--------|------|-------------|----------|
| **heavy** | Lead/hero content | 400–600 chars | feature, feature-reversed |
| **medium** | Standard content | 160–280 chars | gazette, bulletin, card-event, pinboard, directory-ref, alert-notice, generous-exchange, spotlight-local |
| **light** | Compact/sidebar | 0–160 chars | ticker, digest, ledger, whisper-notice, quick-ref, ticker-update |

Weight matching is **strict** in the layout engine — no cross-weight placement. A light post never fills a medium slot.

---

## Layout Variants (7 grid shapes)

| Variant | Grid columns | Description | Use case |
|---------|-------------|-------------|----------|
| `lead-stack` | 4 + 2 | Wide lead column + narrow stacked sidebar | Hero with supporting tickers/digests |
| `full` | 6 | Single full-width column | Standalone features, ticker strips |
| `trio` | 2 + 2 + 2 | Three equal columns | Medium-weight balanced rows |
| `lead` | 4 + 2 | Wide + narrow (non-stacking) | Feature + single companion |
| `pair` | 3 + 3 | Two equal columns | Balanced two-column content |
| `pair-stack` | 3 + 3 (second stacks) | One lead + stacked posts | Lead with supporting stack |
| `widget-standalone` | N/A | No grid wrapper | Section breaks, ads |

---

## Row Templates (34 total)

Each template has a **slug**, a **layout_variant**, a **sort_order** (lower = preferred), and one or more **slots**.

### Slot definitions

Each slot specifies:
- **slot_index** — position within the row (0-based)
- **weight** — required post weight (heavy/medium/light)
- **count** — how many posts this slot holds
- **post_template_slug** — default post template for posts in this slot
- **accepts** — type filter (currently NULL everywhere — unused)

### Span constraints

Certain template families are designed for specific column widths:

| Template family | Valid spans | Notes |
|----------------|------------|-------|
| **Ticker** | **full-width only** (span-6) | Single-line items designed for full row width. Never use in trio/pair/lead columns. |
| Feature Hero/Photo | span-6 | Full-width visual elements |
| Feature Story/Editorial | span-4 | Wide lead column |
| Gazette, Bulletin | span-2, span-3, span-4 | Flexible column content |
| Ledger, Digest | span-2 | Compact sidebar/stacked items |

### Key templates by priority

| sort_order | Slug | Variant | Slots summary |
|-----------|------|---------|---------------|
| 1 | hero-with-sidebar | lead-stack | 1 heavy feature + 3 light digests |
| 2 | hero-full | full | 1 heavy feature |
| 3 | three-column | trio | 3 medium gazettes |
| 4 | two-column-wide-narrow | lead | 1 heavy feature + 1 medium bulletin |
| 6 | classifieds | trio | 6 light digests |
| 7 | ticker | full | 8 light tickers |

---

## Post Templates (16 total)

| Slug | Weight | Compatible types | Body target/max |
|------|--------|-----------------|-----------------|
| feature | heavy | story, event, spotlight | 400/600 |
| feature-reversed | heavy | notice | 200/280 |
| gazette | medium | story, notice, exchange, event, spotlight, reference | 200/280 |
| bulletin | medium | notice, exchange, event, reference, spotlight | 180/240 |
| alert-notice | medium | notice | 180/240 |
| pinboard-exchange | medium | exchange | 180/240 |
| card-event | medium | event | 160/220 |
| directory-ref | medium | reference | 0/0 |
| generous-exchange | medium | exchange | 180/240 |
| spotlight-local | medium | spotlight | 180/240 |
| ledger | light | notice, exchange, event, reference | 120/160 |
| ticker | light | notice, exchange, event | 0/0 |
| digest | light | story, notice, exchange | 0/0 |
| whisper-notice | light | notice | 120/160 |
| quick-ref | light | reference | 0/0 |
| ticker-update | light | notice | 0/0 |

---

## Layout Engine Algorithm

The Rust layout engine (`layout_engine.rs`) is a greedy forward-only algorithm with no backtracking.

### Step 1: Load eligible posts

- County-relevant posts via location joins, plus statewide posts
- Filters: `status = 'active'`, `published_at` within 7 days of edition period
- Ordered by `priority DESC NULLS LAST`
- Topic tags loaded for section grouping

### Step 2: Select row templates (max 12 rows)

**Phase 1 — Hero rows** (up to 3):
- Iterates templates with heavy slots
- Scores each by how many slots it can fill from remaining posts
- Avoids consecutive identical templates
- Picks highest-scoring template, deducts consumed posts

**Phase 2 — Remaining rows** (up to 12 minus hero count):
- Same scoring but excludes heavy-slot templates if hero rows already placed
- Variety adjustments:
  - **0.4x penalty** for repeating the previous row's slug
  - **0.3x penalty** for layout variants used 2+ times
  - **1.5x bonus** for templates not yet used in the edition

### Step 3: Fill slots

For each row template, iterates slots in order:
- For each slot, iterates posts in priority order
- Requires **strict weight match** (`post.weight == slot.weight`)
- Resolves post template: prefers the slot's `post_template_slug` if compatible with post type, otherwise falls back to `find_compatible_post_template`
- Fills up to `slot.count` posts per slot
- Posts that don't match any remaining slot are **silently dropped**

### Step 4: Order rows

Rows sorted by `max_priority` descending — highest-priority content goes to the top.

### Step 5: Topic sections

Groups consecutive rows sharing the same dominant topic into `BroadsheetSection` objects. Rows without a clear topic break section continuity.

---

## Frontend Rendering

### Row rendering pipeline

1. `BroadsheetRenderer` receives rows from GraphQL, sorted by `sortOrder`
2. For each row, `getRowLayout()` maps `layoutVariant` to CSS grid config (cell spans + posts-per-cell)
3. `distributeSlots()` assigns posts to cells based on `slotIndex`
4. **Empty cell check**: Rows where any cell has zero posts are **skipped entirely**
5. `Row.tsx` renders `<div class="row row--{variant}">` with `Cell` children
6. Each `Cell` renders `<div class="cell cell--span-{N}">`

### Post preparation (`prepare.ts`)

- Selects body text tier based on post template weight
- Applies line clamping per template (tickers=2, bulletins=3, digests/ledgers=4, gazettes=6, features=unlimited)
- Features get paragraph splitting, drop caps, optional 2-column layout
- Derives render hints (date formatting, person info, tags, labels)

### Template resolution (`templates.ts`)

Registry of `(postTemplate, postType) → ReactComponent`. ~40 specific components covering every combination. Falls back to gazette variants.

---

## Known Issues & Gaps

1. **`accepts` column unused** — All slot `accepts` values are NULL. Type filtering happens indirectly through `post_template_slug`'s `compatible_types`.

2. **Body limit mismatch** — `prepare.ts` has hardcoded `TEMPLATE_CONFIGS` with different body limits than the database `post_template_configs` table. Frontend uses its own values.

3. **Partial row mismatch** — The layout engine includes rows with at least 1 filled slot. The frontend skips rows where any cell is empty. This causes backend-included, frontend-hidden rows.

4. **No height balancing** — Nothing ensures columns within a row have similar visual heights. A `pair` row can have one tall post and one short post, leaving large whitespace.

5. **Greedy placement** — No backtracking means suboptimal post-to-slot assignments. A post that would perfectly fit a later row may get placed (or dropped) early.

6. **`schedules` table deprecated** — The old polymorphic `schedules` table exists in the DB but has zero references in frontend or backend code. Candidate for `DROP TABLE` in a future migration.
