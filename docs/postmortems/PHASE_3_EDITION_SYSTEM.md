# Phase 3 Postmortem: Edition / Broadsheet System

**Date**: 2026-02-25
**Commit**: Uncommitted (pending review)
**Stats**: ~30 files changed/created, 4,024 lines added

---

## What This Was

Phase 3 of the Root Editorial Pivot — build the edition/broadsheet system that makes Root Editorial a newspaper CMS rather than a flat post list. An "edition" is a county-specific weekly collection of posts arranged in rows and slots, like a newspaper front page. The system must scale to **87 MN counties**, each receiving its own weekly edition (~4,500 editions/year).

This phase built the full data layer: database tables, Rust models, layout engine, Restate service, GraphQL schema, and minimal admin pages. Full CMS drag-drop UX is deferred to Phase 4.

## Scope: Plan vs. Reality

**Planned** (from the Phase 3 plan):
- Migration with 8 new tables + seed data (87 counties, ~900 zip mappings, 8 row templates, 7 post templates)
- Rust models for all config + edition CRUD
- Layout engine activity (the "deliberately dumb" placement algorithm from CMS_SYSTEM_SPEC.md)
- Restate service with 16 handlers
- GraphQL schema (5 types, 3 queries, 9 mutations)
- Admin pages: editions list, edition detail, generate page

**Actual**:
- All of the above, minus the standalone generate page (batch generation is integrated into the list page instead)
- 4 bugs discovered and fixed during live browser testing (see Surprises below)
- No scope creep — Phase 3 stayed on target

## Inventory of Changes

### Migration: `000174_phase3_edition_system.sql` (308 lines)

| Section | What it does |
|---------|-------------|
| `counties` | 87 MN counties with FIPS codes and centroid lat/lng |
| `zip_counties` | ~900 zip-to-county mappings (PK on zip_code + county_id) |
| `row_template_configs` | 8 row layouts: hero-with-sidebar, hero-full, three-column, etc. |
| `row_template_slots` | Slot definitions per template (weight + count + type filters) |
| `post_template_configs` | 7 visual treatments: feature, gazette, ledger, bulletin, etc. |
| `editions` | Per-county per-period editions (draft/published/archived) |
| `edition_rows` | Ordered rows within an edition, each using a row template |
| `edition_slots` | Posts placed in row slots with a post template assignment |
| Indexes | 9 indexes on foreign keys, status, period range, zip lookups |

### New Rust Domain: `domains/editions/` (18 files, 2,052 lines)

| Layer | Files | Lines | What |
|-------|-------|-------|------|
| `models/` | 8 | 767 | County, ZipCounty, RowTemplateConfig, RowTemplateSlot, PostTemplateConfig, Edition, EditionRow, EditionSlot |
| `activities/` | 3 | 470 | Layout engine (placement algorithm), edition ops (create/generate/publish/archive/batch) |
| `restate/services/` | 3 | 749 | EditionsService with 16 handlers + private helpers |
| `data/` | 2 | 49 | BroadsheetDraft, BroadsheetRow, BroadsheetSlot, LayoutPost types |
| `mod.rs` files | 2 | 17 | Module wiring |

### Rust Models (8 new)

| Model | File | Lines | Key methods |
|-------|------|-------|-------------|
| `County` | `county.rs` | 53 | `find_all`, `find_by_id` |
| `ZipCounty` | `zip_county.rs` | 50 | `find_counties_for_zip`, `find_zips_for_county` |
| `RowTemplateConfig` | `row_template_config.rs` | 73 | `find_all_with_slots` (loads slots eagerly) |
| `RowTemplateSlot` | `row_template_slot.rs` | 52 | `find_by_template` |
| `PostTemplateConfig` | `post_template_config.rs` | 55 | `find_all`, `find_by_slug`, `is_compatible` |
| `Edition` | `edition.rs` | 180 | `create`, `find_by_id`, `find_current_for_county`, `publish`, `archive`, `list` with pagination |
| `EditionRow` | `edition_row.rs` | 107 | `create`, `find_by_edition`, `update`, `reorder`, `delete` |
| `EditionSlot` | `edition_slot.rs` | 189 | `create`, `find_by_row`, `find_by_row_with_posts` (JOIN), `move_to`, `replace_for_row`, `delete` |

