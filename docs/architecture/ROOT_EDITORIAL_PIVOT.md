# Root Editorial Pivot

**Date:** 2026-02-24
**Author:** Tim (design/editorial lead) + Claude
**Status:** Historical — the pivot rationale as it stood in Feb 2026.

> **Heads up.** This document captures the *why* of the pivot from the monolith to Root Editorial + Root Signal. It is kept for the historical record. **For the current data model and current system shape, read [`DATA_MODEL.md`](DATA_MODEL.md).** Enumerations below ("what Editorial does", "what tables we keep", etc.) reflect the Feb 2026 intent, not what actually shipped.

---

## What Is This Document?

This is the reference bible for the pivot of the `mntogether` repository from a monolithic platform into **Root Editorial** — an open CMS purpose-built for community journalism. Read this before touching the codebase.

---

## The Three Layers

```
┌─────────────────────────────────────────────────────────┐
│  Root Signal  (Craig's repo, separate)                  │
│  AI-powered signal discovery. Crawls the web, finds     │
│  what matters, delivers organized content to Editorial.  │
│  We don't touch this. We consume its output.            │
└──────────────────────┬──────────────────────────────────┘
                       │  API (TBD — daily/weekly fetch)
                       ▼
┌─────────────────────────────────────────────────────────┐
│  Root Editorial  (this repo)                            │
│  The CMS. Receives signal, adds human stories,          │
│  auto-generates broadsheet editions, gives editors      │
│  control. Owns the database, auth, public site.         │
└──────────────────────┬──────────────────────────────────┘
                       │  serves
                       ▼
┌─────────────────────────────────────────────────────────┐
│  MN Together  (visual theme + first instance)           │
│  The public site at mntogether.org.                     │
│  A 3-column broadsheet layout — the first "skin"        │
│  applied to Root Editorial. Other instances could        │
│  follow (e.g., per-state community editions).           │
└─────────────────────────────────────────────────────────┘
```

### What Root Editorial Is

1. A CMS GUI for non-technical community editors (markdown authoring, story management)
2. A semi-automated daily/weekly digital broadsheet (newspaper without the paper)
3. A first-class consumer of Root Signal data (organized content arrives ready to publish)
4. An auto-generated edition preview, with editorial control over placement and priority
5. A layout engine that understands 3-column broadsheet layout and places stories intelligently
6. An email newsletter system (shorter email version of the weekly edition)
7. A weather widget that runs daily per county/city

### What Root Editorial Is NOT

- It does **not** crawl websites or discover content (Root Signal does that)
- It does **not** run extraction pipelines or LLM-powered content parsing
- It does **not** manage scraping infrastructure (Firecrawl, Apify, Tavily)
- It does **not** do deduplication or signal analysis

### The Ideal User Story

> "I have an hour or two a week to check and prep the edition going out. The AI gives me a 95% ready broadsheet, then I tweak or add."

---

## The Repo Before The Pivot

### Current Package Structure

```
packages/
├── admin-app/          ← ALIVE: Becomes the CMS interface
├── web-app/            ← ALIVE: Public-facing site (MN Together)
├── search-app/         ← DEAD: Stub, never built out
├── shared/             ← ALIVE: GraphQL schema, resolvers, shared types
├── server/             ← ALIVE: Rust backend (needs pruning)
├── ai-client/          ← REMOVED (2026-03-10): AI handled by Root Signal
├── twilio-rs/          ← ALIVE: SMS OTP auth
├── extraction/         ← DEAD: Web extraction library (Root Signal concern)
├── openai-client/      ← DEAD: Superseded by ai-client
├── apify-client/       ← DEAD: Web scraping (Root Signal concern)
└── web/                ← DEAD: Original monolith, superseded by app split
```

### Server Domains — What Stays, What Goes

#### ALIVE (Core CMS)

| Domain | Purpose |
|--------|---------|
| **auth** | Phone-based OTP login via Twilio, JWT tokens |
| **member** | User profiles, roles |
| **posts** | The core content entity — listings, stories, events |
| **organization** | Orgs that posts belong to |
| **tag** | Taxonomy system (post types, services, populations) |
| **notes** | Editorial annotations on posts/orgs |
| **jobs** | Background job tracking |
| **locations** | Zip codes, geographic data, proximity search |

#### DEAD (Root Signal Concerns)

