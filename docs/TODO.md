# Root Editorial — Outstanding Work

> **Last updated:** 2026-04-22
>
> What's done, what's next, and what's punted. This is the single source of truth for prioritization.
>
> **2026-04-22 additions:** Full Root Signal handoff package landed at [`docs/handoff-root-signal/`](handoff-root-signal/README.md) — API request, taxonomy expansion brief, tag vocabulary reference. Companion internal tracking in [`docs/status/2026_04_22_ROOT_SIGNAL_INTEGRATION_GAPS.md`](status/2026_04_22_ROOT_SIGNAL_INTEGRATION_GAPS.md). The queue items below for #1/#2/#3 are now individually specified with acceptance criteria in the gaps doc.

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

### 1. Root Signal Integration — All Editorial-side work the handoff assumes complete

The handoff package at [`docs/handoff-root-signal/`](handoff-root-signal/README.md) is written as a specification to Root Signal, with all Editorial-side infrastructure presented as in place. Every item below is work we've committed to having done before Root Signal returns with the built integration. Order of implementation is roughly top-down; most items are independent.

- **Contract (✅ done):** [ROOT_SIGNAL_DATA_CONTRACT.md](architecture/ROOT_SIGNAL_DATA_CONTRACT.md) — authoritative on-the-wire spec.
- **Handoff package (✅ done):** [ROOT_SIGNAL_API_REQUEST.md](handoff-root-signal/ROOT_SIGNAL_API_REQUEST.md), [TAXONOMY_EXPANSION_BRIEF.md](handoff-root-signal/TAXONOMY_EXPANSION_BRIEF.md), [TAG_VOCABULARY.md](handoff-root-signal/TAG_VOCABULARY.md), [handoff README](handoff-root-signal/README.md).
- **Integration gaps doc (✅ done):** [status/2026_04_22_ROOT_SIGNAL_INTEGRATION_GAPS.md](status/2026_04_22_ROOT_SIGNAL_INTEGRATION_GAPS.md) — internal punch list with dependencies.

#### 1.1 Ingest endpoint (core build)

- New ingest-compliant handler. Current `POST /Posts/create_post` at `packages/server/src/api/routes/posts.rs:1513-1532` is a 7-field admin stub; replace with a handler accepting the full envelope in [ROOT_SIGNAL_DATA_CONTRACT.md](architecture/ROOT_SIGNAL_DATA_CONTRACT.md) §2.
- `ServiceClient` auth extractor in `packages/server/src/api/auth.rs`. Machine-token Bearer validation (hash lookup, scope check, last_used_at update). Scope: `posts:create`.
- `api_keys` table: `id`, `client_name`, `prefix`, `token_hash`, `scopes[]`, `rotated_from_id`, `created_at`, `revoked_at`, `last_used_at`. Partial index on `token_hash` where `revoked_at IS NULL`.
- `dev-cli apikey` subcommands: `issue`, `rotate`, `revoke`, `list`. Token format: `rsk_{env}_<32-char-url-safe-base64>`. Plaintext shown once at issuance; only SHA-256 stored.
- `ApiError::Validation(Vec<FieldError>)` variant returning structured 422 body per handoff §11. Extend `packages/server/src/api/error.rs`.
- `api_idempotency_keys` table: `key UUID PK`, `api_key_id`, `payload_hash`, `response_status`, `response_body JSONB`, `created_at`. Canonicalised SHA-256 comparison (sorted keys, whitespace stripped). Hourly cleanup job deletes rows older than 24h.
- Organisation dedup ladder activity per handoff §7.1: `already_known_org_id` → website domain match → exact name match → insert.
- `source_individuals` table + individual dedup ladder per handoff §7.2: `(platform, handle)` → `platform_url` → insert. Consent-gated (`consent_to_publish = false` → `in_review`).
- Editor-only field rejection on ingest path: reject submissions that set `is_urgent`, `pencil_mark`, or `status`.
- Tag resolution across all kinds (topic / service_area / safety / population). Unknown `service_area` and `safety` hard-fail; unknown `topic` and `population` auto-create and flag `in_review`.
- Populate `service_areas` and `post_locations` tables alongside tag rows (currently dead weight — exist from migration 000107 but never populated).
- Return `post_id`, `organization_id`, `individual_id`, `idempotency_key_seen_before` on 201.

