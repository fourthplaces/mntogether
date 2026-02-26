# Phase 2 Postmortem: Expand Post Types

**Date**: 2026-02-25
**Commit**: Uncommitted (pending review)
**Stats**: 40 files changed, 817 lines added, 5,294 lines deleted

---

## What This Was

Phase 2 of the Root Editorial Pivot — replace the old 4-type system (service/opportunity/business/professional) with a new 6-type system (story/notice/exchange/event/spotlight/reference) defined in CMS_SYSTEM_SPEC.md. The key design idea: **types are form presets, not rigid schemas**. Every post has access to every field group; the type just decides which fields are open by default.

This phase touched the data model (migration + Rust models), the Restate service layer, the GraphQL schema, and the admin-app. It also ended up including significant Phase 1 leftover cleanup in the admin-app that was discovered when GraphQL codegen failed.

## Scope: Plan vs. Reality

**Planned** (from the Phase 2 plan):
- Migration with new columns, 7 new tables, data migration, tag seeding
- Rust model updates (PostType enum, Post struct, 6 new field group models)
- GraphQL schema updates (enums, field group types, post stats)
- Admin-app type filter + stats updates

**Actual** (superset):
- All of the above, plus...
- 5,233 lines of dead admin-app code cleaned up (Phase 1 leftovers that were blocking codegen)
- 5 dead GraphQL query files deleted, 6 more trimmed
- 4 dead page directories deleted (websites, sources, search-queries, proposals)
- 1 dead component deleted (Chatroom)
- 6 live pages surgically cleaned of dead imports

## Inventory of Changes

### Migration: `000173_phase2_post_types.sql` (202 lines)

| Section | What it does |
|---------|-------------|
| New columns | `weight TEXT DEFAULT 'medium'`, `priority INT DEFAULT 0` on posts |
| `post_type_configs` | Config table with 6 rows: story, notice, exchange, event, spotlight, reference |
| `post_items` | 1:many name+detail pairs (Exchange directories, Reference listings) |
| `post_media` | 1:many image with caption and credit |
| `post_person` | 1:1 profile fields for Spotlight (name, role, bio, photo, quote) |
| `post_link` | 1:1 CTA button with optional deadline |
| `post_source_attribution` | 1:1 source name and attribution line |
| `post_meta` | 1:1 editorial metadata (kicker, byline, timestamp, updated) |
| Reserved tags | 8 system tags: urgent, recurring, closed, need, aid, action, person, business |
| Data migration | service/opportunity → exchange, business/professional → spotlight |
| Tag migration | Scraped exchange posts get `aid` tag; spotlight posts from business get `business` tag |
| Cleanup | Delete old `post_type` tag kind and its tags/taggables |
| Constraints | CHECK constraints on `post_type` (6 values) and `weight` (3 values) |

**Not run yet** — migration is staged but requires manual execution.

### New Rust Models (6 files, 409 lines)

| Model | File | Pattern | Key method |
|-------|------|---------|------------|
| `PostItem` | `post_item.rs` (91 lines) | 1:many | `replace_all` — delete + reinsert for bulk |
| `PostMedia` | `post_media.rs` (66 lines) | 1:many | `replace_all` |
| `PostPerson` | `post_person.rs` (67 lines) | 1:1 | `upsert` via ON CONFLICT DO UPDATE |
| `PostLink` | `post_link.rs` (62 lines) | 1:1 | `upsert` |
| `PostSourceAttribution` | `post_source_attribution.rs` (58 lines) | 1:1 | `upsert` |
| `PostMeta` | `post_meta.rs` (65 lines) | 1:1 | `upsert` |

### Updated Rust Models

| File | Changes |
|------|---------|
| `post.rs` | `PostType` enum: 4 → 6 values. New `Weight` enum. `CreatePost`/`UpdatePostContent` builders gain weight + priority. 5 SQL queries rewritten (tag join → column filter). |
| `mod.rs` | 6 new module declarations + re-exports |
| `upcoming_events.rs` | `"business"` → `"spotlight"` in batch-load filter |
| `posts.rs` (Restate) | `PostStatsResult`: 3 type counters → 6. Stats handler match arms updated. |

### GraphQL Schema

| Change | Detail |
|--------|--------|
| `PostType` enum | `story \| notice \| exchange \| event \| spotlight \| reference` |
| `Weight` enum | `heavy \| medium \| light` (new) |
| `Post` type | Added `weight: Weight`, `priority: Int` |
| `PostStats` type | 6 type fields replacing 3 |

### Admin-App Changes (Phase 2 proper)

| File | What changed |
|------|-------------|
| `lib/graphql/posts.ts` | `PostStatsQuery` fields updated to 6 types |
| `lib/types.ts` | `PostType` union: 6 values. `PostStatsResult`: 6 counters. |
| `posts/page.tsx` | Type filter dropdown (7 values incl "all"), data-driven stats grid, updated review tips |
| `codegen.ts` | Added `allowPartialOutputs: true` to unblock codegen |

### Admin-App Cleanup (Phase 1 leftovers, unblocked codegen)

