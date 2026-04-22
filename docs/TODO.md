# Root Editorial — Outstanding Work

> **Last updated:** 2026-04-20
>
> What's done, what's next, and what's punted. This is the single source of truth for prioritization.

---

## Completed Phases

| Phase | Summary | Postmortem |
|-------|---------|------------|
| **1. Dead Code Removal** | 45,947 lines removed. 11 domains, 4 packages deleted. | [Phase 1](status/PHASE_1_DEAD_CODE_REMOVAL.md) |
| **2. Post Types** | 6-type system (story/notice/exchange/event/spotlight/reference). 7 field group tables. | [Phase 2](status/PHASE_2_POST_TYPES.md) |
| **3. Edition System** | 87-county edition model, layout engine, batch generation, admin pages. | [Phase 3](status/PHASE_3_EDITION_SYSTEM.md) |
| **4. CMS UX + Broadsheet** | Editorial dashboard, kanban workflow, broadsheet rendering (43 post components, 9 widget components, 3,623 lines CSS), widget system, shadcn admin rebuild. Dead code cleanup. | [Phase 4](status/PHASE4_CMS_UX_REWORK.md), [Broadsheet](status/BROADSHEET_DESIGN_IMPORT.md) |

## Recently Completed (2026-04-20 session — Root Signal contract, Statewide, lifecycle polish)

| Item | Summary |
|------|---------|
| **Root Signal Data Contract** | Authoritative merged spec at `docs/architecture/ROOT_SIGNAL_DATA_CONTRACT.md` — supersedes `ROOT_SIGNAL_SPEC.md` + `ROOT_SIGNAL_INGEST_SPEC.md`. Framing settled: Signal *produces* posts (not enrichment), 250-char body_raw floor on every weight, per-post-type field-group requirements, hard/soft validation rules, individual-source model stub. |
| **Seed Data Overhaul (v2)** | All 168 posts enriched to contract: three body tiers, meta.byline + kicker, sourceAttribution, per-post-type field groups (contacts/items/datetime/link/scheduleEntries), organizationName → post_sources linkage. Seed.mjs gains `contacts[]` + `organizationName` handlers. Audit script `data/audit-seed.mjs` + `make audit-seed` regression check against `audit-seed.baseline.json`. 157/168 posts contract-perfect (remaining gaps are all media-related, deferred with media ingest). |
| **Statewide Pseudo-County (Phase B)** | Migration 236 adds `is_pseudo` column + Statewide row. Layout engine `load_county_posts` branches on pseudo → `load_statewide_posts` pulls only `service_area='statewide'` posts. `default_edition_title` drops " County" suffix for pseudo. Dashboard surfaces Statewide status in a dedicated callout with real generate/publish actions; bulk ops exclude pseudo. Web-app picker groups pseudo under "All of Minnesota" optgroup; home defaults to Statewide when URL is bare. |
| **Lifecycle Gate (Phase C)** | `require_populated_edition` guard blocks review/approve/publish on any edition with zero `post_id OR widget_id` slots. Prevents the "Aitkin approved-but-empty" class of orphan. Draft stays writable so regeneration still works. |
| **Layout Engine Cell Cohesion** | `fill_slot_group` pre-pass counts candidates per post_type in the pool and biases the sort so the most-represented type starts the cell. Lone high-priority minority-type posts no longer scatter into otherwise-pure cells. 4 new unit tests (first direct coverage on `fill_slot_group`). |
| **Pair-stack + Classifieds Density** | Migrations 233 / 234 bump slot counts so stacked cells visually match their anchors after line-clamp fixes. `pair-stack-alert` / `pair-stack-spotlight` go from digest×3 → digest×5; `classifieds*` trios from ×2 → ×3 per cell. |
| **Dashboard Redesign** | Current-week context header, 5-bucket status breakdown (Missing/Draft/Review/Approved/Published), adaptive primary CTA (Generate → Publish all → Review → All set), "How the editorial flow works" explainer, Statewide callout with real generate+publish actions. Saturated icon tints match their card tones (amber-700 on bg-amber-50, etc.). |
| **County Picker (public site)** | Simple `<select>` in the public home nav, grouped by "All of Minnesota" / "By county". URL-param-driven (`?county=<id>`). `list_counties` route made public (OptionalUser). |
| **Anchor Clamp Fix** | `prepare.ts` flipped the anchor sentinel from `0` to `undefined` so anchor cards pick up the template's computeClamp value. Fixes overflow on alert-urgent / gaz-story / gaz-request (22 card templates total). |
| **Whole-Tile Click Restored** | `<ClickableTile>` wrapper in `BroadsheetRenderer`. Preserves text selection, nested-link hover states, and cmd-click-to-new-tab. Superseded the old `.post-link__overlay` pattern. Ticker + FeatureHero titles now clickable via `<MInlineTitle>` / `<MTitle>`. |
| **Date Badge Relational Labels** | Pencil-circle annotation uses duration language (Today / Tomorrow / This weekend / This week / Next weekend / Next week / Two weeks / This month / Next month), drops labels entirely for past events or anything >60 days out. Never restates the circled weekday. |
| **UX Polish** | Items list moved from sidebar to main column on post detail pages (with type-aware heading for need/aid/reference, hidden when no meaningful label fits). View Site link fixed (plain `<a target="_blank">` avoids Turbopack Performance.measure bug). Alert width: `px-6` on parent column, drop `mx-6` on alerts. All user-facing unicode glyphs (✓ ⚠ ● ○ —) swapped for Lucide icons. Sidebar Tooltip defers mount until post-hydration to sidestep Base UI useId mismatches. |