| Domain | Why It's Dead |
|--------|--------------|
| **crawling** | Web scraping orchestration → Root Signal |
| **extraction** | LLM-powered content parsing → Root Signal |
| **website** | Website approval/assessment → Root Signal |
| **social_profile** | Instagram/TikTok tracking → Root Signal |
| **sync** | Deduplication/merge proposals → Root Signal |
| **curator** | Content curation/filtering → Root Signal |
| **newsletter** | Inbound email webhook/parsing → Root Signal |
| **providers** | Service provider directory → not needed |
| **chatrooms** | ~~Planned for removal~~ → rescued in Phase 1, powers post comments |
| **contacts** | Alive — CRUD added to admin post detail in Phase 4 |
| **schedules** | Alive — CRUD added to admin post detail in Phase 4 |

### External Services — What Stays, What Goes

#### STAYS

| Service | Purpose |
|---------|---------|
| PostgreSQL + pgvector | Database (pgvector retained for future use; AI columns removed) |
| Twilio | SMS OTP authentication |

> **Note (2026-03-17):** Restate was removed — all services migrated to direct Axum HTTP handlers. OpenAI/OpenRouter removed — AI work handled externally by Root Signal.

#### GOES

| Service | Why |
|---------|-----|
| Tavily | Web search/discovery → Root Signal |
| Firecrawl | Web scraping → Root Signal |
| Apify | Social media scraping → Root Signal |
| Expo | Push notifications → not needed for CMS |
| Voyage AI | Embeddings → switched to OpenAI |
| NATS | Real-time streaming → no real-time features in CMS |

---

## The Post Entity

Posts are the core content object. The data model stays, but the type system expands significantly.

### Current Post Types (in database)

```
service | opportunity | business | professional
```

### Target Post Types

| Type | Description |
|------|-------------|
| **Story** | Narrative, human-written or editorial content |
| **Request — Volunteer** | People/time needed |
| **Request — Donation** | Items or money needed |
| **Request — Housing/Shelter** | Housing assistance needed |
| **Offer — Volunteer** | People/time available |
| **Offer — Donation** | Items or money available |
| **Offer — Housing/Shelter** | Housing available |
| **Event** | Community events, gatherings |
| **Support Local** | Business listing (economic solidarity) |
| **Call to Action** | Contact campaigns, petitions, advocacy |
| **Resource/Guide** | Evergreen reference material |
| **Update** | Brief news item |

### Modifiers (can apply to most types)

| Modifier | Notes |
|----------|-------|
| **Urgent** | Currently supported via `urgency` field |
| **Recurring** | Supportable via existing `schedules` table |
| **Expired/Closed** | Supportable via `status` field |

### Current Post Schema (key fields, updated 2026-03-17)

```
posts
├── id, title, description, description_markdown, summary
├── post_type: story | notice | exchange | event | spotlight | reference
├── weight: heavy | medium | light
├── priority (INT)
├── category (TEXT)
├── status: active | filled | rejected | expired | draft
├── submission_type: scraped | admin | org_submitted | revision
├── urgency: none | notice | urgent
├── location, latitude, longitude
├── source_url
├── revision_of_post_id, translation_of_id, duplicate_of_id
├── submitted_by_id → members
├── comments_container_id → containers
└── created_at, updated_at, published_at, deleted_at
```

Removed columns: `capacity_status` (migration 190), `scored_at` (migration 198), `embedding`/`relevance_score`/`relevance_breakdown` (migration 193), `pending_approval` status (migration 187).

### Related Tables

| Table | Purpose |
|-------|---------|
| post_contacts | Phone, email, website, address per post |
| post_locations | Geographic service locations (many-to-many) |
| post_sources | Origin tracking (which source a post came from) |
| service_areas | County/city/state coverage areas |
| schedules | Operating hours, recurring events (polymorphic) |
| taggables | Flexible tag associations (polymorphic join) |

### Schema Decision — Resolved

**Option A was chosen**: `post_type` column with CHECK constraint (6 values: story, notice, exchange, event, spotlight, reference). The old `post_type` tag kind and all its tags/taggables were deleted in migration 000173. Types are now column-based, not tag-based. See Phase 2 postmortem.

---

## The Broadsheet Concept

### What It Is

A digital newspaper front page. Auto-generated daily or weekly from Root Signal data + human editorial content.

### Layout

3-column composable layout. The design is 95% complete in a separate repo as static HTML/CSS/JS. Will be ported into the web-app.

### How It Works (Envisioned)

