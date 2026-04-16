# Broadsheet Generation — Postmortem & Tech Debt Assessment

**Date:** April 2026
**Scope:** Edition generation pipeline, layout engine, seed data, widget system, and readiness for Root Signal integration.

---

## What We Built

The broadsheet generation pipeline takes a county + date range, finds eligible posts, and assembles them into a newspaper-style layout:

```
County + Period → load_county_posts → select_row_templates → fill_rows → place_widgets → persist
```

The layout engine (`layout_engine.rs`, ~1,480 lines) handles template selection, height balancing, type compatibility, density progression, and visual diversity. It runs in-process on the Rust server, triggered by an admin "Regenerate" action.

### Key metrics (current state)

| Metric | Value |
|---|---|
| Post templates (visual variants) | 16, all active |
| Row templates (layout recipes) | 41 |
| Seed posts | 150 |
| Pilot counties with local content | 6 (Hennepin, Ramsey, Dakota, Aitkin, Anoka, Becker) + Beltrami |
| Counties with only statewide content | 80 |
| Widget types | 6 (section_sep, pull_quote, resource_bar, number, photo, weather placeholder) |
| County-specific widgets | 21 across 6 pilot counties |
| Evergreen widgets | 16 |
| Editions generated (dev) | 174, 1 published |

---

## What Went Well

1. **Weight-budget system works.** The three-phase (hero/mid/dense) selection with per-county `target_content_weight` scales naturally. Small counties get short broadsheets; large counties get full ones. No hard-coded row caps.

2. **Type-compatibility pre-check prevents phantom deductions.** `can_fill_template()` simulates slot filling before committing, so the engine doesn't starve Phase 3 by picking unfillable templates in Phase 2. This was the single biggest fill-rate fix.

3. **Service-area scoping works correctly.** County-specific posts (Scott County resources) no longer leak into Aitkin's broadsheet. The "no location = statewide" fallback correctly excludes posts with explicit `service_area` tags.

4. **All 16 post templates now have viable paths to render.** Every template has at least one row template that references it and enough seed data to fill it.

5. **Widget tiering (statewide vs county-specific) is clean.** Resource bars and stats are county-scoped. Section separators, pull quotes, and photos are evergreen. No cross-county leakage.

---

## What Went Wrong (and what we fixed)

### Layout engine bugs (all fixed)

| Bug | Impact | Root cause |
|---|---|---|
| Templates picked but unfillable → phases starved | Aitkin slotted 3 of 11 | `pick_best_template` checked weight counts but not type compatibility |
| Trio/pair cells not height-balanced | Reference posts towered over siblings | `is_stacked` only matched lead-stack/pair-stack, not trio/pair |
| Pair layout dropped 2-slot rows | 4 posts silently missing from preview | `postsPerCell` hardcoded to `[2,2]` in row-map.ts |
| Nested `<a>` in post cards | Hydration errors in Next.js | Whole-card `<a>` wrapper conflicted with inner CTA links |

### Seed data bugs (all fixed)

| Bug | Impact | Root cause |
|---|---|---|
| posts.json used old 6-type enum | Re-seeding failed with constraint violations | Migration 216 updated DB in-place but not the seed file |
| Sub-metro service_area tags orphaned 21 posts | Posts invisible in all editions | `lake-street`, `phillips` etc. didn't match any county slug |
| Seeder only cleaned `ingested` posts | Duplicates accumulated on re-seed (3× multiplication) | `admin` submission_type posts weren't cleaned |
| Evergreen posts had old `publishedOffsetDays` | Reference posts fell outside 7-day eligibility window | `-14` and `-30` offsets pre-dated the edition period |
| No service_area tags for pilot counties | 40 posts had no tags → wrong statewide fallback | `tags.json` only had 5 metro county slugs |

### Row template rules (learned the hard way)

**Rule:** A `pair` layout with `count=1` per cell must use **same-weight slots**. Cross-weight pairs (medium + light) render with massive empty space in the lighter cell because height balancing only runs WITHIN a cell (for stacking), not ACROSS cells.

