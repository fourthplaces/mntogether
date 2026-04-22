# Data Model — Canonical

This document is the single source of truth for Root Editorial's current data model. When runtime code, migrations, and this doc disagree, **this doc wins for design intent and runtime code wins for implementation**. If the two imply different answers, one of them has drifted and needs to be reconciled — raise it, don't pick.

**Maintenance rule.** Update this doc whenever a design decision changes what a domain object is, what fields it carries, or how it relates to others. A migration that only adds a column doesn't necessarily need a doc update; a migration that changes a kind or a relationship does. Stale entries here are bugs.

**Pivot context.** See [`CLAUDE.md` §Reading This Codebase](../../CLAUDE.md) for the pre/post-pivot framing. Everything below describes the post-pivot system.

---

## 1. System shape

Root Editorial is a curated CMS that consumes fully-formed posts from Root Signal (separate repo, upstream producer) and publishes them as weekly per-county broadsheets. Editorial does not scout, extract, or score trust — those are upstream concerns. Human editors review, sequence, and apply verified-badge decisions.

```
Root Signal  →  POST /Posts/create_post  →  posts table
                 (ingest envelope)            │
                                              ├─ 1:1 field groups: post_meta, post_datetime, post_status,
                                              │                     post_link, post_person
                                              ├─ 0..N field groups: post_items, post_schedule, post_media,
                                              │                     post_sources, contacts (polymorphic)
                                              ├─ taggables → tags (topic | service_area | safety)
                                              └─ post_sources → sources → organizations / source_individuals
```

---

## 2. Posts

The central content unit. One post per publishable item. The submission contract is specified in [`docs/handoff-root-signal/ROOT_SIGNAL_API_REQUEST.md`](../handoff-root-signal/ROOT_SIGNAL_API_REQUEST.md); this section documents the resulting persisted shape.

### 2.1 Post types (9)

`post_type` is a CHECK-constrained TEXT column on `posts`. Nine values:

| `post_type` | Purpose |
|---|---|
| `story` | Feature reporting, narrative. |
| `update` | "News you should know" — reader needs a next step. |
| `action` | Deadline-driven civic action; `link.deadline` required. |
| `event` | Community gathering; `datetime` + `location` required. |
| `need` | Community ask for help / volunteers / donations. |
| `aid` | Community offer: food shelf, free clinic, tool library. |
| `person` | Profile / spotlight. |
| `business` | Independent local business listing. `is_evergreen` defaults true. |
| `reference` | Resource directory. The item list IS the post. `is_evergreen` defaults true. |

Per-type required field groups live in [`docs/architecture/POST_TYPE_SYSTEM.md`](POST_TYPE_SYSTEM.md) and are authoritative for validation.

### 2.2 Post fields on the `posts` table

Grouped by concern. This is the post row; field groups are separate tables (§3).

**Identity.** `id UUID`, `title TEXT`, `post_type TEXT`, `weight TEXT` (`heavy|medium|light`), `priority INT` (0–100).

**Body.** `body_raw TEXT` (≥250 chars, always), `body_heavy TEXT?`, `body_medium TEXT?`, `body_light TEXT`, `body_ast JSONB?` (Plate.js). Signal produces all weight-appropriate tiers; layout engine selects template at slot-fill time.

**Timeline.** `published_at TIMESTAMPTZ` (source publication time, not ingest time), `is_evergreen BOOLEAN`, `created_at`, `updated_at`.

**Location.** `location TEXT?` (human-readable), `zip_code TEXT?`, `latitude`/`longitude DOUBLE PRECISION?`.

**Lifecycle / trace.** `status TEXT` (`active | in_review | filled | expired | archived | rejected`), `submission_type TEXT` (`ingested | admin | org_submitted | reader_submitted | revision`), `is_urgent BOOLEAN`, `pencil_mark TEXT?` (`star | heart | smile | circle | null`), `extraction_confidence INT?` (0–100).