### Layout Engine: `activities/layout_engine.rs` (282 lines)

The core placement algorithm, adapted from CMS_SYSTEM_SPEC.md:

1. **Load county posts** — SQL joins through `post_locations` > `locations` > `zip_counties` to find posts relevant to a county. Statewide posts (no location or tagged `statewide`) are included for all counties.
2. **Select row templates** — greedy heuristic scores each template by how many remaining posts it can consume, weighted by heavy/medium/light distribution.
3. **Fill slots** — greedily assigns highest-priority posts to compatible slots (weight match + type filter).
4. **Sort rows** — rows ordered by highest-priority post within each.

Pure function (no I/O) for the placement logic; only `load_county_posts` and `generate_broadsheet` touch the database.

### Restate Service: 16 handlers

| Handler | What it does |
|---------|-------------|
| `list_counties` | Returns all 87 counties |
| `get_county` | Single county by ID |
| `list_editions` | Paginated list with county/status filters |
| `get_edition` | Full edition with rows, slots, and embedded post data |
| `current_edition` | Latest published/draft edition for a county |
| `create_edition` | Creates a draft edition for a county + date range |
| `generate_edition` | Runs layout engine, populates/replaces rows and slots |
| `publish_edition` | Sets status to published + timestamp |
| `archive_edition` | Sets status to archived |
| `batch_generate` | Creates + generates editions for all 87 counties |
| `row_templates` | Returns all row template configs with slots |
| `post_templates` | Returns all post template configs |
| `update_edition_row` | Change a row's template or sort order |
| `reorder_rows` | Bulk reorder rows within an edition |
| `remove_post` | Delete a slot (spike a post from the edition) |
| `change_slot_template` | Change a slot's visual treatment |

### GraphQL Schema Additions

| Type | Fields |
|------|--------|
| `County` | id, fipsCode, name, state |
| `Edition` | id, county, title, periodStart, periodEnd, status, publishedAt, rows, createdAt |
| `EditionRow` | id, rowTemplate, sortOrder, slots |
| `EditionSlot` | id, post, postTemplate, slotIndex |
| `EditionConnection` | editions, totalCount |
| `RowTemplate` | id, slug, displayName, description, slots |
| `RowTemplateSlotDef` | slotIndex, weight, count, accepts |
| `PostTemplateConfig` | id, slug, displayName, compatibleTypes, bodyTarget, bodyMax, titleMax |
| `BatchGenerateEditionsResult` | editionsCreated, editionsFailed, errors |

Plus 3 queries, 9 mutations, and type-level resolvers for all nested relationships.

### GraphQL Resolvers: `packages/shared/graphql/resolvers/edition.ts` (326 lines)

Type-level resolvers that bridge flat Restate data to nested GraphQL types:

| Resolver | What it bridges |
|----------|----------------|
| `Edition.county` | `countyId: UUID` → full `County` object via Restate call |
| `Edition.rows` | Passes through pre-loaded rows, defaults to `[]` |
| `EditionRow.rowTemplate` | `rowTemplateId: UUID` → full `RowTemplate` via lookup |
| `EditionSlot.post` | Embedded post fields → `Post` object (no N+1) |
| `County.fipsCode` | Handles snake_case fallback |
| `RowTemplate.displayName` | Handles snake_case fallback |
| `PostTemplateConfig.*` | Handles snake_case fallbacks for 5 fields |
| `EditionConnection.totalCount` | Handles snake_case fallback |

### Admin-App: 2 pages + sidebar update

| File | Lines | What |
|------|-------|------|
| `editions/page.tsx` | 375 | List page: county dropdown, status filter tabs, create/batch-generate forms, edition table |
| `editions/[id]/page.tsx` | 336 | Detail page: header with actions, summary stats, broadsheet row/slot rendering, remove post |
| `AdminSidebar.tsx` | +12 | Added "Editions" nav item in Content group |
| `lib/graphql/editions.ts` | 213 | 5 queries + 8 mutations for urql |

## Key Design Decisions