## Recently Completed (prior session)

| Item | Summary |
|------|---------|
| **Field Group Hydration** | 9 Rust models with batch queries, field groups flow through broadsheet pipeline (Rust → GraphQL → frontend). Migration 207 adds post_datetime, post_status, post_schedule tables + deck column on post_meta. |
| **Render Hints** | `computeRenderHints()` in `web-app/lib/broadsheet/render-hints.ts` — computes month/day/dow/when/circleLabel (events), count (items), tagline (person.role), pullQuote, date from field group data. |
| **Widget Template System** | Merged stat_card + number_block → single `number` type. Added `widget_template` column to edition_slots. SectionSep supports default/ledger variants. Deleted dead LedgerSectionBreak.tsx. Migration 209. |
| **Seed Data Overhaul** | Migration 208 seeds field group records for all 35 posts: media, meta, source attribution, items, status, datetime, person, schedule. |
| **resolveWidget** | Centralized widget rendering in `web-app/lib/broadsheet/widget-resolver.ts` — parallel to `resolveTemplate` for posts. Owns JSON parsing, variant resolution, component selection. |
| **Story Editor** | Plate.js WYSIWYG replaces markdown textarea. ArticlePreview with web-app body styling. Field group panels on edit page (media, meta, person) and detail page (link, source, datetime, status, items). 7 upsert endpoints + GraphQL mutations. "Open Preview" button in EditorTopBar. |
| **Editor UX Fixes** | DnD block reordering via @platejs/dnd. TurnIntoMenu for block type conversion. Fixed Turbopack resolution, Slate SVG errors, void plugin input focus, content loading race condition, save handler. Lucide icons replace all Unicode/emoji hacks. |
| **Data Model Consolidation** | Renamed `description` → `body_raw`, dropped `description_markdown` and `summary`. Full-stack rename across migration, Rust server, GraphQL, admin-app, web-app. Seed data updated with body tier fields (`body_heavy/medium/light`). Search vector trigger updated. Migration 211. |
| **Post Detail Pages** | Full broadsheet detail layout: `(broadsheet)` route group (no site chrome), NewspaperFrame, ArticlePage 2/3+1/3 grid. Field groups load via OptionalUser (was AdminUser — blocked all public access). Components: EmailA, WebsiteA, PhoneA, AddressA, LinksA, ResourceListA. SVG icon sprite. Related posts endpoint + `Post::find_related()` (county → tags → type → recency). Admin preview bar. SiteFooter. Sidebar data backfill migration 212. |

---

## Active Work Queue

Priority order. Each item unblocks the ones below it.

### 1. Root Signal Ingestion Endpoint

Connect the CMS to Root Signal so AI-analyzed content flows into editions automatically. Contract is settled — only the ingestion code remains.

- **Contract (✅ done):** [ROOT_SIGNAL_DATA_CONTRACT.md](architecture/ROOT_SIGNAL_DATA_CONTRACT.md) — authoritative spec.
- **Scope remaining:**
  - Build `POST /Posts/create_post` per-post ingestion endpoint per §2 envelope
  - Validation pass per §9 (hard failures reject 422; soft failures land as `in_review`)
  - Dedup: resolve organization by `already_known_org_id` → website domain → exact name (contract §5.1)
  - Wire individual-source path (contract §5.2) — requires new `source_individuals` table (see Deferred Schema Work)
  - Optional cron / webhook trigger (cadence is an open question in the contract)
- **Data model ready:** columns + field groups exist; need the ingest glue.

### 2. Individual-Source Schema (blocks Signal ingestion for `source.kind='individual'`)