**Graph.** `revision_of_post_id UUID?`, `translation_of_id UUID?`, `duplicate_of_id UUID?`, `is_seed BOOLEAN`, `deleted_at TIMESTAMPTZ?`, `deleted_reason TEXT?`.

**Derived / system.** `search_vector TSVECTOR` (trigger-maintained). No `embedding` column — the pgvector work was dropped in migration 000193 when the AI pipeline moved upstream to Root Signal. See [`EMBEDDING_FEATURES_REFERENCE.md`](EMBEDDING_FEATURES_REFERENCE.md) for the historical catalogue if semantic search gets reintroduced.

**Source language.** `source_language TEXT` (ISO 639-1, default `en`).

### 2.3 Editor-only fields

These are set by editors post-ingest; ingest rejects submissions that set them: `status`, `is_urgent`, `pencil_mark`, `deleted_at`/`deleted_reason`, edition-slot placement.

---

## 3. Field groups

Structured supplements to a post. Each maps to its own table; cardinality is per-table.

| Group | Table | Cardinality | Purpose |
|---|---|---|---|
| Meta | `post_meta` | 1:1 | `kicker`, `byline`, `deck`, `pull_quote`, `timestamp`, `updated` — editorial presentation. |
| Datetime | `post_datetime` | 1:1 | `start_at`, `end_at`, `cost`, `recurring` — required for `event`. |
| Schedule | `post_schedule` | 0..N | `day`, `opens`, `closes`, `sort_order` — operating hours, volunteer windows. |
| Items | `post_items` | 0..N | `name`, `detail`, `sort_order` — directory entries, need/aid lists. |
| Person | `post_person` | 1:1 | `name`, `role`, `bio`, `quote`, `photo_url`, `photo_media_id` — required for `person`. |
| Link | `post_link` | 1:1 | `label`, `url`, `deadline` — CTA; required for `action`. |
| Media | `post_media` | 0..N | `image_url`, `media_id`, `caption`, `credit`, `alt_text`, `sort_order`. Field groups unified with library uploads via polymorphic `media_references`. |
| Status | `post_status` | 1:1 | `state` (`open | closed`), `verified` — required for `need`, `aid`. |
| Contacts | `contacts` (polymorphic) | 0..N | `contactable_type = 'post'`, `contact_type` ∈ {phone, email, website, address, booking_url, social}. |
| Source attribution | `post_source_attribution` | 1:1 | `source_name`, `attribution` — the human-readable credit line shown on the post. |

---

## 4. Sources and citations

Posts link to sources through a polymorphic graph, not a direct FK.

```
post  →  post_sources  →  sources  →  organizations
                                   ↘  source_individuals
```

### 4.1 `post_sources`

0..N rows per post (a post can cite multiple sources). Each row: `source_type` (website | instagram | facebook | x | ...), `source_id` (polymorphic target), `source_url`, `first_seen_at`, `last_seen_at`, `disappeared_at`.

After [Addendum 01](../handoff-root-signal/ADDENDUM_01_CITATIONS_AND_SOURCE_METADATA.md) lands, this table additionally carries `content_hash`, `snippet`, `confidence`, `platform_id`, `platform_post_type_hint` per citation.

### 4.2 `sources`

Parent table for websites and social profiles attached to organisations. One row per distinct source (many-to-many with orgs via FK on the source row).

### 4.3 `organizations`

Organisation identity. Fields Editorial stores: `name`, `website`, `phone`, `email` (via contacts), `primary_address`, `latitude`, `longitude`, social handles, `verified` badge (applied by editors), `logo_media_id` / `logo_url`, `status` (`pending | approved | active | inactive`).

**Editorial does not do trust scoring.** Organisations are not ranked by age, size, or signal strength — those decisions happen upstream in Root Signal or at editor review. The `verified` column is a single badge editors apply after reviewing an org; nothing auto-scores.

### 4.4 `source_individuals`