### 1. Posts exist once, editions reference them

A statewide story appears in all 87 county editions but exists as a single row in `posts`. The `edition_slots` table is a lightweight join — just `post_id` + `post_template` + `slot_index`. This keeps the database lean (~81k slot rows/year vs. duplicating post content 87x).

### 2. County relevance is computed, not stored

No `post_counties` join table. The layout engine computes county relevance at generation time by joining `posts` > `locationables` > `locations` > `zip_counties`. This avoids a denormalized mapping that would drift as posts are updated or locations change.

### 3. Row templates are config tables, not code

The 8 row templates and 7 post templates are database rows in `row_template_configs` / `post_template_configs`, not Rust enums or hardcoded structs. New templates can be added with an INSERT, no code change needed. The slot definitions (`row_template_slots`) define weight constraints and type filters per slot group.

### 4. Embedded post data avoids N+1

The `EditionSlot` GraphQL type needs post title, type, weight, and status. Rather than making a separate Restate call per slot (N+1 for 25+ slots per edition), the Rust model `find_by_row_with_posts` JOINs `edition_slots` with `posts` in a single query. The slot result carries embedded post fields that the GraphQL resolver maps directly.

### 5. The layout engine is "deliberately dumb"

Per CMS_SYSTEM_SPEC.md, the algorithm is a greedy heuristic, not an optimization solver. It picks row templates that consume the most remaining posts, fills slots with highest-priority posts, and sorts rows by priority. An editor can then manually adjust via the admin UI. This keeps the algorithm simple and predictable — editors understand why posts land where they do.

## Surprises and Gotchas

### 1. `posts.zip_code` doesn't exist — locations are normalized

The plan assumed posts had a `zip_code` column. In reality, location data flows through a normalized path: `posts` > `post_locations` > `locations` > `locations.postal_code`. The layout engine's county-relevance query had to join through this chain:

```sql
-- Plan assumed:
LEFT JOIN zip_counties zc ON p.zip_code = zc.zip_code

-- Reality requires:
LEFT JOIN post_locations pl ON pl.post_id = p.id AND pl.is_primary = true
LEFT JOIN locations loc ON loc.id = pl.location_id
LEFT JOIN zip_counties zc ON loc.postal_code = zc.zip_code
```

**Fix**: Rewrote the `load_county_posts` SQL to join through the actual location chain. Changed the statewide fallback from `p.zip_code IS NULL` to `pl.id IS NULL` (no primary location = treated as statewide).

**Lesson**: Plans written from spec documents may assume simplified schemas. Always verify column existence with `\d tablename` before writing SQL in a new domain.

### 2. Flat Restate data vs. nested GraphQL types

The Restate service returns flat objects (`{ countyId: "uuid", periodStart: "2026-02-24", ... }`) but GraphQL expects nested types (`{ county: { id, name, fipsCode }, periodStart, ... }`). The codebase has a `snakeToCamel` utility that handles field name conversion automatically, but it can't resolve foreign key IDs into nested objects.

**Fix**: Added type-level resolvers to `edition.ts` — `Edition.county` calls `get_county` to resolve `countyId`, `EditionRow.rowTemplate` looks up the template by `rowTemplateId`, etc. This is the standard pattern for bridging Restate's flat response model to GraphQL's type graph.

**Lesson**: Any GraphQL type with nested object fields (not just scalar renames) needs a type-level resolver when the data source returns flat references. This will recur in every new Restate service.

### 3. No `get_post` Restate handler existed

The initial `EditionSlot.post` resolver tried `ctx.restate.callService("Posts", "get_post", ...)` but the Posts service has no single-post fetch handler. This would have caused N+1 calls even if it existed (25+ posts per edition).

**Fix**: Embedded post data in the Rust query. Added `SlotWithPost` struct that JOINs `edition_slots` with `posts`, and carries `post_title`, `post_post_type`, `post_weight`, `post_status` fields. The GraphQL resolver maps these directly — zero additional API calls.

**Lesson**: Before writing a resolver that calls another service, verify the handler exists. Better yet, prefer SQL JOINs over cross-service calls when the data lives in the same database.

### 4. `posts.organization_id` doesn't exist either