#### 1.2 Media pipeline (handoff assumes live from day one)

- Server-side fetch of `source_image_url`: 5s timeout, 5 MiB cap, follow redirects, HTTPS-only, SSRF guards (refuse localhost, private IPs, link-local, `file://`).
- Magic-bytes validation (JPEG/PNG/WebP/AVIF).
- EXIF strip + WebP normalisation (quality 85).
- SHA-256 content-hash dedup against existing `media` rows.
- Store to MinIO under `media/{yyyy}/{mm}/{uuid}.webp`; populate `post_media.media_id`.
- Spec: [ROOT_SIGNAL_MEDIA_INGEST.md](guides/ROOT_SIGNAL_MEDIA_INGEST.md).

#### 1.3 Signal Inbox admin UI

- `/admin/signal-inbox` page listing posts with `status = in_review`.
- Group by flag reason: `source_stale`, `low_confidence`, `possible_duplicate`, `individual_no_consent`, `deck_missing_on_heavy`, etc.
- Per-post actions: approve (→ `active`), reject (→ `rejected`), open-for-edit, merge-if-duplicate.
- Side-by-side view: extracted payload + source URL.
- Plan: [SIGNAL_INBOX.md](architecture/SIGNAL_INBOX.md).

#### 1.4 Revision auto-reflow

- On ingest with `revision_of_post_id`: archive prior post, chain revision, **and auto-reflow any active edition that contained it.** Reflow runs the layout engine against the edition's slot configuration; editor sees the updated layout when they next open the edition. No manual "regenerate" click required.

#### 1.5 Content-hash dedup on posts

- `posts.content_hash TEXT` column + index. Hash function: SHA-256 over normalised title + `source_url` + day-bucket(published_at) + sorted service_area slugs.
- On ingest, if hash matches an existing row, refresh that row's `published_at` (extends 7-day eligibility) and return its `post_id` with a stored response. No duplicate insert.
- Internal protection against sources that produce slightly-different-but-same content across scrapes.
- Design: [POST_EDITION_LIFECYCLE.md](guides/POST_EDITION_LIFECYCLE.md) §"Dedup options".

#### 1.6 Citation rendering

- Parse `[signal:UUID]` tokens in `body_raw` / `body_heavy` / `body_medium` at render time.
- Render as superscript citations (`[1]`, `[2]`, …) with popovers showing signal title + source URL + summary.
- Link target: configurable via env var (Signal provides URL pattern at kickoff; we default to `https://signal.example.com/signals/<uuid>`).
- Graceful fallback if URL pattern is empty: show citation as unlinked superscript.

#### 1.7 Tag vocabulary cleanup + expansion (before handoff is acted upon)