| Deleted | Lines | What it was |
|---------|-------|-------------|
| `lib/graphql/chat.ts` | 47 | Dead chatroom queries |
| `lib/graphql/websites.ts` | 242 | Dead website queries |
| `lib/graphql/sources.ts` | 280 | Dead source queries |
| `lib/graphql/sync.ts` | 80 | Dead sync/proposal queries |
| `lib/graphql/search-queries.ts` | 57 | Dead search query queries |
| `proposals/page.tsx` | 762 | Dead proposals review page |
| `sources/[id]/page.tsx` | 801 | Dead source detail page |
| `sources/[id]/snapshots/page.tsx` | 87 | Dead source snapshots page |
| `sources/page.tsx` | 363 | Dead sources list page |
| `websites/[id]/page.tsx` | 763 | Dead website detail page |
| `websites/[id]/snapshots/page.tsx` | 87 | Dead website snapshots page |
| `websites/page.tsx` | 262 | Dead websites list page |
| `components/admin/Chatroom.tsx` | 286 | Dead chatroom component |
| `dashboard/page.tsx` (edited) | ~42 | Removed website references |
| `organizations/[id]/page.tsx` (edited) | ~500 | Removed sources UI, dead mutations |
| `organizations/page.tsx` (edited) | ~12 | Removed dead column references |
| `posts/[id]/page.tsx` (edited) | ~84 | Removed sync imports, proposals UI |
| `layout.tsx` (edited) | ~14 | Removed chatroom sidebar |

## Key Design Decisions

### 1. Types are config, not architecture

The `post_type_configs` table stores each type's default weight, default field groups, and compatible templates. When a user creates a "Notice," the CMS reads this config to pre-open the meta and source field groups. But nothing prevents adding media to a Notice — the groups are just hidden by default.

This avoids the common CMS trap of rigidly coupling content structure to content type.

### 2. Separate tables for field groups, not JSONB

Per CLAUDE.md ("Normalize into relational tables. JSONB only for truly unstructured data."), each field group gets its own table. This gives us type safety via `FromRow` derives, per-field indexing, and clean Rust structs.

The 1:1 groups use `UNIQUE` on `post_id` + `ON CONFLICT DO UPDATE` for idempotent upserts. The 1:many groups use `replace_all` (delete + reinsert in a transaction).

### 3. Column filtering replaces tag-based filtering

The old system stored post types as tags (`kind = 'post_type'`), requiring a double-JOIN through `taggables → tags` on every query. Five query methods were rewritten to filter on `p.post_type` directly — fewer JOINs, better index usage, simpler SQL.

The old `post_type` tag kind and all its tags/taggables are deleted in the migration cleanup step.

### 4. No new `body` column

The spec defines a `body` field, but `description_markdown` already serves this role. Rather than adding a third content column, we keep the existing pair (`description` for plaintext, `description_markdown` for rich text) and document the semantic shift. `summary` stays as the short version.

### 5. Priority bootstrapped from relevance_score

Existing posts get `priority = COALESCE(relevance_score, 50)`. This preserves the editorial signal from the old scoring system while giving human editors a field they can manually adjust.

## Surprises and Gotchas

### 1. Admin-app codegen was blocked by Phase 1 dead code

When we ran `graphql-codegen` after updating the schema, it produced 84 TypeScript errors. The errors weren't from Phase 2 changes — they were from dead Phase 1 GraphQL queries that referenced types no longer in the schema (websites, sources, sync, proposals).

Phase 1 had cleaned the Rust server and GraphQL schema, but the admin-app's GraphQL query files were left behind because the app still compiled (the queries were unused). Codegen doesn't care about usage — it validates all queries against the schema.

**Fix**: Three-pronged approach:
1. Added `allowPartialOutputs: true` to `codegen.ts` as an escape hatch
2. Deleted 5 entirely dead query files, edited 6 more to remove dead portions
3. Deleted 4 dead page directories and cleaned 6 live pages

**Lesson**: Dead GraphQL queries are time bombs. They compile fine in TypeScript (unused imports are allowed), but break codegen the moment the schema changes. Phase 1 should have cleaned the admin-app GraphQL layer, not just the server.

### 2. Codegen with clean `gql/` directory + `allowPartialOutputs` = empty output

When codegen ran with `allowPartialOutputs: true` but the `gql/` output directory was empty, it generated only `index.ts` — an empty barrel file. The partial outputs feature only skips broken queries; with an empty output dir and many broken queries, it skipped everything.

**Fix**: Had to fully clean up the dead queries first, then run codegen from scratch. `allowPartialOutputs` remains as a safety net for future schema changes but isn't a substitute for cleanup.

### 3. The scope creep was worth it

What started as "update PostType from 4 to 6 values" expanded into a 5,200-line admin-app cleanup because codegen forced the issue. This was genuinely necessary — the dead code would have blocked the next person who touched the schema.

The cleanup also revealed the admin-app had drifted further from reality than anyone knew: entire page directories for features that no longer existed on the backend.

### 4. Stale old-type references in UI components (caught in review)