The `SlotWithPost` query initially included `LEFT JOIN organizations o ON o.id = p.organization_id` to include organization names in slot data. But `posts` has no `organization_id` column — the relationship flows through `organization_posts` or is inferred from source data.

**Fix**: Removed the organization join entirely. Organization names in slots are a nice-to-have that can be added in Phase 4 via the proper join path.

**Lesson**: Same pattern as gotcha #1 — assumed a direct FK that doesn't exist. The codebase consistently uses join tables rather than direct FKs for cross-domain relationships.

### 5. Post-review: `post_locations` table was superseded by `locationables`

Caught in code review after the initial implementation. Migration 000161 moved post location data to the polymorphic `locationables` table. The layout engine's fix for gotcha #1 joined `post_locations`, which only contains pre-migration data. New posts write to `locationables` exclusively, so the layout engine would miss all posts created after the migration.

**Fix**: Changed the join to `locationables la ON la.locatable_id = p.id AND la.locatable_type = 'post' AND la.is_primary = true`.

**Lesson**: When fixing a table reference, verify the table is still the canonical source. Polymorphic migration patterns (like `post_locations` → `locationables`) leave the old table in place "for rollback safety," making it easy to join the stale version.

### 6. Post-review: `taggable_type = 'Post'` should be lowercase `'post'`

Caught in code review. Every other query in the codebase uses lowercase `'post'` for `taggable_type`, consistent with migration 000117 which standardized the value. The layout engine's statewide tag check used `'Post'` (capital P), silently matching zero rows.

**Fix**: Changed to `taggable_type = 'post'`.

**Lesson**: Polymorphic type discriminators are case-sensitive string comparisons with no compile-time checking. A `grep` for the value across the codebase would have caught this immediately.

### 7. Post-review: Multi-count slots all got the same `slot_index`

Caught in code review. A `three-column` template with one slot definition (`slot_index=0, count=3`) placed all 3 posts with `slot_index=0`. The front-end couldn't distinguish positions within the row.

**Fix**: Changed to a row-level `next_slot_index` counter that increments per placed post, giving each post a unique position.

### 8. Post-review: N+1 RPC calls in `EditionRow.rowTemplate` resolver

Caught in code review. Each row in the GraphQL response fired a separate `row_templates` RPC call to the Restate service (10 rows = 10 identical full-list calls), then filtered client-side.

**Fix**: Embedded full template data (`display_name`, `description`, slot definitions) in `EditionRowResult` from the Rust service. The Rust `load_edition_detail` now loads all templates + slots in 2 upfront queries instead of N per-row queries. The GraphQL resolver constructs the `RowTemplate` object from parent data with zero RPC calls.

### 9. Post-review: Several smaller issues

Also caught in review and fixed:
- `Edition.county` resolver could return `null` for non-nullable `County!` field
- `.unwrap()` on second county lookup in `create_edition` (used first lookup result instead)
- `generate_edition` allowed regenerating archived editions (guard now blocks non-draft)
- `change_slot_template` returned null post data (now re-fetches with JOIN)
- Dead `AddPostToEditionRequest` type with no handler (removed)
- `generateEdition` error swallowed in create-then-generate UI flow (now shows error)
- 3 dead sidebar nav links to deleted pages (Proposals, Sources, Search Queries)
- `edition_slots.post_template` had no FK constraint (added in migration 000175)

## What Went Well

1. **Clean domain isolation**: The entire `domains/editions/` directory (18 files, 2,052 lines) has zero dependencies on other domains. It only reaches into `kernel/ServerDeps` for the DB pool. This will make Phase 4 additions (drag-drop editor, visual rendering) straightforward — they extend the edition domain without touching posts or organizations.

2. **Config-table pattern reused from Phase 2**: The `post_type_configs` table from Phase 2 established the config-table pattern. Phase 3 followed it exactly for `row_template_configs`, `row_template_slots`, and `post_template_configs`. Consistency made the Rust models near-copy-paste.

3. **Batch generation worked first try**: The `batch_generate` handler creates and generates editions for all 87 counties in sequence. Despite being the most complex operation (87 x layout engine runs), it worked correctly on the first attempt — 87 editions created, each with county-relevant posts properly placed.