For medium + light combos, use `pair-stack` variant with `count ≥ 3` on the light side — the stacked lights reach the medium's height. See `pair-stack-gazette` for the canonical shape.

Violations we shipped and fixed:
- `pair-bulletin-ledger` (medium bulletin height=7 vs light ledger height=3 = 2.3× imbalance) — converted to pair-stack with 3 lights
- `pair-bulletin-digest` (bulletin=7 vs digest=2 = 3.5× imbalance) — same fix
- `pair-gazette-bulletin`, `pair-gazette-spotlight`, `pair-spotlight-bulletin` — mixed-medium pairs where one side renders taller (gazette+story is 2-3× a bulletin+need). Converted to pair-stack with count=2 on the shorter side. Note: this means "medium" as a weight category doesn't guarantee equal rendered heights — the post_type + post_template combination determines actual height.

### Design mistakes

| Decision | Why it was wrong | What we did |
|---|---|---|
| Global widgets (resource_bar, number) | Phone numbers and local services are county-specific. Global resources give dangerous advice. | Restructured into statewide (narrow: 211, 988) + per-county tiers |
| `pencil-circle` on MTitle | Circle is for dates, not titles. Looked broken on event cards. | MTitle now ignores `circle` pencilMark |
| Hard-coded `max_rows = 14` | Arbitrary cap didn't account for pool size or county content weight | Replaced with per-county `target_content_weight` driving a weight budget |

---

## Known Tech Debt

### High priority (fix before Root Signal integration)

1. **`county_service_area_slug()` is duplicated.** Identical implementations exist in `layout_engine.rs` and `post.rs`. Both are private. Should be extracted to a shared utility in `common/` or `domains/editions/`. If the slug format changes, one copy will drift.

2. **`effective_height()` is a hardcoded lookup table.** The reference-post height bonus (`+3` for ledger/bulletin) is code-level knowledge that should live in the database — either a `height_units_by_type` JSONB column on `post_template_configs` or a separate mapping table. Currently only handles `(ledger, reference)` and `(bulletin, reference)`; other outlier combos will need manual additions.

3. **The seeder is destructive.** `DELETE FROM posts WHERE deleted_at IS NULL` wipes every post on every re-seed. This is fine for dev but will be catastrophic if accidentally run against a database with real content. Needs either:
   - A `seed_batch_id` column to scope deletions
   - A separate `dev_seed` table that maps to posts
   - At minimum, a `--yes-i-am-sure` flag

4. **`publishedOffsetDays` is fragile.** Seed posts use relative offsets from NOW(), meaning re-seeding shifts all publish dates. Evergreen content (references, businesses) needs a different mechanism — either `publishedAt: null` (always eligible) or an `evergreen: true` flag that bypasses the 7-day filter.

5. **No post-type coverage for `need` at light weight.** Only 0 light `need` posts exist. `need` posts are medium-only in the seed, which means light-only row templates can never show community needs. This matters for ticker-style "quick asks."

### Medium priority

6. **41 row templates with no usage analytics.** We can't tell which templates are effective vs. dead weight. Should add a `times_used` counter or a query that joins `edition_rows` to `row_template_configs` for a usage report.

7. **Widget placement rules are positional, not semantic.** `place_widgets()` inserts widgets at fixed positions (after row 2, 4, 6, 8...). A semantic approach ("insert a section_sep when the topic changes" or "insert a resource_bar near related need/aid posts") would produce more editorially coherent broadsheets.

8. **`build_topic_sections()` output is unused by the public renderer.** Sections are built, persisted, and served via GraphQL — but the public broadsheet renderer ignores them entirely (flat sort_order rendering). The admin UI still shows them for editorial grouping. This is intentional but confusing for new developers.