1. Root Signal delivers organized content on a cadence (daily or weekly)
2. The CMS creates a draft **edition** with auto-placed stories
3. The layout engine assigns stories to cells in the 3-column grid based on:
   - Story importance/urgency
   - Content type (volunteer opportunities go here, big issues go there)
   - Editorial priority overrides
4. The editor reviews in the CMS, tweaks placement, adds human stories
5. The editor publishes the edition
6. The web-app renders the broadsheet

### What's Been Built (updated 2026-03-17)

- ✅ **Edition model** — `editions`, `edition_rows`, `edition_slots` tables with county-scoped weekly editions
- ✅ **Layout engine** — weight-aware greedy heuristic places posts by priority into row templates
- ✅ **Editorial priority** — `weight` (heavy/medium/light) and `priority` (int) columns on posts
- ✅ **Cell/slot system** — 8 row templates with typed slots, 7 post templates for visual treatment
- ✅ **Auto-generation** — batch generate creates 87 county editions, layout engine fills slots
- ✅ **Broadsheet rendering** — 45 post renderers, full CSS design system, public API endpoint
- ⏳ **Email newsletter** — deferred to post-MVP (Amazon SES decided, not built)

---

## The CMS (admin-app) — Current State

### Existing Admin Pages

| Route | Purpose | Keep? |
|-------|---------|-------|
| `/admin/login` | OTP phone login | Yes |
| `/admin/dashboard` | Stats, pending approvals | Rethink — becomes edition dashboard |
| `/admin/posts` | List/approve/reject posts | Yes — core editorial function |
| `/admin/posts/[id]` | Post detail/edit | Yes — needs markdown editor |
| `/admin/organizations` | Org management | Yes |
| `/admin/organizations/[id]` | Org detail | Yes |
| `/admin/tags` | Taxonomy management | Yes |
| `/admin/sources` | Source management (crawl sources) | Remove — Root Signal concern |
| `/admin/sources/[id]` | Source detail | Remove |
| `/admin/sources/[id]/snapshots` | Crawl snapshots | Remove |
| `/admin/websites` | Website management (legacy) | Remove |
| `/admin/websites/[id]` | Website detail (legacy) | Remove |
| `/admin/jobs` | Job queue monitoring | Keep (simplified) |
| `/admin/proposals` | Dedup/sync proposals | Remove — Root Signal concern |
| `/admin/search-queries` | Discovery search queries | Remove — Root Signal concern |

### CMS Status (updated 2026-03-17)

1. ✅ **Edition manager** — Create, batch-generate, preview, publish/archive editions per county
2. ⏳ **Story editor** — Next priority. Plate.js WYSIWYG planned.
3. ⏳ **Signal inbox** — Blocked on Root Signal API contract + story editor
4. ✅ **Layout editor** — DnD broadsheet editor with section/row reordering, widget insertion
5. ✅ **Editorial dashboard** — Weekly cockpit with pipeline status cards and one-click generation
6. 🔜 **Email newsletter builder** — Deferred to post-MVP
7. 🔜 **Weather integration** — Deferred to post-MVP

---

## The Public Site (web-app) — Current State

### Existing Pages

| Route | Purpose |
|-------|---------|
| `/` | Homepage (search/browse posts) |
| `/about` | About page |
| `/contact` | Contact page |
| `/submit` | Submit resource link (public form) |
| `/posts` | Public posts list with filters |
| `/posts/[id]` | Post detail (read-only) |
| `/organizations` | Public org directory |
| `/organizations/[id]` | Org detail |

### What It Becomes

The **MN Together broadsheet** — a 3-column newspaper layout showing the current edition, with pages for individual stories, an archive of past editions, and community features (comments).

---

## Root Signal Integration

### What We Know

- Root Signal delivers organized, analyzed content
- Content maps to our post types (stories, requests, offers, events, etc.)
- Signal data can be directed to specific areas of the broadsheet (volunteer opportunities slot, big issues slot, etc.)
- Editorial tone can be applied ("write in this style")
- The CMS provides human-in-the-loop before anything goes public

### What We Don't Know Yet

- **API contract**: How does Root Signal expose data? REST? GraphQL? Webhook push?
- **Data format**: What shape does organized signal arrive in? Does it map cleanly to our post schema?
- **Cadence mechanism**: Does the CMS pull on a schedule, or does Signal push when ready?
- **Incremental updates**: Does Signal send a complete edition draft, or individual stories that we assemble?
- **Confidence/priority metadata**: Does Signal include its own ranking of what matters most?
- **Source attribution**: How do we trace a post back to its original source through Signal?