- **Drop the `population` tag kind entirely.** People aren't single-bucket identities, and anything the kind was trying to capture fits better in open-ended topic tags. Remove the kind from the schema, drop any seeded rows, and remove `population` from the CHECK constraint on `tags.kind`.
- **Clean up topic vocabulary.** `data/tags.json` topic kind currently mixes in neighborhood slugs and a category slug (`brooklyn-center`, `phillips`, `north-minneapolis`, `lake-street`, `south-metro`, `restaurant`). Move neighborhood slugs to a new `neighborhood` tag kind or drop entirely; drop `restaurant` (business posts use `post_type=business`, not a topic tag). Add `public-works` which is used in seed but undeclared.
- **Rework the safety vocabulary as access-policy modifiers**, per handoff [`TAG_VOCABULARY.md`](handoff-root-signal/TAG_VOCABULARY.md) §3. The three existing slugs (`no_id_required`, `ice_safe`, `know_your_rights`) become `no-id-required`, `ice-safe` (keep); `know-your-rights` drops (it's a content/topic concept, not a policy modifier) — migrate any references to `topic` tags. Seed the full expanded vocabulary (~29 slugs across identity-and-documentation, cost, privacy, procedure, cultural affirmation, accessibility, substance use, minors, law enforcement, and family logistics). See the handoff doc for the authoritative list.
- **Normalise safety slugs to hyphen-case** (currently underscore-case): `no_id_required` → `no-id-required`, etc. All other tag kinds already use hyphens.
- Update seed posts in `data/posts.json` to reference the cleaned-up topic slugs.
- Re-run `make audit-seed --rebaseline` to update the audit baseline.

#### 1.8 Tag kind constraint cleanup

- Update `tags.kind` CHECK constraint to the final set: `CHECK (kind IN ('topic', 'service_area', 'safety', 'neighborhood'))` — no population, no retired kinds.
- Drop any tag rows under retired kinds (`reserved`, `post_type`, `structure`, `audience_role`). No real data yet; clean implementation, no legacy baggage.

#### 1.9 Route path and convention cleanup

- Confirm `/Posts/create_post` convention documented in CLAUDE.md (capital-P `/{Service}/{handler}`) is still the standard. No blockers here; just noting the cross-reference.

#### 1.10 Assumptions we're not building (intentionally omitted from handoff)

- **HMAC body signing.** Bearer token over HTTPS is sufficient for the threat model. If we later need replay protection beyond what idempotency keys provide, we add HMAC then.
- **Feedback webhook to Signal.** Editorial → Signal lifecycle notifications (published / rejected / edited) would be useful as training signal for them, but they don't need it to build the ingest integration. Out of scope for this cycle.
- **Rate-limit auto-tuning / per-day quotas.** Start with 15/50 req/sec token bucket; adjust empirically.

### 2. Fresh Week Batch Generation Cron

Currently the only way to get a new week's editions generated is to click the dashboard CTA. For an unattended deploy we need a scheduled task that runs every Sunday night to prep the next Mon–Sun period's editions across every county (incl. Statewide).

- **Scope:** Scheduled task (Rust-side or container cron) invoking `batch_generate_editions`. Must be idempotent — re-running mid-week shouldn't clobber editor edits. The activity's existing status-check already gates this, but verify under real cron conditions.

### 3. Bulk Actions on the Editions List View

Dashboard has "Publish all N approved"; `/admin/editions` list doesn't. Nice-to-have second surface for editors who filter the list first.

- **Scope:** Multi-select on the editions table; bulk `publishEdition` loop (same shape as the dashboard handler); probably bulk `archiveEdition` too.

### 4. IP Geolocation for County Picker Default (deferred)

Public home defaults to the Statewide pseudo-county when the URL is bare. Better: auto-select a county based on the visitor's IP for MN visitors; fall back to Statewide otherwise.

- **Scope:** Pick a backend (MaxMind GeoLite2 / ipinfo.io / Cloudflare `CF-IPCountry`+`CF-Region` headers if we deploy behind CF). Server-side resolver that maps IP → county row.
- **Blocked on:** Infrastructure / vendor decision.

### 5. Post Detail Page Polish

Core detail layout done (NewspaperFrame, ArticlePage, field group components, related posts, SiteFooter). Remaining:

- **Full component audit** — sidebar fields (schedule, link, deadline, items) render as unstyled text instead of proper detail components (HoursScheduleA, LinksA, etc.). Need to map each field group type to the correct A/B component variant.
- **Mobile responsive** — detail page should stack sidebar below main on mobile
- **Post click navigation** — broadsheet homepage cards may not link to detail yet

### 6. Seed Missing Row Templates

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

### 7. Image Widget Type

Add `image` widget type. Fields: `src`, `alt`, `caption`, `credit`. Referenced in prototype RT-02 (Photo Essay) but never implemented. Distinct from post media — these are editorial images placed by the layout editor that aren't associated with a post.

### 8. Integration Tests

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
| Root Signal's proposed `Condition` signal type has no direct Editorial `post_type` mapping. | Resolved as "drop unless newsworthy, in which case map to `story`" in request doc §15.1. Revisit if Signal produces significant Condition volume. |
| Root Signal's proposed `Job` signal may need a new Editorial `job` post_type. | Three options in request doc §17 Q9: add new type, map to `action`, or defer. Decide during Phase 2 scoping. |
| Schema/seed drift: `post_locations`, `service_areas`, polymorphic `schedules` tables exist (migration 000107) but seed populates via tags instead. | Gaps doc §1.8. Recommendation is to populate both on ingest as secondary indices. 1 day of work. |
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