Individuals cited as sources (not subjects — that's `post_person`). Fields: `display_name`, `handle`, `platform` (enum), `platform_url`, `verified_identity`, `consent_to_publish`, `consent_source`, `consent_captured_at`. Dedup ladder: `(platform, handle)` → `platform_url` → insert.

### 4.5 `post_source_attribution` vs `post_sources`

Two tables with intentional distinct roles:

- `post_sources` — raw provenance (every URL, with metadata). 0..N per post. Admin-visible.
- `post_source_attribution` — the single rendered credit line shown to public readers. 1:1 per post. Derived from the primary `post_sources` row.

---

## 5. Tags

Three tag kinds. Polymorphic `taggables` join attaches any tag to any entity.

| `kind` | Vocabulary | Attached to | Purpose |
|---|---|---|---|
| `topic` | Open. Current seed list in `data/tags.json`. Expected to grow — propose new slugs freely; unknown slugs auto-create and flag `in_review`. | post, organization | Topic-grouping, search, discovery. ≥1 required per post. |
| `service_area` | Closed. 87 MN counties + `statewide`. | post, organization | Geographic eligibility for county editions. ≥1 required per post. Unknown slugs hard-fail. |
| `safety` | Reserved. Access-policy modifiers (e.g. `no-id-required`, `ice-safe`, `sliding-scale`, `confidential`, `harm-reduction`). Full list in [handoff `TAG_VOCABULARY.md` §3](../handoff-root-signal/TAG_VOCABULARY.md). | post, organization | Flags that remove hesitation to seek a service. Optional. Unknown slugs hard-fail. |

### 5.1 Dead tag kinds

Pre-pivot migrations introduced and then abandoned: `reserved`, `structure`, `audience_role`, `population`, `post_type` (as a tag kind), `community_served`, `service_offered`, `org_leadership`, `business_model`, `certification`, `ownership`, `worker_structure`, `listing_type`, `provider_category`, `provider_specialty`, `with_agent`, `county`, `city`, `language`, `verification`.

Migration `000237` removed their rows from `tag_kinds` and any remaining rows from `tags`, then added a CHECK on `tags.kind` so new writes can only use the canonical kinds below.

### 5.2 Tag reservation: `neighborhood`

Reserved for future use. Present-day geographic sub-county granularity (e.g., "North Minneapolis", "Phillips", "Lake Street") lives in `location` text fields, not tags. If we formalise neighborhoods as tags, the kind will be `neighborhood`. The `tag_kinds` row exists (seeded by migration `000237`) so the CHECK constraint accepts it when we turn it on; no `tags` rows yet.

### 5.3 `platform` — lookup kind, not a tag kind

The `platform` kind is preserved as a read-only lookup table used by the `organization_links` picker (display name, color, emoji per platform — instagram, facebook, substack, etc.). **It is not a tag kind in the design sense:** nothing attaches `platform` tags to posts or orgs. Migration 232 moved platform presence to the first-class `organization_links` table; the 46 `tags` rows where `kind = 'platform'` stayed as a UI lookup and the `tag_kinds.allowed_resource_types` was cleared to reflect that. The CHECK constraint on `tags.kind` includes `'platform'` for that reason.

If the picker is ever rewritten to read from a dedicated `platforms` table or a static config, the `platform` kind can go away and the CHECK constraint should be tightened.

---

## 6. Geography

### 6.1 Counties

87 Minnesota counties (real) + 1 pseudo-county (`Statewide`). `counties` table: `fips_code` (PRIMARY identifier), `name`, `state`, `latitude`, `longitude`, `is_pseudo BOOLEAN`, `target_content_weight INT`.

**Statewide** is a real row in `counties` with `fips_code = 'statewide'` and `is_pseudo = true`. The layout engine branches on `is_pseudo`: real counties pull county-tagged + statewide-tagged posts; Statewide pulls only explicitly `service_area = 'statewide'` posts.

### 6.2 `zip_counties`

Many-to-many: one ZIP can span multiple counties. Used to infer county from a post's `zip_code` when no `service_area` tag is set.

---

## 7. Editions and layout

Editions are weekly per-county broadsheet compositions.

```
editions (id, county_id, period_start, status, …)
  ├── edition_slots (edition_id, row_index, slot_index, post_id | widget_id, template, …)
  └── row_templates (variant configurations for layout)
```

`edition_slots` is polymorphic — a slot holds either a `post_id` or a `widget_id`. Layout engine fills slots by matching posts to row templates by weight + post_type compatibility.

Edition lifecycle: `draft → in_review → approved → published → archived`. Lifecycle gates prevent 0-slot editions from advancing past `draft`.

---

## 8. Widgets

Standalone broadsheet items that aren't posts — editorial items placed by the layout editor. Types: `number`, `pull_quote`, `resource_bar`, `weather`, `section_sep`, `photo`. Widgets have their own geo/temporal targeting (county + period) and can appear alongside posts in edition slots.

---

## 9. Members and auth

Minimalist privacy-conscious member model.

```
members (id, searchable_text, latitude, longitude, location_name, active, …)
identifiers (id, member_id, phone_hash, is_admin)  -- SHA-256 hashes only; no plaintext
```

Auth flow: phone / email → Twilio Verify OTP → JWT tied to `identifiers.id`. `ADMIN_IDENTIFIERS` env var lists admin phone/email hashes; the `AdminUser` extractor gates admin routes.

A new `ServiceClient` extractor (tracked in [TODO §1.1](../TODO.md)) authenticates Root Signal's machine-token Bearer for the ingest endpoint.

---

## 10. Notes (editorial)

```
notes (id, noteable_type, noteable_id, body, severity, is_public, source_url, cta_text, expired_at, created_by, created_at)
```

Polymorphic — attaches to posts, organisations, or any other entity. `urgent_notes` on a post are a GraphQL projection of `notes where severity='urgent' and noteable_type='post'`.

---

## 11. What Editorial does NOT store

Explicit non-scope, to prevent pre-pivot confusion returning:

- **Trust scoring** on organisations or sources. No `year_founded`, `employee_count`, reputation score, or credibility rank. Editors apply a single `verified` badge per org; that's it.
- **Raw crawl state.** No `last_crawled_at`, scrape metadata, or extraction pipeline state. Root Signal owns all of that.
- **AI-generated summary fields** on posts. No `custom_tldr`, `custom_description`, `custom_title`. Root Signal's extraction produces body tiers directly; no downstream AI rewriting.
- **Capacity, heat-map, or signal-strength metrics.** Deleted in migrations 190–192.
- **Audience-role / population-served taxonomy.** People aren't single-bucket identities; no fixed vocabulary captures them.

Anything from the pre-pivot era that describes Editorial as scoring, ranking, crawling, or auto-summarising content is stale. See [`CLAUDE.md` §Reading This Codebase](../../CLAUDE.md).

---

## 12. Where to look for details

- SQL-level schema: generated from migrations in `packages/server/migrations/`. See the `Post` Rust model at `packages/server/src/domains/posts/models/post.rs` for a compact view of the final posts-table shape.
- Post-type field-group requirements: [`POST_TYPE_SYSTEM.md`](POST_TYPE_SYSTEM.md).
- Tag vocabulary: [handoff `TAG_VOCABULARY.md`](../handoff-root-signal/TAG_VOCABULARY.md).
- Ingest contract: [handoff `ROOT_SIGNAL_API_REQUEST.md`](../handoff-root-signal/ROOT_SIGNAL_API_REQUEST.md) + [`ROOT_SIGNAL_DATA_CONTRACT.md`](ROOT_SIGNAL_DATA_CONTRACT.md).
- Addenda: [Addendum 01 — Citations and source metadata](../handoff-root-signal/ADDENDUM_01_CITATIONS_AND_SOURCE_METADATA.md).
- Edition lifecycle: [`EDITION_STATUS_MODEL.md`](EDITION_STATUS_MODEL.md) + [`POST_EDITION_LIFECYCLE.md`](../guides/POST_EDITION_LIFECYCLE.md).
- Outstanding scrub work: [`POST_PIVOT_SCRUB.md`](../status/POST_PIVOT_SCRUB.md).
