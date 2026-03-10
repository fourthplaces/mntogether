# Root Editorial Pivot

**Date:** 2026-02-24
**Author:** Tim (design/editorial lead) + Claude
**Status:** Living document — update as decisions are made

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
├── ai-client/          ← ALIVE: Provider-agnostic AI (OpenAI + OpenRouter)
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
| **chatrooms** | Real-time chat/comments → not needed for CMS |
| **contacts** | Generic contact management → minimal/legacy |
| **schedules** | May be revived for events, currently over-engineered |

### External Services — What Stays, What Goes

#### STAYS

| Service | Purpose |
|---------|---------|
| PostgreSQL + pgvector | Database + semantic search |
| Restate | Durable workflow execution |
| Twilio | SMS OTP authentication |
| OpenAI / OpenRouter | Embeddings, summaries, editorial AI |

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

### Current Post Schema (key fields)

```
posts
├── id, title, description, description_markdown, summary
├── post_type (TEXT) ← needs expansion
├── category (TEXT)
├── status: pending_approval | active | filled | rejected | expired | draft
├── submission_type: scraped | admin | org_submitted | revision
├── capacity_status: accepting | paused | at_capacity
├── urgency: low | medium | high | urgent
├── location, latitude, longitude
├── source_url
├── embedding (pgvector 1024d)
├── relevance_score, relevance_breakdown
├── revision_of_post_id, translation_of_id, duplicate_of_id
├── submitted_by_id → members
├── comments_container_id → containers
└── created_at, updated_at, published_at, deleted_at
```

### Related Tables

| Table | Purpose |
|-------|---------|
| post_contacts | Phone, email, website, address per post |
| post_locations | Geographic service locations (many-to-many) |
| post_sources | Origin tracking (which source a post came from) |
| service_areas | County/city/state coverage areas |
| schedules | Operating hours, recurring events (polymorphic) |
| taggables | Flexible tag associations (polymorphic join) |
| service_listings | Service-specific properties (legacy) |
| opportunity_listings | Volunteer/donation specifics (legacy) |
| business_listings | Business-specific fields (legacy) |

### Schema Decision Needed

The expanded type system needs a migration strategy:

- **Option A:** Expand the `post_type` CHECK constraint to include all new values
- **Option B:** Use the existing tag system exclusively (tag kind = `post_type`)
- **Option C:** Hybrid — core type in `post_type` field, sub-types via tags

The tag system already has a `post_type` tag kind with values `seeking`, `offering`, `announcement`. This overlaps with the field-level `post_type`. **This needs to be reconciled before building the CMS.**

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

### What Doesn't Exist Yet

- **Edition model** — no concept of a "weekly issue" or "daily edition" in the database
- **Layout engine** — no placement logic
- **Editorial priority** — no per-edition story ordering or positioning
- **Cell/slot system** — no concept of "this area is for volunteer opportunities"
- **Auto-generation** — no workflow that receives signal data and builds a draft edition
- **Email newsletter** — no system for generating a shorter email version of the edition

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

### What the CMS Needs to Become

1. **Edition manager** — Create, preview, and publish daily/weekly broadsheet editions
2. **Story editor** — Rich markdown editor for human-authored content
3. **Signal inbox** — Review incoming Root Signal content, approve/edit/reject
4. **Layout editor** — Visual control over story placement in the 3-column grid
5. **Editorial dashboard** — "Here's this week's edition, 95% ready, here's what needs your attention"
6. **Email newsletter builder** — Generate a shorter email version of the edition
7. **Weather integration** — Configure weather widget per county/city

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

## Cleanup Plan

### Phase 1: Remove Dead Code

**Packages to remove:**
- `packages/search-app/` — dead stub
- `packages/extraction/` — Root Signal concern
- `packages/openai-client/` — superseded by ai-client
- `packages/apify-client/` — Root Signal concern
- `packages/web/` — superseded by admin-app + web-app

**Server domains to remove:**
- `crawling`, `extraction`, `website`, `social_profile`, `sync`, `curator`, `newsletter`, `providers`

**Config cleanup:**
- Remove Tavily, Firecrawl, Apify, Expo, Voyage AI, NATS from `.env.example`
- Update `Cargo.toml` workspace members
- Update `package.json` workspaces
- Simplify `docker-compose.yml` (remove search-app, NATS services)

**GraphQL cleanup:**
- Remove resolvers for dead domains (source crawl mutations, website mutations, sync/proposals, search-queries)
- Remove dead queries from schema
- Keep: posts, organizations, tags, notes, jobs, auth