### What Needs to Happen

1. Tim does a technical dive on the Root Signal repo to understand its API surface
2. Define a data contract between Signal and Editorial
3. Build an ingestion endpoint in the CMS that receives/fetches Signal content
4. Map Signal output to CMS post types + editorial metadata

---

## Cleanup Plan — Status as of 2026-03-17

### Phase 1: Remove Dead Code ✅ COMPLETE

Removed 4 packages, 11 server domains, 5 ServerDeps fields, 45,947 lines. See [Phase 1 postmortem](../status/PHASE_1_DEAD_CODE_REMOVAL.md).

### Phase 2: Expand Post Types ✅ COMPLETE

Replaced 4-type system with 6 types (story/notice/exchange/event/spotlight/reference). Added 7 field group tables, weight/priority columns. Cleaned 5,233 lines of dead admin-app code. See [Phase 2 postmortem](../status/PHASE_2_POST_TYPES.md).

### Phase 3: Build Edition System ✅ COMPLETE

Built full edition/broadsheet system: 8 DB tables, layout engine, 16 HTTP handlers, GraphQL schema, admin pages. 87 MN counties seeded. See [Phase 3 postmortem](../status/PHASE_3_EDITION_SYSTEM.md).

### Phase 4: CMS UX + Broadsheet ✅ COMPLETE (frontend)

- ✅ Dashboard reworked as "edition cockpit"
- ✅ Sidebar restructured around editorial workflow
- ✅ Post scoring removed (Root Signal handles relevance)
- ✅ Kanban reworked (no Live column, deliberate publishing)
- ✅ Broadsheet design ported from prototype into web-app (45 renderers, 3,623 lines CSS)
- ✅ Widget system built (standalone domain, geo/temporal filtering, admin pages)
- ✅ Dead code cleanup: BusinessPost, scored_at, capacity_status, heat_map, memo_cache, agents, AI/embeddings removed
- ✅ Admin UI rebuilt on shadcn/Base UI components
- See [Phase 4 postmortem](../status/PHASE4_CMS_UX_REWORK.md) and [Broadsheet import postmortem](../status/BROADSHEET_DESIGN_IMPORT.md)

### What's Next

**Active work queue:**
1. **Story Editor** — Plate.js WYSIWYG for post creation/editing. Unblocks editorial workflow.
2. **Root Signal Integration** — API contract with Craig, ingestion endpoint, post mapping.
3. **Signal Inbox** — Triage UI for incoming Signal content. Depends on story editor + Signal integration.
4. **Integration Tests** — Test harness and API-edge tests per CLAUDE.md TDD rules. Project-wide gap.
5. **Broadsheet detail pages** — Post detail routes when clicking posts on the broadsheet.
6. **Specialty component registry** — 9 broadsheet components exist but aren't mapped to CMS templates.

**Deferred (post-MVP):**
- Abuse Reporting — backend stubs exist, needs DB migration + all UI
- Map Page — plan written, not started
- Email Newsletter — designed (Amazon SES), not started
- Weather Widgets — components ported, needs data source API
- Post status expansion (draft → pending → in_review → approved → active)
- Edition currency model (latest edition per county vs week-scoped)

---

## Open Questions

| # | Question | Owner | Status |
|---|----------|-------|--------|
| 1 | What is Root Signal's API? How do we fetch/receive organized content? | Tim (needs Craig input) | Blocked |
| 2 | What data format does Signal deliver? Does it map to our post schema? | Tim + Craig | Blocked |
| 3 | Post type expansion: field-based, tag-based, or hybrid? | Tim + Claude | **Resolved** — field-based. `post_type` column with CHECK constraint (6 values). Old tag-based `post_type` kind deleted. See Phase 2 postmortem. |
| 4 | Edition data model: what tables do we need? | Tim + Claude | **Resolved** — 8 tables: counties, zip_counties, row_template_configs, row_template_slots, post_template_configs, editions, edition_rows, edition_slots. See Phase 3 postmortem. |
| 5 | Layout engine: algorithmic placement or manual-only? | Tim | **Resolved** — algorithmic with manual override. Greedy heuristic places posts by weight/priority, editors adjust in admin UI. |
| 6 | Email newsletter: Amazon SES (decided — see [ARCHITECTURE_DECISIONS.md](ARCHITECTURE_DECISIONS.md) Decision 3) | Tim | Decided — **deferred**, not needed for MVP |
| 7 | Weather API: which provider? How granular? (county vs city) | Tim | **Deferred** — weather widget components ported but no data source. Punted to post-MVP. |
| 8 | Multi-tenancy: any architectural guardrails needed now? | Tim | Deferred (don't block on this, just don't make it impossible) |
| 9 | How much of the existing Restate workflow machinery stays for the CMS? | Tim + Claude | **Resolved** — Restate replaced by direct Axum HTTP handlers. All Restate handlers migrated to `api/routes/`. Restate SDK removed. |
| 10 | Should the broadsheet design repo be merged in, or remain separate? | Tim | **Resolved** — merged. Broadsheet design ported into `packages/web-app/` (see BROADSHEET_DESIGN_IMPORT.md). |