A post-implementation audit found 3 files with hardcoded old type values that would have silently broken after the migration:

| File | Issue | Impact |
|------|-------|--------|
| `PostReviewCard.tsx` | `TYPE_VARIANTS` keyed on `service`, `opportunity`, `business` | Badge colors would fall through to default for all posts |
| `Badge.tsx` | Variant styles `service`, `opportunity`, `business` | Dead CSS variants taking up space |
| `organizations/[id]/page.tsx` | `post.postType === "service"` comparisons | Type badge colors would all be grey |

**Fix**: Updated all type references to the new 6-type system. Also cleaned up stale field definitions in `lib/types.ts` (removed old service/opportunity-specific fields, added `weight` and `priority`).

**Lesson**: When migrating enum values, grep for the OLD values as string literals, not just the enum definition. `cargo check` and `tsc` won't catch stale string comparisons — they're type-correct but semantically dead.

### 5. `apply_revision` didn't forward new fields (caught in review)

The `Post::apply_revision` method copies revision content to the original post but wasn't passing `post_type`, `weight`, or `priority` to the `UpdatePostContent` builder. Since these fields default to `None`, the `COALESCE` in SQL would silently preserve the original values.

**Fix**: Added `.post_type()`, `.weight()`, and `.priority()` to the builder call in `apply_revision`.

**Lesson**: When adding fields to a builder, audit all call sites — not just the obvious ones like `create`. Builder defaults (`None`) can silently swallow updates.

## Verification

| Check | Result |
|-------|--------|
| `cargo check` | 0 errors, 0 warnings |
| `tsc --noEmit` (admin-app) | 0 errors |
| `graphql-codegen` | Clean generation, 0 partial skips |
| Migration syntax review | Valid PostgreSQL, all `IF NOT EXISTS` / `ON CONFLICT` idempotent |
| Old type grep (`service\|opportunity\|business\|professional`) | 0 matches in `.rs` and `.tsx` files |
| Model ↔ migration schema audit | All 6 field group models match their table definitions |

## What Went Well

1. **Reusing existing infrastructure**: The posts table already had `post_type`, `urgency`, `status`, `location`, and `source_url` columns. `post_contacts`, `locations`, `schedules`, and `tags` tables were all already in place from earlier work. Phase 2 only added what was genuinely new.

2. **The 1:1 vs 1:many pattern was clean**: The upsert-or-replace-all split maps perfectly to the data semantics. A post has one person profile (upsert), but many items (replace all). The Rust models are ~60-90 lines each with a consistent API surface.

3. **Tag-to-column migration removed query complexity**: The 5 rewritten queries are measurably simpler — eliminating a JOIN through `taggables` + `tags` on every filtered post list query.

4. **Admin-app cleanup cascaded correctly**: Once the dead query files were deleted, TypeScript errors pointed directly to every dead import in live pages. No guessing required.

5. **Post-implementation review caught real bugs**: The stale string comparisons in 3 UI files and the `apply_revision` gap would have been silent failures in production. Systematic grepping for old enum values caught what the type checker couldn't.

## What Could Be Better

1. **Phase 1 should have cleaned the admin-app GraphQL layer**. The dead queries wouldn't have been discovered until codegen ran because TypeScript doesn't error on unused imports. A pre-check script (`graphql-codegen --check` or equivalent) should be added to CI.

2. **The migration is large** (202 lines, 7 sections). It could be split into multiple numbered migrations for finer-grained rollback, but the `IF NOT EXISTS` / `ON CONFLICT` guards make it safely re-runnable.

3. **No GraphQL resolvers for field groups yet**. The Rust models exist but aren't wired into GraphQL queries. The admin-app can't yet fetch or mutate field group data. This is intentional (UI comes later), but the gap should be closed in Phase 3 or 4.

4. **String enums are invisible to the type checker**. Both Rust (`post_type: String`) and TypeScript use strings for post types, so stale value comparisons compile fine. Consider a future refactor to use typed enums in SQL queries (e.g., SQLx type-checked queries or TypeScript `as const` narrowing) so the compiler can catch value drift.

## Remaining Work (Not in Scope for Phase 2)

- **Run migration 000173** on the database
- Wire field group models into GraphQL resolvers (Phase 3-4)
- Build per-type CMS editor forms with field group UI (Phase 4)
- Broadsheet/edition system: rows, slots, templates (Phase 3)
- Post template character limits / truncation (Phase 5)
- Remove `allowPartialOutputs` from codegen.ts after confirming no more dead queries exist

## Stats Summary

| Metric | Value |
|--------|-------|
| Files changed (tracked) | 34 |
| Files created (new) | 7 (migration + 6 field group models) |
| Lines added | 817 |
| Lines deleted | 5,294 |
| Net change | -4,477 lines |
| New database tables | 7 (1 config + 6 field groups) |
| New Rust models | 6 |
| SQL queries rewritten | 5 (tag join → column filter) |
| Admin-app dead pages removed | 7 |
| Admin-app dead query files removed | 5 |
| Time | ~4 hours across 2 sessions |