### Phase 2: Expand Post Types

1. Reconcile `post_type` field vs `post_type` tag kind (pick one source of truth)
2. Migrate to the expanded type system (Story, Request/*, Offer/*, Event, etc.)
3. Add modifier support (urgent, recurring, expired) — likely via tags or flags
4. Update GraphQL schema enums
5. Update Rust models
6. Update admin-app forms and filters

### Phase 3: Build Edition System (New)

1. Design edition data model (edition → edition_slots → posts)
2. Build edition CRUD in backend
3. Build auto-generation workflow (receive Signal → create draft edition → place stories)
4. Build edition preview in CMS
5. Build layout editor (drag-and-drop or priority-based)
6. Build publish workflow (edition → live broadsheet)

### Phase 4: CMS UX Overhaul

1. Redesign dashboard as "edition cockpit" (this week's edition, what needs attention)
2. Add markdown story editor with rich formatting
3. Build Signal inbox (incoming content review queue)
4. Simplify navigation (remove Root Signal admin pages)
5. Add email newsletter generation from published edition

### Phase 5: Broadsheet Layout

1. Port static 3-column design from separate repo into web-app
2. Build layout engine (slot system, story placement rules)
3. Connect edition data to layout rendering
4. Add weather widget integration
5. Build edition archive (past issues)

---

## Open Questions

| # | Question | Owner | Status |
|---|----------|-------|--------|
| 1 | What is Root Signal's API? How do we fetch/receive organized content? | Tim (needs Craig input) | Blocked |
| 2 | What data format does Signal deliver? Does it map to our post schema? | Tim + Craig | Blocked |
| 3 | Post type expansion: field-based, tag-based, or hybrid? | Tim + Claude | Needs decision |
| 4 | Edition data model: what tables do we need? | Tim + Claude | Needs design |
| 5 | Layout engine: algorithmic placement or manual-only? | Tim | Needs decision |
| 6 | Email newsletter: Amazon SES (decided — see [ARCHITECTURE_DECISIONS.md](ARCHITECTURE_DECISIONS.md) Decision 3) | Tim | Decided |
| 7 | Weather API: which provider? How granular? (county vs city) | Tim | Needs decision |
| 8 | Multi-tenancy: any architectural guardrails needed now? | Tim | Deferred (don't block on this, just don't make it impossible) |
| 9 | How much of the existing Restate workflow machinery stays for the CMS? | Tim + Claude | Needs audit |
| 10 | Should the broadsheet design repo be merged in, or remain separate? | Tim | Needs decision |

---

## Architecture — What Stays

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
│  server (Rust + Restate)  :9080                     │
│  Backend — auth, posts, orgs, tags, editions        │
│  GraphQL resolvers call into Restate services       │
├─────────────────────────────────────────────────────┤
│  PostgreSQL + pgvector  :5432                       │
│  Restate :8180                                      │
└─────────────────────────────────────────────────────┘
```

---

## Key Files Reference

| What | Where |
|------|-------|
| GraphQL schema (all types, queries, mutations) | `packages/shared/graphql/schema.ts` |
| GraphQL resolvers | `packages/shared/graphql/resolvers/` |
| Restate client (GraphQL → Restate bridge) | `packages/shared/graphql/restate-client.ts` |
| Rust post model | `packages/server/src/domains/posts/models/post.rs` |
| Rust post types/enums | `packages/server/src/domains/posts/` |
| Admin app pages | `packages/admin-app/app/admin/(app)/` |
| Public site pages | `packages/web-app/app/` |
| Design system docs | `packages/admin-app/DESIGN_SYSTEM.md` |
| Theme tokens (current) | `packages/admin-app/app/themes/reference.css` |
| Docker orchestration | `docker-compose.yml` |
| Environment template | `.env.example` |
| Database migrations | `packages/server/migrations/` (163 files) |
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

### Needs Edits (5 priority files)

| Doc | What Needs Updating |
|-----|---------------------|
| `README.md` | Rename from "Minnesota Digital Aid", add pivot context |
| `INSTITUTIONAL_LEARNINGS.md` | Remove Seesaw/crawling sections |
| `DOCKER_GUIDE.md` | Remove dead env vars, update ports |
| `architecture/DATA_MODEL.md` | Remove volunteer/discovery sections |
| `architecture/RUST_IMPLEMENTATION.md` | Update core philosophy for CMS context |