4. **Admin UI pages were fast to build**: Following the established patterns from the organizations and posts list/detail pages, the two edition admin pages took minimal iteration. The `urql` hooks + `additionalTypenames` cache invalidation pattern was already proven.

5. **Live browser testing caught all 4 bugs**: Every bug was discovered by actually using the admin UI in Chrome, not by reading code. This validates the approach of building minimal admin pages in Phase 3 rather than waiting for Phase 4 — they serve as integration smoke tests.

## What Could Be Better

1. **The plan's SQL was written from the spec, not the schema**. All 4 bugs were column-name mismatches (`p.zip_code`, `p.organization_id`) or missing join paths. Future phases should include a "schema verification" step: run `\d posts`, `\d locations`, etc. and record actual column names before writing any SQL.

2. **Type-level resolver pattern should be documented**. The `snakeToCamel` utility handles 80% of Restate-to-GraphQL bridging, but nested objects need explicit resolvers. A brief doc or code comment in the resolver utility explaining when type-level resolvers are needed would save future debugging.

3. **The `EditionRow.rowTemplate` resolver had an N+1** (fixed in review). Originally fetched all templates via RPC per row. Now resolved by embedding template data in the Rust service response. The pattern "embed related data in the response" should be the default for all Restate→GraphQL bridges, not an afterthought.

4. **No tests**. The layout engine is a pure function (`place_posts`) that would be trivially testable with mock data. Adding unit tests for the placement algorithm would catch regressions when templates change. Deferred to Phase 4 but should be prioritized.

5. **Migration is monolithic**. 308 lines in a single file including 87 county INSERTs and ~900 zip mapping INSERTs. Could have been split: `000174_phase3_tables.sql` + `000175_seed_counties.sql` + `000176_seed_templates.sql` for finer rollback granularity.

## Verification

| Check | Result |
|-------|--------|
| `cargo check` | 0 errors, 0 warnings |
| `tsc --noEmit` (admin-app) | 0 errors |
| `graphql-codegen` | Clean generation |
| Migration 000174 | Applied successfully (421ms) |
| `counties` query | Returns 87 rows |
| `createEdition` + `generateEdition` | Creates draft, populates rows + slots from active posts |
| Edition detail page | Washington County: 7 rows, 25 posts placed, 2 unique templates |
| Publish / Archive | Status transitions work correctly |
| Batch generate | Creates 87 county editions successfully |
| Remove post from edition | Slot deleted, edition refetched |

## Remaining Work (Not in Scope for Phase 3)

- **Drag-and-drop broadsheet editor** (Phase 4)
- **Visual broadsheet CSS grid rendering** (Phase 5)
- **Unit tests for layout engine** (Phase 4)
- **DataLoader for rowTemplate resolver** (Phase 4/5)
- **Organization name in slot data** (Phase 4, via proper join path)
- **Period-based post filtering** (layout engine currently uses all active posts; needs `published_at` windowing)
- **Edition preview / public viewing** (Phase 5)
- **Email newsletter generation from editions** (Phase 4)

## Stats Summary

| Metric | Value |
|--------|-------|
| Files created | ~25 (18 Rust + 2 admin pages + 1 GraphQL client + 1 resolver + 1 migration + codegen output) |
| Files modified | ~5 (sidebar, server.rs, domains/mod.rs, schema.ts, resolver index) |
| Lines added | ~4,024 (2,052 Rust + 308 SQL + 326 resolver + 213 GraphQL client + 711 admin pages + misc) |
| Lines deleted | ~0 (no existing code replaced) |
| New database tables | 8 |
| Seed data rows | ~1,000 (87 counties + ~900 zip mappings + 8 row templates + ~20 slot defs + 7 post templates) |
| New Rust models | 8 |
| New Restate handlers | 16 |
| GraphQL types added | 9 (5 primary + 4 supporting) |
| GraphQL queries added | 3 |
| GraphQL mutations added | 9 |
| Bugs found via live testing | 4 (all fixed same session) |
| Bugs found via post-review | 9 (fixed in follow-up session) |
| Time | ~8 hours across 3 sessions |