---

## Architecture — Current (updated 2026-03-17)

```
┌─────────────────────────────────────────────────────┐
│  admin-app (Next.js)  :3000                         │
│  The CMS — editors work here                        │
│  Edition management, story editor, signal inbox     │
├─────────────────────────────────────────────────────┤
│  web-app (Next.js)  :3001                           │
│  MN Together public site — readers see this         │
│  3-column broadsheet, individual stories, archive   │
├─────────────────────────────────────────────────────┤
│  shared (Node package)                              │
│  GraphQL schema, resolvers, shared types            │
├─────────────────────────────────────────────────────┤
│  server (Rust/Axum)  :9080                          │
│  Backend — auth, posts, orgs, tags, editions        │
│  HTTP/JSON API + SSE streams                        │
├─────────────────────────────────────────────────────┤
│  PostgreSQL + pgvector  :5432                       │
└─────────────────────────────────────────────────────┘
```

---

## Key Files Reference

| What | Where |
|------|-------|
| GraphQL schema (all types, queries, mutations) | `packages/shared/graphql/schema.ts` |
| GraphQL resolvers | `packages/shared/graphql/resolvers/` |
| Server client (GraphQL → Rust HTTP bridge) | `packages/shared/graphql/server-client.ts` |
| Rust post model | `packages/server/src/domains/posts/models/post.rs` |
| Rust post types/enums | `packages/server/src/domains/posts/` |
| Admin app pages | `packages/admin-app/app/admin/(app)/` |
| Public site pages | `packages/web-app/app/` |
| Design system docs | `packages/admin-app/DESIGN_SYSTEM.md` |
| Theme tokens (current) | `packages/admin-app/app/themes/reference.css` |
| Docker orchestration | `docker-compose.yml` |
| Environment template | `.env.example` |
| Database migrations | `packages/server/migrations/` (206 files) |
| Project coding rules | `CLAUDE.md` |

---

## Appendix: Existing Docs Worth Keeping

A docs audit was performed on 2026-02-24. ~60 dead files were deleted (crawling, extraction, seesaw, chat, volunteer matching). See `docs/DOCS_AUDIT.md` for the full triage.

### Still Relevant

| Doc | Why It Matters |
|-----|---------------|
| `architecture/DATABASE_SCHEMA.md` | Current schema reference |
| `architecture/DATA_MODEL.md` | Post entity relationships (needs minor edits) |
| `architecture/TAGS_VS_FIELDS.md` | Directly relevant to post type expansion decision |
| `architecture/DOMAIN_ARCHITECTURE.md` | Layered domain structure guide |
| `architecture/DESIGN_TOKENS.md` | Theme system reference |
| `security/AUTHENTICATION_GUIDE.md` | OTP + JWT auth reference |
| `RESTATE_MIGRATION_SUMMARY.md` | Current workflow architecture |
| `LOCAL_DEV_SETUP.md` | Getting the dev environment running |
| `INSTITUTIONAL_LEARNINGS.md` | Hard-won lessons from the project |

### Docs Edit Status (updated 2026-03-17)

| Doc | Status |
|-----|--------|
| `README.md` | ✅ Updated for Root Editorial |
| `INSTITUTIONAL_LEARNINGS.md` | ✅ Scrubbed for pivot (Seesaw/crawling removed) |
| `DOCKER_GUIDE.md` | Needs review — dead env vars may remain |
| `architecture/DATA_MODEL.md` | Needs review — may still reference volunteer/discovery |
| `architecture/RUST_IMPLEMENTATION.md` | ✅ Updated for CMS context |
| `architecture/DATABASE_SCHEMA.md` | Stale — references migration 171, schema now at 206 |
