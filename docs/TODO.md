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
| **4. CMS UX + Broadsheet** | Editorial dashboard, kanban workflow, broadsheet rendering (45 components, 3,623 lines CSS), widget system, shadcn admin rebuild. Dead code cleanup (BusinessPost, scored_at, capacity_status, heat_map, memo_cache, agents, AI/embeddings). | [Phase 4](status/PHASE4_CMS_UX_REWORK.md), [Broadsheet](status/BROADSHEET_DESIGN_IMPORT.md) |

---

## Active Work Queue

Priority order. Each item unblocks the ones below it.

### 1. Story Editor ⭐ highest priority

Create and edit posts from the admin UI. Currently the CMS is read-only — no `createPost` or `updatePost` mutations exist.

- **Plan:** [STORY_EDITOR.md](architecture/phase4/STORY_EDITOR.md)
- **Stack:** Plate.js (Slate-based WYSIWYG), markdown round-tripping via `@platejs/markdown`
- **Scope:**
  - `/admin/posts/new` — creation page with type selector, field groups, Plate.js editor
  - `/admin/posts/[id]` — inline edit mode on existing detail page
  - `createPost` + `updatePost` HTTP endpoints + GraphQL mutations
  - Auto-generate `description` (plain text) from `description_markdown` on save
- **Unblocks:** Signal Inbox, editorial workflow, field group UI

### 2. Root Signal Integration

Connect the CMS to Craig's Root Signal service so AI-analyzed content flows into editions automatically.

- **Plan:** [ROOT_SIGNAL_SPEC.md](architecture/ROOT_SIGNAL_SPEC.md) (draft API contract)
- **Scope:**
  - Finalize API contract with Craig (request/response format, auth, cadence)
  - Build ingestion endpoint — receives Signal output, upserts posts with `submission_type = 'signal'`
  - Map Signal fields to post columns (weight, priority, weight-body text, tags, topic)
  - Wire into batch generation flow: Signal runs → posts enriched → `batch_generate_editions`
- **Blocked on:** Craig's availability for API contract discussion

### 3. Signal Inbox

Triage UI for incoming Root Signal content. Editors review, edit, approve, or reject signal items before they enter editions.

- **Plan:** [SIGNAL_INBOX.md](architecture/phase4/SIGNAL_INBOX.md)
- **Depends on:** Story Editor (#1) for "Edit & Approve" flow, Root Signal (#2) for real data
- **Scope:**
  - Admin page: filtered post list where `submission_type = 'signal' AND status = 'pending_approval'`
  - Bulk approve/reject actions
  - Edit-before-approve flow (opens story editor)
  - Can be built with mock data before Signal integration is live

### 4. Integration Tests

Project-wide gap. CLAUDE.md mandates TDD and API-edge testing but no test harness exists.

- **Scope:**
  - `TestHarness` with `#[test_context]` setup (DB pool, test deps)
  - Tests for HTTP handlers: posts CRUD, editions CRUD, auth flow
  - Layout engine unit tests (pure function, trivially testable with mock data)
  - CI pipeline running `cargo test` on PR

### 5. Broadsheet Detail Pages

Clicking a post on the broadsheet does nothing — 14 detail page components are ported but not routed.

- **Location:** `packages/web-app/components/broadsheet/detail/`
- **Scope:**
  - Mount detail components to `/posts/[id]` route (or broadsheet-scoped route)
  - Port `broadsheet-detail.css`
  - Wire GraphQL query for single post data

### 6. Specialty Component Registry

9 broadsheet components exist in code but aren't auto-rendered from CMS data:
AlertNotice, BroadsheetSpotlight, BroadsheetTickerNotice, CardEvent, DirectoryRef, GenerousExchange, PinboardExchange, QuickRef, WhisperNotice.

- **Decision needed:** CMS-driven (need post template mappings) or editorially placed (manual slot assignment)?

---

## Outstanding Polish / Tech Debt

Not blocking, but should be addressed before launch.

| Item | Context | Effort |
|------|---------|--------|
| Period-based post filtering | Layout engine uses all active posts; needs `published_at` windowing to scope to edition period | Small |
| Organization name in slots | Edition slots don't show org names; needs join through `organization_posts` | Small |
| Row variants not exercised | `pair-stack`, `trio-mixed` etc. exist in row-map but untested with real data | Small |
| CSS class name collisions | `.site-footer` styled differently in `globals.css` vs `broadsheet.css`; may need namespace strategy | Small |
| Field group Rust models | Phase 2 DB tables exist (post_items, post_media, post_person, etc.) but Rust models not yet created. Blocked on story editor. | Medium |
| DataLoader for rowTemplate | Embedded data avoids N+1 for now, but proper DataLoader would be cleaner | Small |
| `DOCKER_GUIDE.md` cleanup | May still reference dead env vars | Small |
| `DATA_MODEL.md` cleanup | May still reference volunteer/discovery sections | Small |
| `DATABASE_SCHEMA.md` refresh | Comprehensively stale — covers migration 171, schema now at 206 | Medium |

---

## Deferred (post-MVP)

Explicitly punted. These have plans/specs but are not on the active roadmap.

| Feature | State | Doc |
|---------|-------|-----|
| **Abuse Reporting** | Backend stubs (HTTP handlers, Rust model). Missing: DB migration, GraphQL, admin UI, public UI, tests. | [ABUSE_REPORTING.md](architecture/ABUSE_REPORTING.md) |
| **Map Page** | Plan written, not started. Heat map domain dropped. | [MAP_PAGE_PLAN.md](architecture/MAP_PAGE_PLAN.md) |
| **Email Newsletter** | Designed (Amazon SES, subscriber tables). Not started. Most infrastructure-heavy deferred item. | [EMAIL_NEWSLETTER.md](architecture/phase4/EMAIL_NEWSLETTER.md) |
| **Weather Widgets** | Components ported (4 widgets). No data source API. | [BROADSHEET_DESIGN_IMPORT.md](status/BROADSHEET_DESIGN_IMPORT.md) |
| **Post Status Expansion** | `draft → pending → in_review → approved → active` workflow. Needs migration + model changes. | [PHASE4_CMS_UX_REWORK.md](status/PHASE4_CMS_UX_REWORK.md) |
| **Edition Scoped Post Review** | Per-edition approve/reject within edition context. Depends on post status expansion. | [PHASE4_CMS_UX_REWORK.md](status/PHASE4_CMS_UX_REWORK.md) |
| **Edition Currency Model** | "Latest edition per county" vs week-scoped. | — |
| **Post Template Character Limits** | Body truncation per template size tier. | — |
