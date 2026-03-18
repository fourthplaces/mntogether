# Root Editorial — Outstanding Work

> **Last updated:** 2026-03-17
>
> What's done, what's next, and what's punted. This is the single source of truth for prioritization.

---

## Completed Phases

| Phase | Summary | Postmortem |
|-------|---------|------------|
| **1. Dead Code Removal** | 45,947 lines removed. 11 domains, 4 packages deleted. | [Phase 1](status/PHASE_1_DEAD_CODE_REMOVAL.md) |
| **2. Post Types** | 6-type system (story/notice/exchange/event/spotlight/reference). 7 field group tables. | [Phase 2](status/PHASE_2_POST_TYPES.md) |
| **3. Edition System** | 87-county edition model, layout engine, batch generation, admin pages. | [Phase 3](status/PHASE_3_EDITION_SYSTEM.md) |
| **4. CMS UX + Broadsheet** | Editorial dashboard, kanban workflow, broadsheet rendering (43 post components, 9 widget components, 3,623 lines CSS), widget system, shadcn admin rebuild. Dead code cleanup (BusinessPost, scored_at, capacity_status, heat_map, memo_cache, agents, AI/embeddings). | [Phase 4](status/PHASE4_CMS_UX_REWORK.md), [Broadsheet](status/BROADSHEET_DESIGN_IMPORT.md) |

---

## Active Work Queue

Priority order. Each item unblocks the ones below it.

### 1. Field Group Hydration ⚠️ Critical — visual fidelity depends on this

The broadsheet GraphQL query (`PublicBroadsheetQuery`, `EditionPreviewQuery`) fetches base post fields only. **No field group data flows to the renderer.** Half the 43 post components render empty sections because the data isn't there.

**Missing from broadsheet query — needed by components:**
- `media` (image, caption, credit) → FeatureStory, FeatureSpotlight, all detail pages
- `person` (name, role, photo, bio, quote) → FeatureSpotlight, BroadsheetSpotlight, DigestSpotlight
- `items[]` (name, detail) → PinboardExchange, GazetteExchange, DirectoryRef, QuickRef, GazetteReference
- `datetime` (start, end, cost, recurring) → CardEvent, GazetteEvent, BulletinEvent, LedgerEvent, TickerEvent
- `link` (label, url, deadline) → AlertNotice, FeatureNotice (action variant)
- `source` (name, attribution) → GazetteNotice, BulletinNotice, TickerNotice, most templates
- `meta` (kicker, byline, deck, timestamp, updated) → FeatureStory, GazetteStory, BulletinStory, DigestStory
- `status` (state, verified) → PinboardExchange, GazetteExchange, GenerousExchange
- `schedule` (entries[]) → detail pages (HoursHeat, HoursSchedule, etc.)

**Work required:**
1. Build or verify Rust models for field group tables (`PostMedia`, `PostPerson`, `PostItems`, `PostLink`, `PostSourceAttribution`, `PostMeta`) — Phase 2 DB tables exist but Rust models may not
2. Add field group fetching to edition/broadsheet GraphQL resolvers (DataLoader pattern to avoid N+1)
3. Extend broadsheet GraphQL queries to include field groups on each post
4. Update `Post` type in `web-app/lib/broadsheet/types.ts`

Without this, the broadsheet is a skeleton of titles and descriptions.

### 2. Render Hint Functions

Prototype defines computed display fields derived at render time. None are implemented.

**Fields:** `paragraphs`, `cols`, `dropCap`, `pullQuote`, `clamp`, `tagLabel`, `contactDisplay`, `month`, `day`, `dow`, `when`, `circleLabel`, `date`, `count`, `tagline`, `readMore`

