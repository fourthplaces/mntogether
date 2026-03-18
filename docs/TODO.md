# Root Editorial — Outstanding Work

> **Last updated:** 2026-03-18
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

## Recently Completed (this session)

| Item | Summary |
|------|---------|
| **Field Group Hydration** | 9 Rust models with batch queries, field groups flow through broadsheet pipeline (Rust → GraphQL → frontend). Migration 207 adds post_datetime, post_status, post_schedule tables + deck column on post_meta. |
| **Render Hints** | `computeRenderHints()` in `web-app/lib/broadsheet/render-hints.ts` — computes month/day/dow/when/circleLabel (events), count (items), tagline (person.role), pullQuote, date from field group data. |
| **Widget Template System** | Merged stat_card + number_block → single `number` type. Added `widget_template` column to edition_slots. SectionSep supports default/ledger variants. Deleted dead LedgerSectionBreak.tsx. Migration 209. |
| **Seed Data Overhaul** | Migration 208 seeds field group records for all 35 posts: media, meta, source attribution, items, status, datetime, person, schedule. |
| **resolveWidget** | Centralized widget rendering in `web-app/lib/broadsheet/widget-resolver.ts` — parallel to `resolveTemplate` for posts. Owns JSON parsing, variant resolution, component selection. |
| **Story Editor** | Plate.js WYSIWYG replaces markdown textarea. ArticlePreview with web-app body styling. Field group panels on edit page (media, meta, person) and detail page (link, source, datetime, status, items). 7 upsert endpoints + GraphQL mutations. "Open Preview" button in EditorTopBar. |

---

## Active Work Queue

Priority order. Each item unblocks the ones below it.

### 1. Root Signal Integration

Connect the CMS to Root Signal so AI-analyzed content flows into editions automatically.

- **Plan:** [ROOT_SIGNAL_SPEC.md](architecture/ROOT_SIGNAL_SPEC.md) (draft API contract)
- **Scope:**
  - Finalize API contract (request/response format, auth, cadence)
  - Build ingestion endpoint — receives Signal output, upserts posts with `submission_type = 'signal'`
  - Map Signal fields to post columns (weight, priority, weight-body text, tags, topic)
  - Wire into batch generation flow: Signal runs → posts enriched → `batch_generate_editions`
- **May affect:** Whether "sections" survive as a concept, how editions are structured, broadsheet data flow

### 2. Signal Inbox

Triage UI for incoming Root Signal content.

- **Plan:** [SIGNAL_INBOX.md](architecture/phase4/SIGNAL_INBOX.md)
- **Depends on:** Story Editor (✅ done) for "Edit & Approve" flow, Root Signal (#1) for real data
- **Scope:** Admin page with filtered post list, bulk approve/reject, edit-before-approve flow

### 3. Broadsheet Detail Pages

Clicking a post on the broadsheet does nothing — 15 detail page components are ported but not routed.

- **Location:** `packages/web-app/components/broadsheet/detail/`
- **Components:** `ArticlePage`, `Title`, `Kicker`, `BodyA`, `BodyB`, `Photo`, `List`, `Links`, `Audio`, `Phone`, `Address`, `ArticleMeta`, `ArticleNav`, `Related`, `SidebarCard`, plus 5 hours visualizations
- **Scope:** Mount to `/posts/[id]` route, wire GraphQL query, link broadsheet post clicks
- Self-contained — no impact on existing code

### 4. Seed Missing Row Templates

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

### 5. Image Widget Type

Add `image` widget type. Fields: `src`, `alt`, `caption`, `credit`. Referenced in prototype RT-02 (Photo Essay) but never implemented. Distinct from post media — these are editorial images placed by the layout editor that aren't associated with a post.

### 6. Integration Tests

Project-wide gap. CLAUDE.md mandates TDD and API-edge testing but no test harness exists.

- `TestHarness` with `#[test_context]` setup (DB pool, test deps)
- Tests for HTTP handlers: posts CRUD, editions CRUD, auth flow, field group upserts
- Layout engine unit tests (pure function, trivially testable)
- CI pipeline running `cargo test` on PR

---

## Deferred (post-MVP)

Explicitly punted. These have plans/specs but are not on the active roadmap.

| Feature | State | Doc |
|---------|-------|-----|
| **Abuse Reporting** | Backend stubs (5 HTTP handlers, Rust model). Missing: DB migration (`post_reports`), GraphQL, all UI, tests. | [ABUSE_REPORTING.md](architecture/ABUSE_REPORTING.md) |
| **Map Page** | Plan written, not started. Uses existing tables. | [MAP_PAGE_PLAN.md](architecture/MAP_PAGE_PLAN.md) |
| **Email Newsletter** | Designed (Amazon SES, subscriber tables). Not started. Most infrastructure-heavy deferred item. | [EMAIL_NEWSLETTER.md](architecture/phase4/EMAIL_NEWSLETTER.md) |
| **Weather Widgets** | 4 components ported (forecast, almanac, thermo, line). No data source API. | — |
| **Edition Currency Model** | "Latest edition per county" vs week-scoped. | — |
| **Ticker Strips** | Prototype shows tickers as standalone full-width items between sections. Current approach: tickers-as-rows works visually. Revisit if pacing feels wrong with real content. | — |

---

## Stale Docs

| Document | Issue |
|---|---|
| `status/BROADSHEET_DESIGN_IMPORT.md` | Says migrations 183/184 "NOT YET APPLIED" — applied long ago (schema now at 209) |
| `architecture/ROOT_EDITORIAL_PIVOT.md` | Lists Q1–Q10 open questions, several answered by implementation. Needs pass to close resolved Qs. |
| `architecture/DATABASE_SCHEMA.md` | Covers through migration 171, schema now at 209. Still documents dropped tables. |
| `guides/TESTING_WORKFLOWS.md` | References Restate workflow testing patterns that may have shifted. |