9. **Height balancing uses integer estimates.** Real rendered heights depend on text length, line breaks, and responsive layout. Integer `height_units` are a rough proxy. A post-render pass that measures actual DOM heights and rebalances would produce tighter layouts — but that's a significant architecture change (server-side rendering or client-side reflow).

### Low priority

10. **`yarn.lock` has an uncommitted 3,800-line deletion.** From an unrelated dependency change. Should be committed or reverted separately.

11. **`weather` widget type exists in the schema but has no resolver.** Returns null. Not blocking but creates confusion.

12. **174 dev editions, 1 published.** Stale editions accumulate. No cleanup mechanism. Could add a `DELETE stale editions older than N days` maintenance task.

---

## Questions for Root Signal Integration

### Data contract

1. **What does Root Signal's output look like per county per week?** How many posts, what type/weight distribution? The layout engine is tuned for ~20-40 posts per county (weight target 66). If Signal produces 10 or 100, the engine flexes — but the broadsheet quality depends on the mix.

2. **Does Signal assign `service_area` tags?** The entire county-scoping system relies on `service_area` tags like `aitkin-county`. If Signal uses a different mechanism (e.g., a `county_id` FK on the post, or geographic coordinates), we need an adapter.

3. **Does Signal assign `post_type` using our 9-type enum?** The layout engine, component registry, and template compatibility arrays all key off `post_type`. If Signal uses different categories, we need a mapping layer.

4. **Does Signal assign weight (heavy/medium/light)?** The three-phase layout depends on a realistic weight distribution. If all Signal posts arrive as `medium`, the broadsheet will be a wall of mid-density gazette cards with no hero features or ticker density.

5. **Does Signal produce `bodyHeavy`/`bodyMedium`/`bodyLight` tiers?** The renderer selects body text based on the template's display size. Without tiered bodies, a heavy-weight feature story shows the same text as a light-weight ticker.

### Operational

6. **How should `target_content_weight` be set per county?** Currently a manual admin setting (default 66). Should Signal recommend a target based on source availability? Should it auto-adjust weekly?

7. **When does generation run?** Currently manual (admin clicks Regenerate). For Signal integration, this should be automated — probably a cron job or webhook trigger after Signal completes its weekly batch. The layout engine is stateless and idempotent, so re-running is safe.

8. **What about incremental updates?** If Signal produces 30 posts on Monday and 5 more on Wednesday, should the broadsheet regenerate? Currently the engine does a full rebuild each time. Incremental slot-swapping would be more efficient but much more complex.

### Content quality

9. **How do we handle Signal posts that don't fit any template?** Currently they're silently dropped (unslotted). Should there be an alert ("5 posts couldn't be placed — consider adding a row template for light `need` posts")?

10. **`pencilMark` and `is_urgent` are editor-only.** These are editorial judgment calls set manually by editors in the admin UI after generation, not by Root Signal. Signal should never set these fields — they represent human editorial emphasis.

11. **Evergreen vs. fresh content.** The 7-day `published_at` filter keeps broadsheets fresh but kills reference posts. We currently hack this with recent `publishedOffsetDays`. A real solution: either an `evergreen` flag that bypasses the date filter, or a separate content pool (widgets vs. posts) for standing resources.

---

## Recommendations

**Before wiring Signal:**
1. Extract `county_service_area_slug` to shared utility
2. Add `evergreen` flag to posts (bypasses date filter)
3. Add a "post placement report" log (which posts couldn't be placed and why)
4. Confirm Signal's output schema matches our 9-type + weight + service_area expectations

**First Signal integration:**
1. Signal writes posts to the `posts` table with proper `post_type`, `weight`, `service_area` tags, and tiered body text
2. A weekly cron triggers `generate_broadsheet` for each county with active Signal output
3. Editors review via admin UI, apply `pencilMark`/`is_urgent` where warranted, then publish

**Later:**
1. Semantic widget placement (topic-aware, not positional)
2. Post-render height rebalancing
3. Automated `target_content_weight` recommendations based on Signal output volume