**Work required:**
- Create `lib/broadsheet/render-hints.ts` in web-app
- Pure function: `computeRenderHints(post, fieldGroups): RenderHints`
- Date formatting, string splitting, conditional logic (~100 lines)
- All 43 components import and use this instead of ad-hoc formatting
- Depends on field group hydration (#1) for datetime, items, person, etc.

### 3. Widget Template System

Widgets have no variant/template concept. Components can't adapt rendering based on context (e.g., span-2 vs span-3, visual style variant).

**Three fixes:**

**a. Merge `stat_card` + `number_block` → single `number` type with variants.**
Both are "big number + heading + blurb" styled differently. `section_sep` is also "heading + blurb" with no number. Merged `number` type keeps all fields (`number`, `title`, `body`, `color`); variant controls rendering (compact card vs colored tile).

**b. Fix `SectionSep` variants.**
`LedgerSectionBreak` exists in `posts/LedgerSectionBreak.tsx` as **dead code** — never imported, never registered in the template registry, takes a `Post` type with `d.sub` which doesn't exist on the actual Post struct. It's a centered, larger-text variant of `SectionSep` (both CSS styles exist in `broadsheet.css`). This was a prototyping mistake — they're the same widget, two visual treatments.
- Delete `LedgerSectionBreak.tsx`
- Add `variant` prop to `SectionSep` component (`"default"` | `"ledger"`)
- Both CSS classes (`.section-sep` and `.led-section-break`) already exist

**c. Add `widget_template` to edition slots.**
Post slots have `post_template`; widget slots need the equivalent. Add `widget_template TEXT` to `edition_slots` (nullable, for widget slots). Keep separate from `post_template` — don't unify. Slot `kind` already discriminates.

**Also needed:**
- Add `image` widget type (not yet specced). Fields: `src`, `alt`, `caption`, `credit`. Referenced in prototype RT-02 (Photo Essay) but never implemented.
- Admin template picker shows widget variants when slot kind is `widget`
- Centralize widget rendering: create a `resolveWidget(widgetType, widgetTemplate, span)` function in `web-app/lib/broadsheet/` (parallel to `resolveTemplate` for posts). Today widget `data` JSON is parsed inline in `BroadsheetRenderer`; this will break down as variant count grows with the image widget and number type merge. This function should own JSON parsing, variant resolution, and component selection — single entry point for all widget rendering.

**Section separator architecture note:** Rendering a section separator currently requires: Widget record → edition_slot (kind=widget) → edition_row (template=widget-standalone) → BroadsheetRenderer detects layout variant → skips Row/Cell wrapper → renders SectionSep. Three table records and a special-case render path for a horizontal line with a title. This exists because we intentionally decoupled separators from sections (thrashed on this — see commits `c6381b0` → `0bd0d2f` → `a80e2df`). Editors should place separators wherever they want, or not at all. The "sections as parents of rows" concept may be reworked or removed once Root Signal integration clarifies the broadsheet data flow. If sections get removed, the widget approach is already correct.

### 4. Story Editor

Create and edit posts from the admin UI. Currently the CMS is read-only for post content.

- **Plan:** [STORY_EDITOR.md](architecture/phase4/STORY_EDITOR.md)
- **Stack:** Plate.js (Slate-based WYSIWYG), markdown round-tripping via `@platejs/markdown`
- **Scope:**
  - `/admin/posts/new` — creation page with type selector, field groups, Plate.js editor
  - `/admin/posts/[id]` — inline edit mode on existing detail page
  - `createPost` + `updatePost` HTTP endpoints + GraphQL mutations
  - Auto-generate `description` (plain text) from `description_markdown` on save
- **Unblocks:** Signal Inbox, editorial workflow, field group UI

### 5. Root Signal Integration

Connect the CMS to Root Signal so AI-analyzed content flows into editions automatically.

- **Plan:** [ROOT_SIGNAL_SPEC.md](architecture/ROOT_SIGNAL_SPEC.md) (draft API contract)
- **Scope:**
  - Finalize API contract (request/response format, auth, cadence)
  - Build ingestion endpoint — receives Signal output, upserts posts with `submission_type = 'signal'`
  - Map Signal fields to post columns (weight, priority, weight-body text, tags, topic)
  - Wire into batch generation flow: Signal runs → posts enriched → `batch_generate_editions`
- **May affect:** Whether "sections" survive as a concept, how editions are structured, broadsheet data flow

### 6. Signal Inbox

Triage UI for incoming Root Signal content.

- **Plan:** [SIGNAL_INBOX.md](architecture/phase4/SIGNAL_INBOX.md)
- **Depends on:** Story Editor (#4) for "Edit & Approve" flow, Root Signal (#5) for real data
- **Scope:** Admin page with filtered post list, bulk approve/reject, edit-before-approve flow

### 7. Broadsheet Detail Pages

Clicking a post on the broadsheet does nothing — 15 detail page components are ported but not routed.

- **Location:** `packages/web-app/components/broadsheet/detail/`
- **Components:** `ArticlePage`, `Title`, `Kicker`, `BodyA`, `BodyB`, `Photo`, `List`, `Links`, `Audio`, `Phone`, `Address`, `ArticleMeta`, `ArticleNav`, `Related`, `SidebarCard`, plus 5 hours visualizations
- **Scope:** Mount to `/posts/[id]` route, wire GraphQL query, link broadsheet post clicks
- Self-contained — no impact on existing code

### 8. Seed Missing Row Templates

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
- RT-11: 2× Number Block (pair) — widget row
- RT-14: 2× Generous Exchange (pair)
- RT-18: Gazette Story + 4× Gazette Notice (pair-stack) — needs layout variant implementation
- RT-19: 3× Stat Card (trio) — widget row
- RT-20: 3× Whisper Notice (trio)
- RT-22: 3× Digest Spotlight (trio)
- RT-24 through RT-31: Mixed-family trios and cross-family combinations

**Missing specialty post template configs (components exist, DB rows don't):**
- `feature-editorial` — 2-column body, no image (RT-08)
- `feature-hero` — full-bleed image overlay (RT-01)

Additive work — seed migrations only for templates where components already exist.

### 9. Overhaul Seed / Test Data

The current seed data (`000185_seed_diverse_posts.sql`) creates ~35 posts with titles and descriptions only. **Zero field group records** — no `post_media`, `post_person`, `post_items`, `post_meta`, `post_link`, `post_source_attribution`, `post_datetime`, `post_schedule`, or `post_status` rows. Even once field group hydration (#1) is wired up, there's nothing to display.

Seed data should exercise the full visual range of the prototype:
- Posts with images + captions + credits (FeatureStory, FeatureSpotlight)
- Spotlight posts with person profiles (name, role, photo, bio, quote)
- Exchange posts with items lists and status (PinboardExchange, GenerousExchange)
- Events with datetime fields (CardEvent, GazetteEvent, BulletinEvent)
- Notices with source attribution (GazetteNotice, AlertNotice)
- Stories with meta fields (kicker, byline, deck, timestamp)
- Reference posts with items lists (DirectoryRef, QuickRef)
- Widget seed data covering all types and variants (including merged number type)
- Enough variety per weight tier (heavy/medium/light) to fill the full RT-01 through RT-31 row template catalog

This is a prerequisite for visually validating that field group hydration, render hints, and row templates all work together.

### 10. Centralize Widget Rendering (`resolveWidget`)

Widget data JSON is currently parsed inline in `BroadsheetRenderer.tsx` with a switch on `widget_type`. This works for 6 types but breaks down as variant count grows (number type merge, image widget, SectionSep variants).

**Work required:**
- Create `lib/broadsheet/widget-resolver.ts` in web-app
- `resolveWidget(widgetType, widgetTemplate, span): WidgetComponent` — parallel to `resolveTemplate` for posts
- Owns JSON parsing, variant resolution, and component selection — single entry point
- Move all widget rendering logic out of BroadsheetRenderer into this module
- Depends on widget template system (#3) for the `widgetTemplate` parameter

### 11. Integration Tests

Project-wide gap. CLAUDE.md mandates TDD and API-edge testing but no test harness exists.

- `TestHarness` with `#[test_context]` setup (DB pool, test deps)
- Tests for HTTP handlers: posts CRUD, editions CRUD, auth flow
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
| `status/BROADSHEET_DESIGN_IMPORT.md` | Says migrations 183/184 "NOT YET APPLIED" — applied long ago (schema now at 206) |
| `architecture/ROOT_EDITORIAL_PIVOT.md` | Lists Q1–Q10 open questions, several answered by implementation. Needs pass to close resolved Qs. |
| `architecture/DATABASE_SCHEMA.md` | Covers through migration 171, schema now at 206. Still documents dropped tables. |
| `guides/TESTING_WORKFLOWS.md` | References Restate workflow testing patterns that may have shifted. |