Contract §5.2 assumes a `source_individuals` table that doesn't exist yet.

- **Scope:** Migration for `source_individuals (id, display_name, handle, platform, platform_url, verified_identity, consent_to_publish, …)` + `post_sources.source_id` polymorphic extension to point at it. Consent-pending ingestion flow (posts land as `in_review` until editor confirms).

### 3. Signal Inbox

Triage UI for incoming Root Signal content.

- **Plan:** [SIGNAL_INBOX.md](architecture/SIGNAL_INBOX.md)
- **Depends on:** Story Editor (✅ done), Root Signal ingestion (#1) for real data
- **Scope:** Admin page with filtered post list, bulk approve/reject, edit-before-approve flow

### 4. Media Ingest Pipeline

Contract §8 + [ROOT_SIGNAL_MEDIA_INGEST.md](guides/ROOT_SIGNAL_MEDIA_INGEST.md) design. Not built. Blocks the remaining 11 seed-audit gaps (`media:no_hero_on_heavy` + `type_group:person_missing_media`).

- **Scope:** Server-side fetch of `source_image_url` → MinIO upload → create `media` row → link via `post_media.media_id`. Hash-based dedup. Size cap + magic-bytes validation. Optional format normalization (convert all → WebP?).
- **Open questions in spec:** dedup strategy, license propagation, retry-on-failure semantics.

### 5. Fresh Week Batch Generation Cron

Currently the only way to get a new week's editions generated is to click the dashboard CTA. For an unattended deploy we need a scheduled task that runs every Sunday night to prep the next Mon–Sun period's editions across every county (incl. Statewide).

- **Scope:** Scheduled task (Rust-side or container cron) invoking `batch_generate_editions`. Must be idempotent — re-running mid-week shouldn't clobber editor edits. The activity's existing status-check already gates this, but verify under real cron conditions.

### 6. Bulk Actions on the Editions List View

Dashboard has "Publish all N approved"; `/admin/editions` list doesn't. Nice-to-have second surface for editors who filter the list first.

- **Scope:** Multi-select on the editions table; bulk `publishEdition` loop (same shape as the dashboard handler); probably bulk `archiveEdition` too.

### 7. IP Geolocation for County Picker Default (Phase D — deferred)

Public home defaults to the Statewide pseudo-county when the URL is bare. Better: auto-select a county based on the visitor's IP for MN visitors; fall back to Statewide otherwise.

- **Scope:** Pick a backend (MaxMind GeoLite2 / ipinfo.io / Cloudflare `CF-IPCountry`+`CF-Region` headers if we deploy behind CF). Server-side resolver that maps IP → county row.
- **Blocked on:** Infrastructure / vendor decision.

### 8. Post Detail Page Polish

Core detail layout done (NewspaperFrame, ArticlePage, field group components, related posts, SiteFooter). Remaining:

- **Full component audit** — sidebar fields (schedule, link, deadline, items) render as unstyled text instead of proper detail components (HoursScheduleA, LinksA, etc.). Need to map each field group type to the correct A/B component variant.
- **Mobile responsive** — detail page should stack sidebar below main on mobile
- **Post click navigation** — broadsheet homepage cards may not link to detail yet

### 9. Seed Missing Row Templates

Prototype defines 31 proven row templates (RT-01 through RT-31) plus 14 additional combinations. Implementation has ~20 row templates across 5 active layout variants.

**Missing layout variants (in type system + CSS but not implemented):**
- `pair-stack` — CSS rule `.row--pair-stack` is empty, `getRowLayout()` doesn't handle it, no row templates seeded
- `trio-mixed` — CSS rule `.row--trio-mixed` is empty, `getRowLayout()` doesn't handle it, no row templates seeded

**Missing row template recipes from prototype:**
- RT-01: Hero Image (full → FeatureHero) — needs `feature-hero` post template config
- RT-02: Photo Essay (full → FeaturePhoto) — may become image widget instead
- RT-05: Feature Notice + Feature Event (lead)
- RT-08: Feature Editorial + 2× Card Event (lead-stack) — needs `feature-editorial` post template config
- RT-09: Alert Notice + 3× Digest (lead-stack)
- RT-10: 2× Feature Spotlight person (pair)
- RT-11: 2× Number widget (pair) — widget row
- RT-14: 2× Generous Exchange (pair)
- RT-18: Gazette Story + 4× Gazette Notice (pair-stack) — needs layout variant implementation
- RT-19: 3× Number widget (trio) — widget row
- RT-20: 3× Whisper Notice (trio)
- RT-22: 3× Digest Spotlight (trio)
- RT-24 through RT-31: Mixed-family trios and cross-family combinations

**Missing specialty post template configs (components exist, DB rows don't):**
- `feature-editorial` — 2-column body, no image (RT-08)
- `feature-hero` — full-bleed image overlay (RT-01)

Additive work — seed migrations only for templates where components already exist.

### 10. Image Widget Type

Add `image` widget type. Fields: `src`, `alt`, `caption`, `credit`. Referenced in prototype RT-02 (Photo Essay) but never implemented. Distinct from post media — these are editorial images placed by the layout editor that aren't associated with a post.

### 11. Integration Tests

Project-wide gap. CLAUDE.md mandates TDD and API-edge testing but no test harness exists.

- `TestHarness` with `#[test_context]` setup (DB pool, test deps)
- Tests for HTTP handlers: posts CRUD, editions CRUD, auth flow, field group upserts
- Layout engine unit tests (pure function, trivially testable) — started this session, 13 passing (topic sections + fill_slot_group cohesion)
- Approve/publish gate tests — the Phase C `require_populated_edition` guard currently has no test coverage because the harness doesn't exist
- CI pipeline running `cargo test` on PR

---

## Session open items (minor)

Things surfaced in this session that don't warrant full queue entries.

| Item | Notes |
|------|-------|
| Aitkin edition `8e564469-60c6-4fdd-89c7-f7a38c1a2206` is `approved` with 0 slots. | Pre-existing data artifact from before the Phase C gate. Remediation: click "Regenerate Layout" in its admin page, or run a one-line SQL to reset its status to `draft`. Will drop out of the dashboard approved-count either way. |
| Local `main` is 25 commits ahead of `origin/main`. | Push when ready. |
| `DATA_CONTRACT.md` §12 open questions are still open. | Cadence (weekly / stream / webhook), extraction-confidence threshold tuning, multi-county-scope tag-spray, image licensing policy, byline-vs-attribution edge cases, priority feedback loop to Signal. Punted until ingestion (#1) is being wired. |
| 11 seed posts still have media-related audit gaps. | All gated on the media ingest pipeline (#4). |

---

## Deferred (post-MVP)

Explicitly punted. These have plans/specs but are not on the active roadmap.

| Feature | State | Doc |
|---------|-------|-----|
| **Abuse Reporting** | Backend stubs (5 HTTP handlers, Rust model). Missing: DB migration (`post_reports`), GraphQL, all UI, tests. | [ABUSE_REPORTING.md](architecture/ABUSE_REPORTING.md) |
| **Map Page** | Plan written, not started. Uses existing tables. | [MAP_PAGE_PLAN.md](architecture/MAP_PAGE_PLAN.md) |
| **Email Newsletter** | Designed (Amazon SES, subscriber tables). Not started. Most infrastructure-heavy deferred item. | [EMAIL_NEWSLETTER.md](architecture/EMAIL_NEWSLETTER.md) |
| **Weather Widgets** | 4 components ported (forecast, almanac, thermo, line). No data source API. | — |
| **Edition Currency Model** | Settled this session: "up to date" = `status === 'published'` AND `periodStart === currentMondayIso`. Dashboard UI and resolver `isStale` both use this definition. Keeping the row here only in case we revisit the week-scoping vs. rolling-latest question later. | — |
| **Ticker Strips** | Prototype shows tickers as standalone full-width items between sections. Current approach: tickers-as-rows works visually. Revisit if pacing feels wrong with real content. | — |

---

## Stale Docs

| Document | Issue |
|---|---|
| `status/BROADSHEET_DESIGN_IMPORT.md` | Says migrations 183/184 "NOT YET APPLIED" — applied long ago (schema now at 211) |
| `architecture/ROOT_EDITORIAL_PIVOT.md` | Lists Q1–Q10 open questions, several answered by implementation. Needs pass to close resolved Qs. |
| `architecture/DATABASE_SCHEMA.md` | Covers through migration 171, schema now at 236. Still documents dropped tables. References `description`/`summary` columns (now `body_raw`, summary dropped). Missing: organization_links, media + media_references, post_contacts → polymorphic contacts, is_pseudo on counties. |
| `status/FINAL_SCHEMA_SUMMARY.md` | May reference `description`/`description_markdown`/`summary` columns — renamed/dropped in migration 211. |
| `architecture/ROOT_SIGNAL_SPEC.md` | Marked superseded by `ROOT_SIGNAL_DATA_CONTRACT.md`. Kept for history but should not be cited in new work. |
| `guides/ROOT_SIGNAL_INGEST_SPEC.md` | Same — merged into the authoritative data contract. |
