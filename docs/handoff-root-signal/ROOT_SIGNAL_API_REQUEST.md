# Root Signal → Root Editorial: Integration Specification

**Authoritative reference:** [`docs/architecture/ROOT_SIGNAL_DATA_CONTRACT.md`](../architecture/ROOT_SIGNAL_DATA_CONTRACT.md) (the on-the-wire contract lives there; this document is the spec package + integration brief that wraps it).

---

## Table of contents

1. [Overview](#1-overview)
2. [System context](#2-system-context)
3. [Integration shape](#3-integration-shape)
4. [Submission envelope](#4-submission-envelope)
5. [Full field contract](#5-full-field-contract)
6. [The 9 post types and their required fields](#6-the-9-post-types-and-their-required-fields)
7. [Source model](#7-source-model)
8. [Geography and service area](#8-geography-and-service-area)
9. [Media](#9-media)
10. [Tags and categorisation](#10-tags-and-categorisation)
11. [Validation, errors, and the 201/422 protocol](#11-validation-errors-and-the-201422-protocol)
12. [Revisions, duplicates, idempotency](#12-revisions-duplicates-idempotency)
13. [Lifecycle and status](#13-lifecycle-and-status)
14. [Auth and transport](#14-auth-and-transport)
15. [Mapping — Root Signal model → Editorial post](#15-mapping--root-signal-model--editorial-post)
16. [Complete worked examples](#16-complete-worked-examples)
17. [Glossary](#17-glossary)
18. [References](#18-references)

---

## 1. Overview

### 1.1 What this integration does

Root Signal delivers fully-formed civic posts into Root Editorial's CMS. Some interpretation of this specification will be implemented; the shape may evolve in the details but the contract described here is what Editorial accepts.

### 1.2 What "fully formed" means

A post Editorial receives is *publishable as-is*. It carries:

- A title and three body tiers (`body_heavy` / `body_medium` / `body_light`) plus a `body_raw` full detail page, sized to the targets in §5.
- A `post_type` from the 9-type taxonomy, a `weight`, and a `priority`.
- A resolved source (organisation, individual, or editorial) with canonical URL and attribution line.
- ≥1 topic tag, ≥1 service-area tag (one of 87 MN counties or `statewide`).
- All post-type-specific field groups (`datetime`, `items`, `contacts`, `link`, `person`, `schedule`, `status`, `media`) populated per §6.
- An `extraction_confidence` score so Editorial can gate low-confidence posts for human review.

This is a higher bar than "extracted a signal"; it is **"packaged for publication, with an editor still in the loop for curation."** Editors review, select, and sequence — they do not routinely rewrite.

### 1.3 What Editorial owns vs what Root Signal owns

| Concern | Owner | Notes |
|---|---|---|
| Source discovery, scraping, extraction | **Root Signal** | — |
| Per-post-type shape conformance (body tiers, field groups, tags, source) | **Root Signal** | The mapping layer lives on Root Signal's side — see §15. |
| 3-body-tier authoring (`heavy`/`medium`/`light`) | **Root Signal** | Templates swap at layout time; which slot a post occupies is decided at render, not submission. |
| Dedup against Root Signal's own corpus | **Root Signal** | Sets `duplicate_of_id` when confident. |
| Dedup against Editorial's corpus (content hash, idempotency keys, revision chain) | **Editorial** | Submitters do not need to think about this beyond correct idempotency key minting. |
| Editor review, curation, slotting, publication | **Editorial** | Root Signal never slots posts into editions or flips them to `published`. |
| Broadsheet rendering, public site, newsletters | **Editorial** | — |

### 1.4 Integration shape

Root Signal pushes fully-formed post envelopes to Editorial's `POST /Posts/create_post` endpoint over HTTPS, with Bearer-token auth and `X-Idempotency-Key` headers. Editorial responds 201 with the assigned `post_id` or 422 with a structured per-field error list. §3 covers the transport.

Sections 4–15 are the contract. §16 is a complete set of worked examples Root Signal engineering can use as reference submissions.

---

## 2. System context

### 2.1 Root Editorial (us)

Root Editorial is the human-curated CMS and public-facing broadsheet for Minnesota civic content. The stack:

| Service | Tech | Port | Role |
|---|---|---|---|
| Admin app | Next.js 16 | 3000 | Editor-facing CMS. Post review, edition assembly, publish. |
| Public site | Next.js 16 | 3001 | Reader-facing broadsheet (county-scoped + Statewide). |
| Server | Rust / Axum | 9080 | HTTP/JSON API + SSE streams. Business logic in pure-function activities. |
| Database | PostgreSQL 17 | 5432 | 236 migrations applied on `main`. Schema documented at `docs/architecture/DATABASE_SCHEMA.md`. |

**Schema and ingest infrastructure:** complete for everything specified in this document. Posts table carries the 9-type `post_type` enum, 3-tier body columns, `extraction_confidence`, revisions, embeddings, Statewide pseudo-county support. Field-group tables (`post_meta`, `post_datetime`, `post_schedule`, `post_items`, `post_person`, `post_link`, `post_status`, `post_media`, `post_sources`, polymorphic `contacts`, `source_individuals`) are all present. Ingest endpoint accepts the full envelope with structured 422 validation, `ServiceClient` Bearer auth, `X-Idempotency-Key` handling, organisation and individual source dedup, editor-only field rejection, tag resolution across all kinds, and server-side media processing.

### 2.2 Root Signal (you)

This section captures Editorial's working understanding of Root Signal's system, informed by reading `dev` branch at `7ffd18e0`. The specification downstream does not depend on this being perfectly correct — flag anything misunderstood.

**Core vocabulary** (from `modules/rootsignal-common/src/types.rs` and `docs/architecture/`):

| Term | What it is |
|---|---|
| **Signal** | A single extracted unit (event, resource, help request, announcement, concern, condition). Carries `NodeMeta` (title, summary, sensitivity, confidence, corroboration_count, locations, url, published_at, source_diversity, cause_heat, category, mentioned_entities). |
| **ActorNode** | Organisation, individual, government body, or coalition. Dedup'd by `canonical_key`. Rich fields: domains, social_urls, bio, location, typical_roles. |
| **Citation / Evidence** | Source URL + `content_hash` + `retrieved_at` + snippet. The audit trail per signal. |
| **Situation** | Cluster container — "root cause + affected population + place." Has `arc` (Emerging → Developing → Active → Cooling → Cold), `temperature`, `headline`, `lede`, `briefing_body` (LLM-synthesised markdown with inline `[signal:UUID]` citations). |
| **Dispatch** | Atomic narrative update within a situation's thread. Types: Update, Emergence, Split, Merge, Reactivation, Correction. |
| **Brief** | The rich, annotated `briefing_body` attached to a situation. |
| **Scout run** | Orchestrated flow — Bootstrap, Scrape, Weave, ScoutSource. Manual trigger today. |
| **Schedule** (signal schedule) | RRULE + rdates + exdates + dtstart/dtend + timezone. For recurring gatherings. |
| **Region** | Center lat/lng + radius_km + geo_terms; default Twin Cities. |

**Signal types today** (6): `Gathering`, `Resource`, `HelpRequest`, `Announcement`, `Concern`, `Condition`.
**Proposed additions** (per `docs/editorial-surface-area-request.md`): `Profile`, `LocalBusiness`, `Opportunity`/`CivicAction`, `Job`.

**API shape:** Axum + async-graphql. Public read queries (`signals_near`, `signals_in_bounds`, `situations`, `situation(id)`, `actors_in_bounds`, `tags`, semantic search). Admin mutations (review, correct, delete_by_source_url, run_scout). WebSocket subscriptions exist. **No outbound HTTP client or webhook emitter today.**

**Freshness policy:** Gatherings expire 30 days after end; Resources/HelpRequests 60 days; Announcements 90 days; Concerns persistent. Anything > 365 days old at extraction is dropped.

### 2.3 Integration goal

Signal produces packaged posts that are complete under the contract in §4–§14. Editorial ingests them with idempotency, validates them, stores them at `status = active` (or `in_review` for soft failures), and lets editors curate into weekly per-county editions. The underlying signal graph stays on your side. We consume the finished product, not the raw signal stream.

---

## 3. Integration shape

Root Signal pushes fully-formed post envelopes to Editorial's HTTP ingest endpoint. One post per call. Bearer-token auth. Idempotent under `X-Idempotency-Key`.

```
Root Signal                              Editorial
─────                                    ─────────
scout.weave() →
  situation materialises →
    mapper: signal → post envelope →    POST /Posts/create_post
                                         Authorization: Bearer <signal-api-key>
                                         X-Idempotency-Key: <uuid>
                                         Content-Type: application/json

                                        validate → org dedup → insert → field groups →
                                        tag resolution → media reference → status='active'

    ← 201 Created                       { "post_id": "…", "status": "active" }
    or 422 Unprocessable Entity         { "errors": [ { "field": "body_raw", "code": "below_min_length", ... } ] }
```

Push was chosen over pull for three reasons. Mapping judgement — which signals become which `post_type`, how `briefing_body` maps to `body_heavy` vs `body_medium`, which dispatches become revisions — lives closer to the source data on Root Signal's side; duplicating it in Editorial creates drift. 422 responses surface validation failures synchronously, so problems are visible at the submission site rather than discovered later. And `revision_of_post_id`, `duplicate_of_id`, and `X-Idempotency-Key` map cleanly onto per-call semantics.

### 3.1 Transport parameters

| | |
|---|---|
| Protocol | HTTPS, TLS 1.2+ |
| Host | Issued with the API key at kickoff |
| Path | `POST /Posts/create_post` (capital-P convention per Editorial's `/{Service}/{handler}` style) |
| Content-Type | `application/json; charset=utf-8` |
| Auth | `Authorization: Bearer <signal-api-key>` — §14 |
| Idempotency | `X-Idempotency-Key: <uuid>` — §12 |
| Request-ID | `X-Request-ID: <uuid>` — echoed on response headers and logs |
| Max body size | 1 MiB per post. Media are URL references, not binary uploads. |
| Rate limits | 15 req/sec sustained, 50 req/sec burst, per API key (§14.3) |
| Retries (Root Signal side) | Exponential backoff on 429 and 5xx. No retry on any other 4xx. |

---

## 4. Submission envelope

One post per HTTP call. The envelope (full, canonical, reproduced from `docs/architecture/ROOT_SIGNAL_DATA_CONTRACT.md:19-92`):

```json
{
  "title": "Highway 169 Bridge Deck Work Scheduled",
  "post_type": "update",
  "weight": "light",
  "priority": 50,
  "body_raw": "<full body text, ≥250 chars, see §5.2>",
  "body_heavy": null,
  "body_medium": null,
  "body_light": "<always required, 40–120 chars>",
  "body_ast": null,
  "published_at": "2026-04-02T14:00:00-05:00",
  "source_language": "en",
  "is_evergreen": false,
  "location": "Aitkin, MN",
  "zip_code": "56431",
  "latitude": 46.5327,
  "longitude": -93.7105,
  "tags": {
    "service_area": ["aitkin-county"],
    "topic": ["transit"],
    "safety": []
  },
  "source": {
    "kind": "organization",
    "organization": {
      "name": "Minnesota Department of Transportation",
      "website": "https://www.dot.state.mn.us/",
      "instagram_handle": null,
      "twitter_handle": "mndotnews",
      "facebook_handle": "MnDOT",
      "address": null,
      "phone": null,
      "populations_served": null,
      "already_known_org_id": null
    },
    "individual": null,
    "source_url": "https://www.dot.state.mn.us/d1/projects/hwy169bridge/",
    "attribution_line": "MnDOT District 1 press release",
    "extraction_confidence": 88
  },
  "meta": {
    "kicker": "Roads",
    "byline": "MnDOT District 1",
    "deck": null,
    "pull_quote": null,
    "timestamp": null,
    "updated": null
  },
  "field_groups": {
    "datetime": {
      "start_at": "2026-04-03T06:00:00-05:00",
      "end_at": "2026-04-05T18:00:00-05:00",
      "cost": null,
      "recurring": false
    },
    "schedule": [],
    "person": null,
    "items": [],
    "contacts": [
      { "contact_type": "phone", "contact_value": "511", "contact_label": "MN 511 traveler info" },
      { "contact_type": "website", "contact_value": "https://511mn.org", "contact_label": "Live road conditions" }
    ],
    "link": {
      "label": "View project details",
      "url": "https://www.dot.state.mn.us/d1/projects/hwy169bridge/",
      "deadline": null
    },
    "media": [],
    "status": null
  },
  "editorial": {
    "revision_of_post_id": null,
    "duplicate_of_id": null
  }
}
```

The envelope shape is stable. New fields may be added; nothing in §5 is removed without a version bump + migration plan.

---

## 5. Full field contract

All tables below: **Req** column is `Y` (required every time), `C` (conditional — see the rule), `N` (optional). All citations point into `docs/architecture/ROOT_SIGNAL_DATA_CONTRACT.md` (DC) and the Root Editorial server code.

### 5.1 Core identity

| Field | Req | Type | Rule |
|---|---|---|---|
| `title` | Y | string | 20–120 chars. Headline case. No trailing period. No calendar dates in the title (use relational language — "this weekend", "next Saturday" — because displayed dates drift as seed/ingest cadence shifts). |
| `post_type` | Y | enum | One of 9 values — see §6. DB `CHECK` constraint in migration `000216:58`. |
| `weight` | Y | enum | `heavy` \| `medium` \| `light`. Drives layout template selection. |
| `priority` | Y | int | 0–100. Placement ordering within weight class. See §5.5. |
| `source_language` | Y | string | ISO 639-1. Default `en`. Translations use `translation_of_id` (future; don't populate until the translation spec lands). |
| `status` | — | enum | **Editor-only after creation.** Signal MUST NOT set. On ingest, Editorial assigns `active` (default) or `in_review` (if any soft-fail flag fires — §11.2). |

### 5.2 Body text — three tiers + raw

| Tier | Column | Min | Target | Max | Used by templates |
|---|---|---|---|---|---|
| Body (full) | `body_raw` | **250** | 500–1500 | — | Post detail page. Always full text. |
| Heavy | `body_heavy` | **250** | 800 | 1400 | feature (2-col hero), feature-reversed, generous-exchange |
| Medium | `body_medium` | **150** | 200 | 280 | gazette, bulletin, spotlight-local, alert-notice |
| Light | `body_light` | **40** | 60 | 120 | digest, ledger, quick-ref, whisper-notice, ticker, ticker-update |

**Four load-bearing rules:**

1. **Validation floors are the Min column.** `body_raw` ≥ 250, `body_heavy` ≥ 250 when `weight = heavy`, `body_medium` ≥ 150 when `weight ∈ {heavy, medium}`, `body_light` ≥ 40 and ≤ 120. Anything below hard-fails. (The earlier table split Heavy into a "Heavy" row at 600 and a "Mid-heavy" row at 250 — that was template-selection guidance, not validation. Collapsed here to match the hard-fail rule in §11.2.)
2. **Target vs minimum.** The Target column is what you should aim for; posts below Target but above Min are accepted. The layout engine uses body length (along with weight) to choose between template variants — a `body_heavy` of 800 chars will land in the 2-col feature template; a `body_heavy` of 300 chars will route to the feature-reversed variant.
3. `body_raw` ≥ 250 applies to **every weight**, including `light`. A ticker appears as a one-liner on the broadsheet, but its detail page is still a full article. Signal must not ship 1-sentence posts.
4. Produce all tiers the weight requires **every time**. The layout engine chooses templates at slot-fill time; Signal cannot predict whether a given post will land in a feature slot or a ticker. Target numbers are **character counts**, spaces and punctuation included. An average English sentence is 100–120 chars.

**Body AST (`body_ast`)**: optional. Editorial uses Plate.js rich-text JSON. If Signal has no rich editor output, omit the field and Editorial will synthesise an AST from `body_raw` on first load.

### 5.3 Timeline

| Field | Req | Type | Rule |
|---|---|---|---|
| `published_at` | Y | timestamptz | ISO 8601 with timezone. When the **source material** was originally published — not when Signal processed it. Drives the 7-day recency eligibility filter. |
| `is_evergreen` | N | bool | Default `false`. Set `true` for references, directories, standing business listings. Bypasses the 7-day filter — the post stays in the eligible pool indefinitely. **Never evergreen a news story, event, or time-bound action.** |

### 5.4 Location and coordinates

| Field | Req | Type | Rule |
|---|---|---|---|
| `location` | N | string | Human-readable ("Aitkin, MN"). Display only. |
| `zip_code` | N | string | Primary ZIP if post has single-point location. Used to infer county via `zip_counties` join. |
| `latitude` / `longitude` | N | decimal | Point coordinates. Required if the post is intended to render on a map widget. |
| `tags.service_area` | Y | string[] | ≥1 tag. Format: `{county-slug}-county` or `statewide`. See §8. |

### 5.5 Priority scoring

Signal recommends a priority. The layout engine uses it for ordering within a weight class. Editors can override.

| Range | Meaning |
|---|---|
| 90–100 | Breaking / urgent — active emergencies, shelter openings, imminent deadlines |
| 70–89 | High — major new programs, time-sensitive events, significant local news |
| 50–69 | Standard — ongoing resources, regular events, routine updates |
| 30–49 | Lower — reference listings, evergreen content |
| 0–29 | Filler — brief, low editorial value |

### 5.6 Weight distribution expectations

Target mix across a weekly pool, per county:

| Weight | % of posts | Editorial role |
|---|---|---|
| `heavy` | 10–20% | Above-the-fold features. 1–3 per county per week. |
| `medium` | 40–60% | Core reporting. Bulk of the paper. |
| `light` | 30–50% | Tickers, briefs, classifieds. |

A batch that's 100% `medium` produces a wall of identical gazette cards with no visual pacing. Signal should consciously vary weight. (DC §4.5.)

### 5.7 Taxonomy / tags

| Field | Req | Type | Rule |
|---|---|---|---|
| `tags.topic` | Y | string[] | ≥1 topic from `data/tags.json`. Drives topic-grouping in editions. Open vocabulary — propose new slugs freely when existing ones don't fit. See `TAG_VOCABULARY.md` §1 for the current canonical list. |
| `tags.safety` | N | string[] | Access-policy modifiers — reserved vocabulary of flags that remove hesitation to seek a service (e.g., `no-id-required`, `ice-safe`, `sliding-scale`, `confidential`). See `TAG_VOCABULARY.md` §3. |
| `tags.service_area` | Y | string[] | ≥1. See §8. |

### 5.8 Source (summary — see §7 for the full treatment)

| Field | Req | Type | Rule |
|---|---|---|---|
| `source.kind` | Y | enum | `organization` \| `individual` \| `editorial`. |
| `source.source_url` | C | string | Required if kind ∈ {organization, individual}. Canonical URL of original source material. Used for dedup + audit trail. |
| `source.attribution_line` | Y | string | Human-readable. Rendered on post detail page. Examples: "MnDOT District 1 press release", "Instagram: @foodshelfmn". |
| `source.extraction_confidence` | N | int 0–100 | Signal's confidence that extracted fields faithfully represent the source. `<60` → soft fail → post lands `in_review`. Omit if you didn't compute one. |
| `source.organization` | C | object | Required if kind=organization. See §7.1. |
| `source.individual` | C | object | Required if kind=individual. See §7.2. |

### 5.9 Editorial metadata (`meta` → `post_meta` 1:1)

| Field | Req | Type | Rule |
|---|---|---|---|
| `meta.kicker` | Y | string | 1–4 words. Topic label above the title ("Roads", "Housing", "Spotlight"). |
| `meta.byline` | Y | string | Who wrote/produced the source content ("MnDOT District 1", "@foodshelfmn", "Root Editorial Staff"). Usually equals the organisation or individual display name. See §7.4 for when byline and attribution diverge. |
| `meta.deck` | C | string | 1–2 sentence standfirst. **Required for weight=heavy.** 120–240 chars. |
| `meta.pull_quote` | N | string | 1 sentence, 60–180 chars. Used by feature-story templates. |
| `meta.timestamp` | N | timestamptz | Override for display timestamp. Defaults to `published_at`. |
| `meta.updated` | N | string | Freeform "Updated: April 3". Shown on revisions. |

### 5.10 Field groups (summary — see §6 for per-post-type requirements)

| Group | Table | Cardinality | Used by |
|---|---|---|---|
| `datetime` | `post_datetime` | 0..1 | `event` (required), `action` (if deadline-linked) |
| `schedule` | `post_schedule` | 0..N | `reference`, `business` (operating hours); `need`/`aid` (volunteer windows) |
| `person` | `post_person` | 0..1 | `person` (required); `story` when profiling |
| `items` | `post_items` | 0..N | `need`, `aid`, `reference` |
| `contacts` | polymorphic `contacts` (with `contactable_type='post'`) | 0..N | All resource-oriented types. Required for `reference`, `business`, `need`, `aid` |
| `link` | `post_link` | 0..1 | `action` (required — the CTA); `event` (register link); `reference` (more-info link) |
| `media` | `post_media` | 0..N | All types. ≥1 hero recommended for `story`/`person`/`business`/heavy weights |
| `status` | `post_status` | 0..1 | `need`/`aid` (`open` \| `closed`) |

### 5.11 Lifecycle / trace

| Field | Req | Type | Rule |
|---|---|---|---|
| `submission_type` | Y | enum | Signal sets `ingested`. Editorial rejects any other value on this endpoint. |
| `is_urgent` | N | bool | **Editor-only.** Signal MUST NOT set. Ingest rejects if present. |
| `pencil_mark` | N | enum | **Editor-only.** `star` \| `heart` \| `smile` \| `circle` \| null. Ingest rejects if present. |
| `revision_of_post_id` | N | uuid | For corrections — see §12.1. |
| `duplicate_of_id` | N | uuid | Set by Signal when it detects a near-duplicate. Editor confirms merge. See §12.2. |

---

## 6. The 9 post types and their required fields

Per migration `000216:58` and `docs/architecture/POST_TYPE_SYSTEM.md`. The authoritative acceptance rules live in `data/audit-seed.mjs:47-57`; DC §7 summarises.

| `post_type` | Default weight | Required field groups | Recommended | `is_evergreen` default | Editorial intent |
|---|---|---|---|---|---|
| `story` | heavy | `meta.deck` (if heavy), `meta.byline` | `meta.pull_quote`, ≥1 hero media | false | Feature reporting, narrative. |
| `update` | light | `contacts` OR `link` (reader needs a next step) | `datetime` if dated | false | "News you should know." |
| `action` | medium | `link` (the CTA itself; `link.deadline` if time-bound) | `contacts`, `datetime` | false | Deadline-driven civic actions. Without `link`, the post is incomplete. |
| `event` | medium | `datetime`, `location`, (`contacts` OR `link` for RSVP) | `schedule` for recurring, media | false | Community gatherings. Calendar-pickable. |
| `need` | medium | `items`, `contacts`, `status` (`open`/`closed`) | `schedule` for drop-off windows, `link` | false | Community asks for help/volunteers/donations. |
| `aid` | medium | `items`, `contacts`, `status` | `schedule`, `link` | false | Community offers: food shelf open, free clinic, tool library. |
| `person` | medium | `person` (name, role, bio, quote), ≥1 media | `meta.byline` | false | Profile / spotlight. |
| `business` | medium | `contacts`, `schedule` (hours), `location` | media, `link` to business site | **true** | Standing listings for independent / co-op / nonprofit businesses. |
| `reference` | medium | `items` (directory entries), `contacts` (per entry if applicable) | `schedule`, `link` per entry | **true** | Resource directories. The list IS the post. |

**Rule:** if a required field group is missing for the chosen `post_type`, Signal must either populate it or downgrade the `post_type`. Example: an `event` without `datetime` is not an event — reclassify as `update`. An `action` without `link` is not an action — reclassify as `update` with contacts.

---

## 7. Source model

### 7.1 Organisation sources

Shape:

```json
"source": {
  "kind": "organization",
  "organization": {
    "name": "Minnesota Department of Transportation",
    "website": "https://www.dot.state.mn.us/",
    "instagram_handle": null,
    "twitter_handle": "mndotnews",
    "facebook_handle": "MnDOT",
    "address": null,
    "phone": null,
    "populations_served": null,
    "already_known_org_id": null
  },
  "source_url": "https://www.dot.state.mn.us/d1/projects/hwy169bridge/",
  "attribution_line": "MnDOT District 1 press release"
}
```

**Editorial's organisation dedup algorithm** (DC §5.1, will be implemented in the new ingest handler — see §18):

1. If `already_known_org_id` is set and resolves, **use it**. Signal sets this when high-confidence from a prior ingest round-trip.
2. Otherwise, look up by `website` domain (case-insensitive, strip `www.` and trailing slash). If matched, link and enrich NULL fields from the submission.
3. Otherwise, look up by exact `name`. If matched, link and enrich NULLs.
4. Otherwise, insert a new `organizations` row and link.

**Stale metadata:** if the submission has newer info than the stored row (e.g., changed website), Editorial flags the post `in_review` with a `source_stale` notice; editors decide whether to overwrite.

**Linkage:** posts are linked to organisations via `post_sources → sources → organizations`. `posts.organization_id` as a direct FK was removed in migration 122 on purpose — the three-table join lets one post cite multiple sources (org website AND Instagram AND a reader submission all about the same thing).

**Actor ID roundtrip:** after a successful ingest, we will return `source.organization.already_known_org_id` (even if you didn't provide one) — store it and pass it back on future submissions involving the same org to skip the dedup work. This corresponds to your `ActorNode.canonical_key` concept; map it into your graph.

### 7.2 Individual sources

```json
"source": {
  "kind": "individual",
  "individual": {
    "display_name": "Jamie Ochoa",
    "handle": "@jamielocal",
    "platform": "instagram",
    "platform_url": "https://instagram.com/jamielocal",
    "verified_identity": false,
    "consent_to_publish": true,
    "already_known_individual_id": null
  },
  "source_url": "https://instagram.com/p/Abc123/",
  "attribution_line": "Instagram: @jamielocal"
}
```

**Consent is load-bearing.** `consent_to_publish = false` ⇒ post lands `in_review`. Signal must not set `consent_to_publish = true` without a real basis: public post with clear CC/permissive signal, or explicit written permission captured and attached.

**Dedup:** the same four-step ladder as organisations (§7.1), keyed on `(platform, handle)` first, then `platform_url`, then `source_url` domain, then insert. Editorial returns `individual_id` on 201; Signal persists it alongside the originating `ActorNode` and passes it back as `already_known_individual_id` on future submissions.

**Editorial-side schema** (referenced so Root Signal knows which fields round-trip):

```sql
CREATE TABLE source_individuals (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    display_name          TEXT NOT NULL,
    handle                TEXT,                        -- '@jamielocal' (no leading '@' stored)
    platform              TEXT CHECK (platform IN (
                              'instagram','twitter','tiktok','facebook',
                              'bluesky','youtube','substack','other')),
    platform_url          TEXT,
    verified_identity     BOOLEAN NOT NULL DEFAULT false,
    consent_to_publish    BOOLEAN NOT NULL DEFAULT false,
    consent_source        TEXT,                        -- how consent was captured: 'public-cc', 'dm-permission', 'email', …
    consent_captured_at   TIMESTAMPTZ,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (platform, handle)
);
```

Linked to posts via `post_sources` (polymorphic: `source_type = 'individual'`, `source_id → sources(id)`). Editorial enriches NULL fields on match (identical semantics to §7.1 organisations): if the submission brings new `platform_url`, `verified_identity`, or `consent_to_publish = true` with a `consent_source`, those fields fill in on a matched row. Conflicting non-NULL values flag the post `in_review` with `source_stale`.

### 7.3 Editorial (editor-authored)

```json
"source": { "kind": "editorial", "attribution_line": "Root Editorial Staff" }
```

Used for <1% of posts an admin hand-authors through the Editorial admin UI. **Signal must never emit `kind = editorial` to this endpoint.** The ingest handler hard-rejects with `editorial_source_forbidden` (§11.3). Editorial-origin posts are created through a different internal path that does not accept external submissions. If Signal detects source material that appears to originate from Editorial itself (e.g., scraped our own public site), drop the post entirely — do not relabel it as `organization` either.

### 7.4 Byline vs attribution

`meta.byline` (who wrote it) and `source.attribution_line` (how we credit it) are usually identical. They diverge when:

- A reporter wrote a story that's carried on an organisation's site — byline = reporter name, attribution = "Published by <org>".
- Aggregated content — byline = organisation that published, attribution = "Aggregated from <platform>".

Default: byline = organisation name for org sources; byline = display name for individuals. Override when clearly different.

---

## 8. Geography and service area

### 8.1 Tag format

Every post carries ≥1 `service_area` tag:

- **County-specific:** `{county-slug}-county`. Slug = lowercase county name, kebab-cased, `.` stripped. Examples: `hennepin-county`, `aitkin-county`, `lac-qui-parle-county`, `st-louis-county`. Full list of all 87 MN counties is in `data/tags.json`.
- **Statewide:** `statewide`. Relevant everywhere in MN. Bypasses county boundaries in layout eligibility.
- **Multi-county:** e.g., `["hennepin-county", "ramsey-county", "dakota-county"]` for metro-wide content. Appears in each tagged county's edition independently.

### 8.2 Statewide pseudo-county

Migration `000236` added an `is_pseudo BOOLEAN` column on `counties` and inserted a "Statewide" row (`fips_code = 'statewide'`, MN center lat/lng). The layout engine branches to a dedicated `load_statewide_posts` helper for pseudo counties, which pulls posts with `service_area = 'statewide'`. Statewide posts do not automatically propagate to each county's edition — they sit in the Statewide edition alone, unless also tagged with the specific county.

### 8.3 Coordinates

- `latitude` / `longitude` — required if the post is intended for map rendering.
- Do not submit fake coordinates. If Signal only knows "somewhere in Aitkin County," leave lat/lng null and set `service_area: ["aitkin-county"]`.
- Precision: don't fuzz coordinates on the Signal side. Editorial will reduce precision for sensitive-population posts (e.g., shelter addresses) at render time, per `docs/architecture/PII_SCRUBBING.md` policy.

---

## 9. Media

### 9.1 Shape

```json
"field_groups": {
  "media": [
    {
      "source_image_url": "https://example.org/photo.jpg",
      "caption": "Volunteers at the food shelf on Saturday",
      "credit": "Photo by Jane Doe",
      "alt_text": "Three volunteers sorting produce into crates inside a warehouse",
      "license": "CC BY 2.0",
      "source_credit": "Jane Doe / Instagram @janedoe"
    }
  ]
}
```

- `source_image_url` (required per entry) — canonical URL. HTTPS only; must return an image content-type (JPEG, PNG, WebP, AVIF).
- `alt_text` (required per entry) — accessibility description. Do not use "image of" / "photo of". Describe the content. If alt text can't be produced, do not submit the image.
- `credit` / `source_credit` — photographer / authoring entity. Preferred over `caption` for attribution.
- `license` — SPDX-style codes (`CC0`, `CC BY 4.0`, `CC BY-NC 4.0`, `public-domain`, `all-rights-reserved-with-permission`, etc.). Editorial will not publish `all-rights-reserved` images without explicit policy clearance.

### 9.2 Editorial-side processing

Editorial fetches, normalises, and stores each submitted image server-side:

- Fetch `source_image_url` with a 5-second timeout, 5 MiB cap, following redirects.
- Validate the response body against magic bytes — content-type header alone is not trusted.
- Strip EXIF metadata, normalise to WebP at quality 85.
- Content-hash for dedup; reused `media` rows when the same image has been seen before.
- Store to internal object storage, populate `post_media.media_id`. Public rendering references the internal URL, not `source_image_url`.

This means `source_image_url` can expire (common on some CDNs) without breaking the published post. Root Signal does not need to re-host or keep the URL alive after submission.

### 9.3 SSRF protection

Editorial refuses localhost, private IPv4/IPv6 ranges, link-local addresses, `file://`, and non-HTTP(S) schemes. Do not submit them — they produce a hard-fail on the media entry.

---

## 10. Tags and categorisation

Full vocabulary reference: [`TAG_VOCABULARY.md`](TAG_VOCABULARY.md). This section summarises the kinds and their validation behaviour.

### 10.1 Tag kinds

Three tag kinds applied per post:

| `tags.kind` | Vocabulary | Usage |
|---|---|---|
| `topic` | Open — the vocabulary exists and is expected to grow. Propose new slugs freely whenever a post doesn't fit an existing one. | Drives topic-grouping in editions, search, discovery. ≥1 required. |
| `service_area` | Closed — 87 MN counties (`{county-slug}-county`) plus `statewide`. | Geographic eligibility. ≥1 required. |
| `safety` | Reserved — access-policy modifiers (see `TAG_VOCABULARY.md` §3). Additions proposed through the integration channel, not invented per-submission. | Removes hesitation to seek a service. Optional. |

Root Signal assigns all tag kinds on submission. Editors can add, remove, or rename tags post-ingest.

### 10.2 Unknown-slug behaviour

| Kind | Unknown slug |
|---|---|
| `service_area` | **Hard-fail** (`unknown_service_area`). Closed vocabulary; unknown always means typo. |
| `safety` | **Hard-fail** (`unknown_value`). Reserved — safety flags drive downstream behaviour and must be added deliberately. |
| `topic` | Post lands `in_review`; new tag row is auto-created with the submitted slug. Editors confirm or rename from the inbox. The topic vocabulary is intentionally open — do not force-fit unfamiliar content into the nearest existing slug, propose a new one. |

### 10.3 Category vs tags

Root Signal's `NodeMeta.category` concept maps to exactly one `topic` tag — the dominant topic for the post. Additional topics populate as additional entries in `tags.topic[]`. Editorial has no first-class `category` column; the first topic entry serves that role.

---

## 11. Validation, errors, and the 201/422 protocol

### 11.1 Response shape

**201 Created** — post ingested successfully.

```json
{
  "post_id": "e8c9d1a4-...",
  "status": "active",
  "organization_id": "f1a2b3c4-...",
  "individual_id": null,
  "idempotency_key_seen_before": false
}
```

- `post_id` — the UUID Editorial assigns.
- `status` — `active` (hard pass) or `in_review` (soft fail; editor needs to clear before publication).
- `organization_id` — if `source.kind = organization`, the resolved org UUID. Store this and pass as `source.organization.already_known_org_id` on future submissions involving the same org.
- `individual_id` — if `source.kind = individual`, the resolved individual UUID. Store and pass as `source.individual.already_known_individual_id`.
- `idempotency_key_seen_before` — if true, the post was already ingested under this key; Editorial returns the original `post_id` without inserting (see §12.3).

**422 Unprocessable Entity** — validation failures. Structured error list:

```json
{
  "message": "Validation failed",
  "errors": [
    { "field": "body_raw", "code": "below_min_length", "detail": "body_raw is 180 chars; minimum is 250" },
    { "field": "tags.service_area", "code": "missing_required", "detail": "at least one service_area tag required" },
    { "field": "post_type", "code": "unknown_value", "detail": "unknown post_type 'notice'; expected one of story|update|action|event|need|aid|person|business|reference" }
  ]
}
```

**`errors[]` object shape:** flat struct, all three fields always present. `field` is a dotted JSON path into the submission (e.g., `source.organization.website`). `code` is a stable string from the taxonomy in §11.3 — Signal should match on it; unknown codes should be treated as hard-fail for safety. `detail` is human-readable and may change between versions. Do not pattern-match on `detail`.

- `field` — dotted path into the submission.
- `code` — stable machine-readable symbol. See §11.3 for the taxonomy.
- `detail` — human-readable explanation.

**Other statuses:** 400 (malformed JSON), 401 (missing/invalid auth), 403 (API key lacks ingest scope), 409 (idempotency conflict — see §12.3), 429 (rate limit), 5xx (transient — retry with backoff).

### 11.2 Hard vs soft failures

**Hard failures → 422.** Signal must fix and retry (with a new idempotency key).

- Missing required field (§5 Y-rules).
- `body_raw` < 250 chars.
- `body_heavy` missing when `weight = heavy`, or < 250.
- `body_medium` missing when `weight ∈ {heavy, medium}`, or < 150.
- `body_light` missing, or < 40 or > 120 chars.
- Unknown `post_type`, `weight`, or `submission_type`.
- Zero `service_area` tags.
- Zero `topic` tags.
- `is_urgent` or `pencil_mark` set by Signal.
- Post-type-required field groups missing (§6).
- `source.kind = organization` with no `source_url`.
- `source.kind = individual` with `consent_to_publish = true` but no `platform_url` or `source_url`.
- `source.kind = editorial` — this endpoint is for Signal-submitted content only; editorial-origin posts are created through the admin UI, not ingest. Returns `editorial_source_forbidden`.

**Soft failures → 201 with `status = "in_review"`.** Post lands; editor must clear before it can be slotted in an edition.

- `source.extraction_confidence < 60`.
- `source.kind = organization` where name matched but website did not (possible bad dedup).
- `editorial.duplicate_of_id` set by Signal (editor confirms merge).
- `meta.deck` missing on a `weight = heavy` post.
- Submission carries newer org metadata than the stored row (`source_stale` notice).
- Individual source with `consent_to_publish = false`.

### 11.3 Error code taxonomy (stable)

| Code | Meaning |
|---|---|
| `missing_required` | A `Y` field is null/absent. |
| `below_min_length` | String length under the documented minimum. |
| `above_max_length` | String length over the documented maximum. |
| `unknown_value` | Enum value not in allowed set. |
| `invalid_format` | String doesn't parse (e.g., `published_at` is not valid ISO 8601). |
| `editor_only_field` | `is_urgent` / `pencil_mark` / status-write attempted. |
| `post_type_group_missing` | Field group required by `post_type` not populated. |
| `source_url_required` | Missing when kind ∈ {organization, individual}. |
| `organization_required` | `kind = organization` without `organization` object. |
| `consent_without_platform_url` | Individual source with consent but no platform URL. |
| `editorial_source_forbidden` | `source.kind = editorial` submitted to ingest. Editorial-origin posts are not ingestible. |
| `duplicate_body_tier` | Same content submitted as two different tiers. |
| `unknown_tag` | Safety tag not in the allowed vocabulary. |
| `unknown_service_area` | `service_area` tag isn't a known county slug or `statewide`. |
| `invalid_coordinates` | lat/lng out of range or obviously fake (e.g., 0,0). |
| `idempotency_conflict` | Same idempotency key, different payload (see §12.3). |
| `rate_limited` | 429. |

More codes may be added; Signal should treat unknown codes as hard-fail for safety.

### 11.4 Response time SLO

- 95th percentile < 500 ms for a well-formed submission.
- 99th percentile < 2 s.
- A slow response is not a failure — do not retry unless 5xx or 429.

---

## 12. Revisions, duplicates, idempotency

### 12.1 Revisions — for material corrections

Root Signal submits a **new post** with `editorial.revision_of_post_id` pointing at the prior version.

Editorial's behaviour:
1. Creates the new post row.
2. Flips the previous row's `status` to `archived` and chains the revision so `/admin/posts/{id}` can render history.
3. Auto-reflows any active edition that contained the previous post. The layout engine re-runs against the edition's slot configuration so the revised post takes the old post's place; editors see the updated layout on next open.

**When to revise:** material change (source content changed, correction issued, fact was wrong). Typo fixes belong to editors, not Root Signal.

### 12.2 Duplicates — when Signal finds two sources for the same underlying thing

Signal sets `editorial.duplicate_of_id` pointing at the post it believes this one duplicates. Editorial's default is to land the new post `in_review`; an editor confirms the merge, keeps the best fields from each, and retires the loser.

If Signal is unsure, leave `duplicate_of_id` null. Editorial has an admin "find duplicates" tool for editor-driven merging.

### 12.3 Idempotency — for safe retries

Every submission MUST carry `X-Idempotency-Key: <uuid>`. Editorial treats the (API key, idempotency key) tuple as unique for 24 hours.

- Same key + identical payload → 201 with `idempotency_key_seen_before: true`; no new insert, original `post_id` returned.
- Same key + different payload → 409 Conflict, `code: idempotency_conflict`. Signal must mint a new key (this represents a real bug on Signal's side).
- New key → normal processing.

**Recommended key strategy:** `<scout_run_id>:<signal_id>` or `<situation_id>:<dispatch_id>` hashed into a UUID v5. Stable under retry, unique across submissions. Do not use a timestamp.

**Payload comparison method.** Editorial computes `SHA-256(canonicalised_json_body)` and compares as hex. Canonicalisation rules:

1. Parse the incoming body as JSON, then re-serialise with **sorted object keys** (recursive).
2. **Strip insignificant whitespace** — no indentation, single space after `:` and `,` is fine.
3. `null` values are kept as `null`; missing keys are kept missing (do not normalise absent → null).
4. Numeric literals preserved exactly as submitted (no canonicalisation of e.g. `1.0` vs `1`).

Same logic applied to the stored reference request the second time a key arrives. Matching hashes → 201 with `idempotency_key_seen_before: true`; different hashes → 409. This makes retries immune to whitespace or key-order differences but catches real payload divergence.

**Editorial-side schema:**

```sql
CREATE TABLE api_idempotency_keys (
    key              UUID PRIMARY KEY,                -- the client-minted X-Idempotency-Key
    api_key_id       UUID NOT NULL REFERENCES api_keys(id),
    payload_hash     TEXT NOT NULL,                   -- 64-char lowercase hex, SHA-256 of canonicalised body
    response_status  INT NOT NULL,                    -- 201 or (rarely, for long-running handlers) a stored final status
    response_body    JSONB NOT NULL,                  -- the exact JSON returned on first success
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_api_idempotency_api_key_created ON api_idempotency_keys(api_key_id, created_at);
```

**TTL cleanup.** Editorial prunes idempotency rows older than 24 hours. Retries past that window produce a new insert as if the key were fresh. The 24-hour window is longer than any realistic retry budget; retries at longer intervals indicate a genuine problem to address at source.

**Why `(api_key_id, idempotency_key)` scoping.** Different API keys (e.g., staging vs prod, or a rotated key during cutover) can legitimately reuse the same idempotency UUID without collision.

---

## 13. Lifecycle and status

### 13.1 Status transitions

Signal sees two possible lifecycle outcomes on ingest:

```
active          (happy path — eligible for edition slotting)
in_review       (soft-fail — editor must clear)
(422)           (hard-fail — post not stored; Signal retries after fix)
```

After ingest, Signal does not control the post further. Editors can:

- `active` → `filled` (need/aid satisfied), `expired` (time-bound post past its date), `archived` (revision chain), `rejected` (content problem).
- `in_review` → `active` / `rejected`.

### 13.2 Acknowledgement

Root Signal receives 201/422 synchronously on submission. There is no out-of-band acknowledgement of downstream editorial actions (publish, reject, edit, merge). If a feedback channel becomes valuable later — e.g., to inform Root Signal's ranking or extraction quality models — it will be specified then.

### 13.3 What happens to `status = in_review` posts

They land in the **Signal Inbox** — an admin queue where editors review extraction quality, adjust fields, and approve or reject. A cleared post becomes `active` and eligible for slotting. A rejected post is out of rotation. Root Signal sees no feedback on this flow; correct it at the source if systematic patterns emerge (e.g., recurring `extraction_confidence` soft-fails suggest a scout or extraction issue worth investigating).

---

## 14. Auth and transport

### 14.1 Authentication — API key in Authorization header

```
Authorization: Bearer rsk_live_<32-char-url-safe-base64-token>
```

**Token format.**

- Prefix: `rsk_` (fixed), followed by environment indicator: `live_` (prod), `test_` (staging), `dev_` (local dev).
- Body: 32 url-safe base64 characters of cryptographic randomness.
- Full length: ~41 chars. Examples: `rsk_live_8f3a9c2b7e1d4f6a5c8b9e2d7f3a6c1b`, `rsk_test_2c7e5a8f1d9b3c6e4a7f2d8b1c5e9a3f`.
- The prefix is visible in logs for operator triage; the token body is never logged.
- Editorial stores only `SHA-256(full_token)`; the plaintext is shown once at issuance and never again.

**Scope.** Initial scope is `posts:create`. Keys are scope-checked at request time. Additional scopes (`posts:read`, `media:upload`, etc.) may be added later.

**Key storage schema:**

```sql
CREATE TABLE api_keys (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_name       TEXT NOT NULL,                 -- 'root-signal-prod', 'root-signal-staging', …
    prefix            TEXT NOT NULL,                 -- 'rsk_live_', 'rsk_test_', 'rsk_dev_'
    token_hash        TEXT NOT NULL UNIQUE,          -- SHA-256 hex of full token (plaintext is never stored)
    scopes            TEXT[] NOT NULL DEFAULT '{}',  -- e.g., ARRAY['posts:create']
    rotated_from_id   UUID REFERENCES api_keys(id),  -- rotation chain; null on original issue
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at        TIMESTAMPTZ,                   -- null = active; set on revoke
    last_used_at      TIMESTAMPTZ
);

CREATE INDEX idx_api_keys_active_token ON api_keys(token_hash) WHERE revoked_at IS NULL;
CREATE INDEX idx_api_keys_client_name ON api_keys(client_name) WHERE revoked_at IS NULL;
```

**Rotation.** `dev-cli apikey rotate <client-name>` issues a new key, sets `rotated_from_id` pointing at the old one, and leaves the old one active. Both keys validate until the old one is explicitly revoked — Root Signal rotates the client-side token, confirms traffic lands on the new key, then pages Editorial to revoke the old one. The overlap avoids a cutover race.

**`ServiceClient` extractor.** Editorial's auth extractor for service credentials, alongside `AdminUser` / `AuthenticatedUser` / `OptionalUser`. It:

1. Reads `Authorization: Bearer <token>`.
2. Computes `SHA-256(token)`.
3. Looks up `api_keys WHERE token_hash = $1 AND revoked_at IS NULL`.
4. Verifies required scope (`posts:create`) is in the row's `scopes` array.
5. Updates `last_used_at`.
6. Returns `ServiceClient { id, client_name, scopes }` for the handler.
7. On miss / revoked / scope-missing: returns 401 / 403 with a terse message (no hints about *why* the key failed, to prevent enumeration).

**Route path convention.** Editorial handlers use capital-P service-style paths (`/Posts/create_post`, `/Editions/publish`, etc.) — the house style in Editorial's CLAUDE.md: `URL paths follow /{Service}/{handler} or /{Object}/{id}/{handler}`. Call the capital-P path exactly.

**Issuance.** Editorial issues keys over a secure channel (1Password shared item or equivalent — not plain email or Slack). The operator runs `dev-cli apikey issue --client=root-signal-<env> --scopes=posts:create`, which prints the full token once.

### 14.2 Rate limits

- Per API key: **15 req/sec sustained, 50 req/sec burst** (token bucket, 50-token capacity, 15/sec refill).
- 429 response includes `Retry-After: <seconds>` header. Root Signal respects it.
- No per-day quota. If rate-limit tuning becomes necessary based on real traffic patterns, it's adjusted at the operator layer without changes to this spec.

### 14.3 Retries

- Editorial is idempotent under `X-Idempotency-Key` — retries are safe.
- Retry on: 429, 500, 502, 503, 504.
- Do not retry on: 400, 401, 403, 404, 409, 422.
- Use exponential backoff with jitter. Persistent failures should land in a local dead-letter queue rather than repeat indefinitely.

### 14.5 TLS / certificates

HTTPS only. TLS 1.2+. Editorial's cert is issued by a standard public CA; no mTLS.

### 14.5 Observability

- Editorial logs `{request_id, api_key_id, idempotency_key, post_id_or_error}` for every request.
- Root Signal sends `X-Request-ID: <uuid>` on every submission. Editorial echoes it in response headers and logs for correlation.

---

## 15. Mapping — Root Signal model → Editorial post

The mapping layer between Root Signal's vocabulary and Editorial's lives on Root Signal's side. The table below is the mapping Editorial expects; tightening is welcome, loosening breaks the contract.

### 15.1 Root Signal type → Editorial `post_type`

| Root Signal type | Editorial `post_type` | Weight hint | Notes |
|---|---|---|---|
| `Gathering` | `event` | medium (heavy for major community events) | `schedule` field group if recurring; `datetime` always required. |
| `Resource` | `aid` | medium | `items` = what's available; `contacts` + `link` + `status` required. |
| `HelpRequest` | `need` | medium (heavy if critical urgency) | `items` = what's being asked for; urgency → `priority` 80+. |
| `Announcement` | `action` if deadline-linked; otherwise `update` | light/medium | `action` requires `link.deadline`. |
| `Concern` | `story` when wrapped in a Situation with ≥2 response signals; otherwise drop | heavy for story | Do not emit bare `Concern` signals as Editorial posts — they're too thin without response context. |
| `Condition` | *do not emit* | — | Persisting environmental state; not publishable on its own. If a Condition becomes newsworthy, package it as a `story` through a Situation, not as its own post. |
| `Profile` | `person` | medium | `person` field group + ≥1 hero media. |
| `LocalBusiness` | `business` | medium | `contacts` + `schedule` (hours) + `location` + `is_evergreen = true`. |
| `Opportunity` | `action` | medium (heavy for high-deadline civic actions) | `link.deadline` populated. |
| `Job` | *do not emit* | — | Editorial has no `job` post_type. If civic-content job coverage becomes a priority, the `post_type` and this mapping will be extended; until that happens, Job signals stay on Root Signal's side only. |

### 15.2 Situation → Editorial `story`

A richly-developed Situation is the most valuable unit Signal produces that maps to Editorial's `story` post_type:

| Signal field | Editorial field |
|---|---|
| `situation.headline` | `title` (if 20–120 chars; else synthesise) |
| `situation.lede` | `meta.deck` |
| `situation.briefing_body` (LLM-synthesised markdown with `[signal:UUID]` citations) | `body_raw` (and synthesise `body_heavy` / `body_medium` / `body_light` from it) |
| `situation.centroid_lat` / `centroid_lng` | `latitude` / `longitude` |
| `situation.location_name` | `location` |
| `situation.category` | `tags.topic[0]` |
| `situation.signal_count` + `dispatch_count` | informs `priority` and `weight` |
| `situation.arc` | informs whether this is a fresh `story` (Emerging/Developing) or an update (Cooling/Cold) |
| signals with `actor_type = Organization` (highest signal_count within situation) | `source.organization` |

**On `[signal:UUID]` citations:** Preserve these tokens in `body_raw` and the three body tiers. Editorial parses them at render time and produces superscript citations linking to Root Signal's signal-detail URL (configured per deploy). If a UUID doesn't resolve on Root Signal's side, the citation renders as an unlinked superscript.

### 15.3 Signal → Editorial body tier generation

Signal's LLM (or deterministic pipeline) should produce all three tiers from the same source material:

- `body_heavy` (800 chars target): primary summary. Full narrative with context. Leads with the most important fact. Names the key actors and place. Ends with a one-sentence "what's next" or "why it matters."
- `body_medium` (200 chars): compressed version. One or two sentences. Preserves lede + place + call-to-action if any.
- `body_light` (60 chars): one-line digest. "What + where" in ~10 words.
- `body_raw` (500–1500 chars target): the full detail-page treatment. May be longer than `body_heavy`; may include `[signal:UUID]` citation anchors.

### 15.4 Actor → source

When a signal is sourced from an `ActorNode` with `actor_type = Organization`:
- `source.kind = "organization"`
- `source.organization.name` = `ActorNode.name`
- `source.organization.website` = first URL in `ActorNode.domains`
- `source.organization.instagram_handle` / `twitter_handle` / `facebook_handle` = extracted from `ActorNode.social_urls`
- `source.organization.already_known_org_id` = the Editorial UUID we returned on a prior submission for this actor (stored in your graph as `ActorNode.editorial_org_id` or similar)

When sourced from an `Individual`:
- `source.kind = "individual"`
- fields per §7.2

### 15.5 Actor roundtrip

Store Editorial's returned `organization_id` / `individual_id` alongside your `ActorNode.canonical_key`. On subsequent submissions, pass it back as `already_known_org_id` / `already_known_individual_id` to skip the dedup ladder. This keeps the Editorial-side org graph aligned with your actor graph without either system holding sole authority.

### 15.6 Citations / Evidence → attribution

- Each Editorial post has exactly one `source.source_url` (the canonical origin).
- Additional citations that Signal used during extraction (from `Evidence` nodes) are **not** directly represented on the Editorial post. If they strengthen editorial trust, mention them in `body_raw` as inline text.
- If one Editorial post meaningfully combines multiple sources (e.g., an Instagram notice + a confirmed press release), pick the authoritative source as `source_url` and mention the others in `body_raw` or `meta.byline`.

### 15.7 Geo mapping

- Signal `GeoPoint { lat, lng, precision }` → Editorial `latitude`/`longitude` at **Exact** or **Neighborhood** precision. Drop lat/lng for `Approximate`/`Region`.
- Signal `Region` → Editorial `service_area` tag(s). Minneapolis/St Paul Region → `["hennepin-county","ramsey-county"]`. Unsure → `["statewide"]` as fallback only if genuinely statewide-relevant.
- Signal `location_name` → Editorial `location`.

### 15.8 Schedule / recurrence

Signal's RRULE-based `Schedule` → Editorial `post_schedule` field group, expanded into discrete day entries:

```json
"schedule": [
  { "day": "Monday",    "opens": "09:00", "closes": "17:00" },
  { "day": "Wednesday", "opens": "09:00", "closes": "17:00" },
  { "day": "Friday",    "opens": "09:00", "closes": "15:00" }
]
```

For time-bound events (single or short recurrence), use `datetime.start_at` / `datetime.end_at` plus `datetime.recurring: true`. For open-ended recurrence, use `schedule[]`.

---

## 16. Complete worked examples

Each example is a complete, valid submission. All timestamps in America/Chicago unless stated. Paste directly into `curl` with the auth and idempotency headers from §14.

### 16.1 Event — Gathering → `event` — medium weight

```json
{
  "title": "Loaves and Fishes Community Meal Every Wednesday",
  "post_type": "event",
  "weight": "medium",
  "priority": 55,
  "body_raw": "Loaves and Fishes hosts a free community meal every Wednesday evening at Holy Rosary Church in the Phillips neighborhood, from 5:00 to 6:30 PM. No registration, no questions asked — all are welcome. The meal is hot, served in the church hall, and includes vegetarian options. Organizers note that winter attendance has grown steadily, with close to 200 guests served on the coldest evenings. Volunteers from area congregations plate and serve; a separate volunteer shift runs from 3:30 to 7:00 PM and is open to anyone who can show up. Donated fresh produce, bread, and prepared dishes are welcome; drop off at the side door before 4 PM. The program runs year-round and is supported by a mix of in-kind donations and a small grant from Catholic Charities.",
  "body_heavy": null,
  "body_medium": "A free, open-to-all community meal at Holy Rosary Church every Wednesday from 5 to 6:30 PM. Volunteers arrive at 3:30 PM; donated fresh food welcome at the side door by 4. Program runs year-round, supported by area congregations.",
  "body_light": "Free community meal every Wednesday, 5–6:30 PM at Holy Rosary, Phillips.",
  "published_at": "2026-04-14T10:00:00-05:00",
  "source_language": "en",
  "is_evergreen": false,
  "location": "Minneapolis, MN",
  "zip_code": "55407",
  "latitude": 44.9537,
  "longitude": -93.2596,
  "tags": {
    "service_area": ["hennepin-county"],
    "topic": ["food", "community"],
    "safety": []
  },
  "source": {
    "kind": "organization",
    "organization": {
      "name": "Loaves and Fishes",
      "website": "https://loavesandfishesmn.org/",
      "instagram_handle": "loavesandfishesmn",
      "twitter_handle": null,
      "facebook_handle": "LoavesandFishesMN",
      "address": "2424 18th Ave S, Minneapolis, MN 55404",
      "phone": "612-377-9810",
      "populations_served": ["low-income"],
      "already_known_org_id": null
    },
    "individual": null,
    "source_url": "https://loavesandfishesmn.org/meal-sites/holy-rosary/",
    "attribution_line": "Loaves and Fishes site listing",
    "extraction_confidence": 92
  },
  "meta": {
    "kicker": "Meals",
    "byline": "Loaves and Fishes",
    "deck": null,
    "pull_quote": null,
    "timestamp": null,
    "updated": null
  },
  "field_groups": {
    "datetime": {
      "start_at": "2026-04-22T17:00:00-05:00",
      "end_at": "2026-04-22T18:30:00-05:00",
      "cost": "Free",
      "recurring": true
    },
    "schedule": [
      { "day": "Wednesday", "opens": "17:00", "closes": "18:30" }
    ],
    "person": null,
    "items": [],
    "contacts": [
      { "contact_type": "phone", "contact_value": "612-377-9810", "contact_label": "Meal program office" },
      { "contact_type": "website", "contact_value": "https://loavesandfishesmn.org/meal-sites/holy-rosary/", "contact_label": "Site info" },
      { "contact_type": "address", "contact_value": "2424 18th Ave S, Minneapolis, MN 55404", "contact_label": "Holy Rosary Church" }
    ],
    "link": {
      "label": "Volunteer to serve",
      "url": "https://loavesandfishesmn.org/volunteer/",
      "deadline": null
    },
    "media": [
      {
        "source_image_url": "https://loavesandfishesmn.org/wp-content/uploads/2024/10/volunteers-serving.jpg",
        "caption": "Volunteers plating meals at a recent Wednesday service",
        "credit": "Photo by Loaves and Fishes",
        "alt_text": "Four volunteers in aprons scooping food from warming trays onto plates in a church hall",
        "license": "CC BY-NC 4.0"
      }
    ],
    "status": null
  },
  "editorial": {
    "revision_of_post_id": null,
    "duplicate_of_id": null
  }
}
```

### 16.2 Need — HelpRequest → `need` — medium weight, open status

```json
{
  "title": "Warming Shelter Seeks Cold-Weather Gear Donations",
  "post_type": "need",
  "weight": "medium",
  "priority": 78,
  "body_raw": "The Salvation Army Harbor Light Center is collecting cold-weather gear for its overnight warming shelter through the end of April. Staff report that donations of new or gently-used winter coats, insulated gloves, waterproof boots (men's sizes 9–13 especially), wool socks, and heavy-duty sleeping bags are running low after a long and unusually cold March. Drop-offs are accepted at the Harbor Light intake desk on Currie Avenue from 8 AM to 8 PM daily. Large donations (10+ items) can be scheduled for pickup by calling the shelter coordinator. Harbor Light also accepts monetary donations earmarked for gear purchase; the shelter has a direct wholesale relationship with a regional workwear supplier and can stretch a donation roughly 30% further than retail. Volunteers willing to sort and inventory donations are welcome on Saturday mornings from 9 to noon.",
  "body_medium": "Harbor Light warming shelter needs winter coats, boots (men's 9–13), wool socks, and sleeping bags through April. Drop off at Currie Avenue 8 AM–8 PM, or call to schedule pickup for large donations.",
  "body_light": "Warming shelter needs winter coats, boots, sleeping bags. Drop-offs 8–8 at Currie Ave.",
  "published_at": "2026-04-18T09:00:00-05:00",
  "source_language": "en",
  "is_evergreen": false,
  "location": "Minneapolis, MN",
  "zip_code": "55403",
  "latitude": 44.9786,
  "longitude": -93.2799,
  "tags": {
    "service_area": ["hennepin-county"],
    "topic": ["donations", "housing", "winter-gear"],
    "safety": ["extreme-cold-shelter"]
  },
  "source": {
    "kind": "organization",
    "organization": {
      "name": "Salvation Army Harbor Light Center",
      "website": "https://centralusa.salvationarmy.org/northern/harborlight/",
      "instagram_handle": null,
      "twitter_handle": null,
      "facebook_handle": "HarborLightMinneapolis",
      "address": "1010 Currie Ave, Minneapolis, MN 55403",
      "phone": "612-338-0113",
      "populations_served": ["homeless", "low-income"],
      "already_known_org_id": null
    },
    "individual": null,
    "source_url": "https://centralusa.salvationarmy.org/northern/harborlight/winter-gear-drive-2026/",
    "attribution_line": "Harbor Light Center announcement",
    "extraction_confidence": 90
  },
  "meta": {
    "kicker": "Winter Gear",
    "byline": "Salvation Army Harbor Light",
    "deck": null,
    "pull_quote": null
  },
  "field_groups": {
    "datetime": null,
    "schedule": [
      { "day": "Monday",    "opens": "08:00", "closes": "20:00" },
      { "day": "Tuesday",   "opens": "08:00", "closes": "20:00" },
      { "day": "Wednesday", "opens": "08:00", "closes": "20:00" },
      { "day": "Thursday",  "opens": "08:00", "closes": "20:00" },
      { "day": "Friday",    "opens": "08:00", "closes": "20:00" },
      { "day": "Saturday",  "opens": "09:00", "closes": "12:00" },
      { "day": "Sunday",    "opens": "08:00", "closes": "20:00" }
    ],
    "person": null,
    "items": [
      { "name": "Winter coats (adult men/women)", "detail": "New or gently used; all sizes" },
      { "name": "Insulated gloves", "detail": "Waterproof preferred" },
      { "name": "Boots (men's 9–13)", "detail": "Running especially low" },
      { "name": "Wool socks", "detail": "New only" },
      { "name": "Heavy-duty sleeping bags", "detail": "Rated 0°F or colder" }
    ],
    "contacts": [
      { "contact_type": "phone", "contact_value": "612-338-0113", "contact_label": "Intake desk" },
      { "contact_type": "address", "contact_value": "1010 Currie Ave, Minneapolis, MN 55403", "contact_label": "Drop-off location" }
    ],
    "link": {
      "label": "Donate money",
      "url": "https://centralusa.salvationarmy.org/northern/harborlight/give/",
      "deadline": null
    },
    "media": [],
    "status": { "state": "open", "verified": true }
  },
  "editorial": { "revision_of_post_id": null, "duplicate_of_id": null }
}
```

### 16.3 Aid — Resource → `aid` — medium weight

```json
{
  "title": "Free Diaper Bank Open Saturdays in North Minneapolis",
  "post_type": "aid",
  "weight": "medium",
  "priority": 60,
  "body_raw": "The Twin Cities Diaper Bank operates a free distribution site out of Shiloh Temple every Saturday from 10 AM to 1 PM. Families can pick up a month's supply of diapers and wipes in any size from newborn through pull-ups, no appointment or income verification required. Sizes 4, 5, and 6 — the ones most often needed but least often donated — are kept in stock specifically for this site. Staff ask families to bring their own tote bags when possible. The site also stocks baby formula (liquid and powder), but supply is limited and varies week-to-week; call ahead on Friday afternoons to confirm. Spanish and Hmong interpreters are available most Saturdays. The program is open to any family in need, regardless of where they live — families have traveled from St. Paul, Brooklyn Center, and even as far as St. Cloud.",
  "body_medium": "Free diapers (sizes newborn–pull-ups) and wipes every Saturday 10 AM–1 PM at Shiloh Temple, North Minneapolis. No appointment. Spanish/Hmong interpreters usually available. Some formula in stock.",
  "body_light": "Free diapers + wipes every Saturday, 10–1 at Shiloh Temple, North Mpls.",
  "published_at": "2026-04-16T13:00:00-05:00",
  "source_language": "en",
  "is_evergreen": false,
  "location": "Minneapolis, MN",
  "zip_code": "55411",
  "latitude": 44.9976,
  "longitude": -93.2983,
  "tags": {
    "service_area": ["hennepin-county"],
    "topic": ["childcare", "donations", "community"],
    "safety": []
  },
  "source": {
    "kind": "organization",
    "organization": {
      "name": "Twin Cities Diaper Bank",
      "website": "https://tcdiaperbank.org/",
      "instagram_handle": "tcdiaperbank",
      "twitter_handle": null,
      "facebook_handle": "tcdiaperbank",
      "address": null,
      "phone": "612-238-1717",
      "populations_served": ["families-with-children"],
      "already_known_org_id": null
    },
    "individual": null,
    "source_url": "https://tcdiaperbank.org/distribution-sites/shiloh/",
    "attribution_line": "TC Diaper Bank site listing",
    "extraction_confidence": 89
  },
  "meta": {
    "kicker": "Family Supplies",
    "byline": "Twin Cities Diaper Bank",
    "deck": null,
    "pull_quote": null
  },
  "field_groups": {
    "datetime": null,
    "schedule": [ { "day": "Saturday", "opens": "10:00", "closes": "13:00" } ],
    "person": null,
    "items": [
      { "name": "Diapers", "detail": "Newborn through pull-ups; sizes 4–6 always stocked" },
      { "name": "Wipes", "detail": "Unscented" },
      { "name": "Baby formula", "detail": "Limited; call Friday to confirm" }
    ],
    "contacts": [
      { "contact_type": "phone", "contact_value": "612-238-1717", "contact_label": "Program line (Fridays for formula check)" },
      { "contact_type": "address", "contact_value": "1201 W Broadway Ave, Minneapolis, MN 55411", "contact_label": "Shiloh Temple distribution" }
    ],
    "link": null,
    "media": [],
    "status": { "state": "open", "verified": true }
  },
  "editorial": { "revision_of_post_id": null, "duplicate_of_id": null }
}
```

### 16.4 Action — Announcement with deadline → `action` — medium weight

```json
{
  "title": "Public Comment Closes Friday on County Transit Plan",
  "post_type": "action",
  "weight": "medium",
  "priority": 82,
  "body_raw": "Hennepin County is accepting public comment on its 2027–2030 transit investment framework through 4:30 PM this Friday. The draft plan reshapes how the county prioritises bus-rapid-transit corridors, with proposed new lines along Nicollet, Central, and West Broadway, and reduced frequency on several suburban routes. Comment can be submitted online through the county's public-input portal, by email to the Transportation Department, or in person at the Thursday 6:30 PM public hearing at the Government Center. Staff will present the draft at the hearing and take questions. Written comments carry the same weight as verbal ones and are entered into the public record in full. The county has specifically asked for feedback from riders on the proposed frequency changes and from neighbourhoods along the new BRT corridors.",
  "body_medium": "Public comment on Hennepin County's 2027–2030 transit plan closes Friday at 4:30 PM. Submit online, by email, or attend Thursday's 6:30 PM hearing at the Government Center. Proposed BRT lines on Nicollet, Central, West Broadway.",
  "body_light": "Comment on county transit plan closes Fri 4:30 PM. Hearing Thu 6:30 PM, Gov Ctr.",
  "published_at": "2026-04-20T08:00:00-05:00",
  "source_language": "en",
  "is_evergreen": false,
  "location": "Minneapolis, MN",
  "zip_code": "55487",
  "latitude": 44.9774,
  "longitude": -93.2658,
  "tags": {
    "service_area": ["hennepin-county"],
    "topic": ["transit", "voting", "community"],
    "safety": []
  },
  "source": {
    "kind": "organization",
    "organization": {
      "name": "Hennepin County Transportation Department",
      "website": "https://www.hennepin.us/transportation",
      "instagram_handle": null,
      "twitter_handle": "HennepinCounty",
      "facebook_handle": "HennepinCounty",
      "address": "300 S 6th St, Minneapolis, MN 55487",
      "phone": "612-596-0300",
      "populations_served": null,
      "already_known_org_id": null
    },
    "individual": null,
    "source_url": "https://www.hennepin.us/transportation/public-input/transit-framework-2027-2030",
    "attribution_line": "Hennepin County public-input portal",
    "extraction_confidence": 95
  },
  "meta": {
    "kicker": "Civic Action",
    "byline": "Hennepin County",
    "deck": null,
    "pull_quote": null
  },
  "field_groups": {
    "datetime": {
      "start_at": "2026-04-23T18:30:00-05:00",
      "end_at": "2026-04-23T20:30:00-05:00",
      "cost": "Free",
      "recurring": false
    },
    "schedule": [],
    "person": null,
    "items": [],
    "contacts": [
      { "contact_type": "email", "contact_value": "transportation@hennepin.us", "contact_label": "Email comment" },
      { "contact_type": "phone", "contact_value": "612-596-0300", "contact_label": "Transportation Dept" }
    ],
    "link": {
      "label": "Submit comment online",
      "url": "https://www.hennepin.us/transportation/public-input/transit-framework-2027-2030/comment",
      "deadline": "2026-04-24T16:30:00-05:00"
    },
    "media": [],
    "status": null
  },
  "editorial": { "revision_of_post_id": null, "duplicate_of_id": null }
}
```

### 16.5 Story — Situation-derived feature → `story` — heavy weight

```json
{
  "title": "Aitkin County Bridge Closures Reshape Spring Commute",
  "post_type": "story",
  "weight": "heavy",
  "priority": 75,
  "body_raw": "For the thousand or so people who drive across the Highway 169 bridge over the Mississippi most mornings, three days in early April will mean a forty-minute detour. MnDOT's District 1 crews are replacing the bridge deck starting April 3, the second of five planned deck replacements on the Aitkin–Crow Wing corridor between now and fall 2027. Each closure is scheduled over a long weekend to minimise weekday disruption, but commuters from Garrison and Deerwood still face a detour via County 25 and Highway 6. [signal:d1a2c3f4-1111-2222-3333-444455556666] MnDOT's public meeting in Aitkin drew about forty residents last Tuesday, many with questions about school bus routing and emergency-vehicle access. District Engineer Marcia Torres answered each one on the record; the full transcript is posted. The county has coordinated with the Aitkin school district to pre-stage activity buses on the east side of the river during closure weekends. Sheriff Dan Guida confirmed that his deputies will pre-position a patrol car on the east side for the duration of each closure to cut response times. [signal:e2b3d4f5-2222-3333-4444-555566667777] For businesses on the corridor, the impact is real but manageable: the gas station at the Highway 6 junction expects a thirty per cent revenue bump during closure weekends, while the café in Garrison is bracing for its usual Saturday regulars to skip the trip. Local Facebook groups have organised carpool threads for the March of Dimes fundraiser scheduled the same weekend — a small sign that the county's long accommodation of construction seasons is, again, mostly about neighbours helping neighbours figure it out.",
  "body_heavy": "Three days of bridge-deck work on Highway 169 starting April 3 will send Aitkin County commuters on a 40-minute detour via County 25 and Highway 6. MnDOT's District 1 has scheduled the closure for the first of five deck replacements along the Aitkin–Crow Wing corridor planned through fall 2027. Last week's public meeting drew about forty residents, with questions about school bus routing, emergency response, and access to the regional hospital. District Engineer Marcia Torres committed to pre-positioned sheriff's deputies, coordinated school bus staging, and a posted detour-congestion update on MnDOT's 511 system every four hours during the closure. Local businesses anticipate mixed effects: the Highway 6 gas station expects higher weekend volume, while Garrison-side cafés prepare for a slow Saturday. Organisers of a March of Dimes fundraiser on April 4 are coordinating carpools through local Facebook groups to keep attendance up despite the detour.",
  "body_medium": "Highway 169 bridge closure April 3–5 sends commuters on a 40-min detour. MnDOT's first of five planned deck replacements; public meeting drew 40 residents with questions on buses, emergency access. Local carpools already forming for a fundraiser the same weekend.",
  "body_light": "Hwy 169 bridge closed April 3–5; 40-min detour via Co 25 + Hwy 6.",
  "published_at": "2026-04-02T14:00:00-05:00",
  "source_language": "en",
  "is_evergreen": false,
  "location": "Aitkin, MN",
  "zip_code": "56431",
  "latitude": 46.5327,
  "longitude": -93.7105,
  "tags": {
    "service_area": ["aitkin-county", "crow-wing-county"],
    "topic": ["transit", "community"],
    "safety": []
  },
  "source": {
    "kind": "organization",
    "organization": {
      "name": "Minnesota Department of Transportation",
      "website": "https://www.dot.state.mn.us/",
      "instagram_handle": null,
      "twitter_handle": "mndotnews",
      "facebook_handle": "MnDOT",
      "address": null,
      "phone": "651-296-3000",
      "populations_served": null,
      "already_known_org_id": null
    },
    "individual": null,
    "source_url": "https://www.dot.state.mn.us/d1/projects/hwy169bridge/",
    "attribution_line": "MnDOT District 1 press release and public-meeting transcript",
    "extraction_confidence": 90
  },
  "meta": {
    "kicker": "Roads",
    "byline": "Root Signal Situations / MnDOT",
    "deck": "A thousand-car morning commute, a forty-minute detour, and a county that's learned to plan around construction seasons.",
    "pull_quote": "The sheriff's pre-positioned patrol car is, at this point, just good neighbouring at scale.",
    "timestamp": null,
    "updated": null
  },
  "field_groups": {
    "datetime": {
      "start_at": "2026-04-03T06:00:00-05:00",
      "end_at": "2026-04-05T18:00:00-05:00",
      "cost": null,
      "recurring": false
    },
    "schedule": [],
    "person": null,
    "items": [],
    "contacts": [
      { "contact_type": "phone", "contact_value": "511", "contact_label": "MN 511 traveler info" },
      { "contact_type": "website", "contact_value": "https://511mn.org", "contact_label": "Live road conditions" }
    ],
    "link": {
      "label": "View project page",
      "url": "https://www.dot.state.mn.us/d1/projects/hwy169bridge/",
      "deadline": null
    },
    "media": [
      {
        "source_image_url": "https://www.dot.state.mn.us/d1/projects/hwy169bridge/images/hero.jpg",
        "caption": "The Highway 169 bridge over the Mississippi, looking east",
        "credit": "MnDOT District 1",
        "alt_text": "A steel truss bridge over a wide river, low morning sun behind bare trees on the far bank",
        "license": "public-domain"
      }
    ],
    "status": null
  },
  "editorial": {
    "revision_of_post_id": null,
    "duplicate_of_id": null
  }
}
```

### 16.6 Person — proposed Profile → `person` — medium weight

```json
{
  "title": "Marcella Ortiz Turns Resettlement Know-How Into a Community Practice",
  "post_type": "person",
  "weight": "medium",
  "priority": 48,
  "body_raw": "Marcella Ortiz arrived in the Twin Cities in 2009 as a refugee from Colombia, sponsored by a congregation in Roseville. Fifteen years later, she runs the resettlement case-work team at the Minnesota Council of Churches, where she has helped more than four hundred families navigate the first ninety days in the state — the window that decides whether a family stays, moves on, or returns. She'll tell you that the most important thing she's learned is that the official checklist misses the biggest variables: a bus route that actually serves the apartment, a neighbour who speaks enough of your language to be useful in a crisis, a grocery store that stocks the flour you grew up on. Her team's case plans are organised around those invisible structures. She also runs a quiet Saturday practice out of her church basement — a two-hour drop-in where people who want to volunteer, donate, or just understand the current arrival landscape can ask her anything. There's no agenda, no sign-in, and no guarantee she'll have the answer. 'I'm not the expert on everyone's life,' she says. 'I'm an expert on staying around long enough to ask the right questions the second time.'",
  "body_medium": "Marcella Ortiz runs the resettlement case-work team at the Minnesota Council of Churches, where her team organises case plans around the unofficial structures — bus routes, neighbours, familiar groceries — that determine whether a refugee family stays in the state. She runs an open Saturday drop-in out of her Roseville church basement for anyone who wants to understand the work.",
  "body_light": "Marcella Ortiz, MN Council of Churches case-work lead — open Saturday drop-in.",
  "published_at": "2026-04-19T09:30:00-05:00",
  "source_language": "en",
  "is_evergreen": false,
  "location": "Roseville, MN",
  "zip_code": "55113",
  "latitude": 45.0061,
  "longitude": -93.1566,
  "tags": {
    "service_area": ["ramsey-county"],
    "topic": ["resettlement", "immigration", "community-voices", "community"],
    "safety": []
  },
  "source": {
    "kind": "organization",
    "organization": {
      "name": "Minnesota Council of Churches",
      "website": "https://www.mnchurches.org/",
      "instagram_handle": "mnchurches",
      "twitter_handle": "mnchurches",
      "facebook_handle": "mnchurches",
      "address": "122 W Franklin Ave, Minneapolis, MN 55404",
      "phone": "612-230-3200",
      "populations_served": ["immigrant-families", "refugees"],
      "already_known_org_id": null
    },
    "individual": null,
    "source_url": "https://www.mnchurches.org/refugee-services/meet-the-team/",
    "attribution_line": "MN Council of Churches staff page and on-the-record interview, 2026-04-15",
    "extraction_confidence": 84
  },
  "meta": {
    "kicker": "Spotlight",
    "byline": "Root Editorial Staff",
    "deck": null,
    "pull_quote": "I'm an expert on staying around long enough to ask the right questions the second time.",
    "timestamp": null,
    "updated": null
  },
  "field_groups": {
    "datetime": null,
    "schedule": [ { "day": "Saturday", "opens": "10:00", "closes": "12:00" } ],
    "person": {
      "name": "Marcella Ortiz",
      "role": "Resettlement case-work lead, Minnesota Council of Churches",
      "bio": "Arrived in Minnesota in 2009 as a refugee from Colombia. Has led case-work for more than four hundred newly-arriving families.",
      "photo_url": "https://www.mnchurches.org/wp-content/uploads/2025/02/ortiz-portrait.jpg",
      "quote": "The official checklist misses the biggest variables — a bus route, a neighbour, the right grocery store."
    },
    "items": [],
    "contacts": [
      { "contact_type": "email", "contact_value": "m.ortiz@mnchurches.org", "contact_label": "Direct contact (office hours only)" },
      { "contact_type": "address", "contact_value": "St. Christopher's, 2300 Hamline Ave N, Roseville, MN 55113", "contact_label": "Saturday drop-in" }
    ],
    "link": null,
    "media": [
      {
        "source_image_url": "https://www.mnchurches.org/wp-content/uploads/2025/02/ortiz-portrait.jpg",
        "caption": "Marcella Ortiz at her Roseville office",
        "credit": "Photo by MN Council of Churches",
        "alt_text": "A middle-aged woman with shoulder-length dark hair seated at a cluttered desk, glancing toward a whiteboard covered in post-it notes",
        "license": "CC BY 4.0"
      }
    ],
    "status": null
  },
  "editorial": {
    "revision_of_post_id": null,
    "duplicate_of_id": null
  }
}
```

### 16.7 Business — proposed LocalBusiness → `business` — medium, evergreen

```json
{
  "title": "Nokomis Beach Coffee — Independent Roaster and Community Workspace",
  "post_type": "business",
  "weight": "medium",
  "priority": 35,
  "body_raw": "Nokomis Beach Coffee has operated out of its corner storefront on 50th Street since 2011, roasting its own beans in the back of the shop and pouring them at a long wooden counter facing the lake. Owner Tomás Rivera sources from a rotating set of three farms in Oaxaca and one in the Peruvian highlands; the shop's Instagram posts a tasting note whenever a new roast lands. Beyond coffee, the café functions as the neighbourhood's informal shared office — a dozen laptops on a weekday morning, knitting circles on Wednesday evenings, and a monthly open mic in the side room. Pastries come from the Mexican bakery three blocks west. Wi-Fi is free and uncapped; the front windows are unusually generous with natural light. There's a small patio with dog bowls. The shop closes on Easter Sunday and Christmas but is otherwise open every day. Tipping is encouraged; staff share tips equally regardless of shift.",
  "body_medium": "Nokomis Beach Coffee — an independent roaster on 50th Street that doubles as the neighbourhood's informal shared office, with knitting circles, a monthly open mic, and pastries from the Mexican bakery three blocks west. Free Wi-Fi, a patio with dog bowls, and beans from rotating farms in Oaxaca and Peru.",
  "body_light": "Nokomis Beach Coffee — indie roaster, great Wi-Fi, dog bowls on the patio.",
  "published_at": "2026-04-10T12:00:00-05:00",
  "source_language": "en",
  "is_evergreen": true,
  "location": "Minneapolis, MN",
  "zip_code": "55417",
  "latitude": 44.9091,
  "longitude": -93.2402,
  "tags": {
    "service_area": ["hennepin-county"],
    "topic": ["culture", "community", "food"],
    "safety": []
  },
  "source": {
    "kind": "organization",
    "organization": {
      "name": "Nokomis Beach Coffee",
      "website": "https://nokomisbeachcoffee.com/",
      "instagram_handle": "nokomisbeachcoffee",
      "twitter_handle": null,
      "facebook_handle": "NokomisBeachCoffee",
      "address": "5030 28th Ave S, Minneapolis, MN 55417",
      "phone": "612-729-9002",
      "populations_served": null,
      "already_known_org_id": null
    },
    "individual": null,
    "source_url": "https://nokomisbeachcoffee.com/",
    "attribution_line": "Nokomis Beach Coffee (with on-the-record owner interview)",
    "extraction_confidence": 91
  },
  "meta": {
    "kicker": "Local Business",
    "byline": "Root Editorial Staff",
    "deck": null,
    "pull_quote": null
  },
  "field_groups": {
    "datetime": null,
    "schedule": [
      { "day": "Monday",    "opens": "06:30", "closes": "19:00" },
      { "day": "Tuesday",   "opens": "06:30", "closes": "19:00" },
      { "day": "Wednesday", "opens": "06:30", "closes": "21:00" },
      { "day": "Thursday",  "opens": "06:30", "closes": "19:00" },
      { "day": "Friday",    "opens": "06:30", "closes": "21:00" },
      { "day": "Saturday",  "opens": "07:00", "closes": "21:00" },
      { "day": "Sunday",    "opens": "07:00", "closes": "18:00" }
    ],
    "person": null,
    "items": [],
    "contacts": [
      { "contact_type": "phone", "contact_value": "612-729-9002", "contact_label": "Shop" },
      { "contact_type": "address", "contact_value": "5030 28th Ave S, Minneapolis, MN 55417", "contact_label": "Storefront" },
      { "contact_type": "website", "contact_value": "https://nokomisbeachcoffee.com/", "contact_label": "Website + current roast notes" }
    ],
    "link": {
      "label": "See current roast notes",
      "url": "https://nokomisbeachcoffee.com/current",
      "deadline": null
    },
    "media": [
      {
        "source_image_url": "https://nokomisbeachcoffee.com/images/storefront.jpg",
        "caption": "The storefront on 50th Street",
        "credit": "Nokomis Beach Coffee",
        "alt_text": "A single-story brick coffee shop with a large front window and a sandwich board on the sidewalk",
        "license": "all-rights-reserved-with-permission"
      }
    ],
    "status": null
  },
  "editorial": {
    "revision_of_post_id": null,
    "duplicate_of_id": null
  }
}
```

### 16.8 Reference — resource directory → `reference` — medium, evergreen

```json
{
  "title": "Free Legal Clinics for Tenants in Ramsey County",
  "post_type": "reference",
  "weight": "medium",
  "priority": 52,
  "body_raw": "Tenants in Ramsey County facing eviction, lease disputes, repair issues, or lease-break questions have at least four free legal options. Southern Minnesota Regional Legal Services runs a weekday intake line and can represent income-qualified clients in court. Volunteer Lawyers Network offers a walk-in clinic every Tuesday evening at the Ramsey County Law Library — no appointment needed, no income cap for thirty-minute consultations. The Tenant Resource Center has drop-in hours at the East Side neighbourhood office on Wednesdays, with case managers who can help negotiate with landlords directly. Mid-Minnesota Legal Aid handles federal-housing issues specifically. Each organisation has different income eligibility and case scope; tenants should call the intake line of whichever matches their situation closest. For same-day emergency eviction defence, Volunteer Lawyers Network's hotline is the fastest route.",
  "body_medium": "Four free legal options for Ramsey County tenants: SMRLS (weekday intake, income-qualified), Volunteer Lawyers Network (Tuesday walk-in, no income cap for 30-min consults), Tenant Resource Center (Wednesday drop-in), Mid-Minnesota Legal Aid (federal-housing).",
  "body_light": "Free legal help for tenants in Ramsey County — four intake options.",
  "published_at": "2026-04-15T10:00:00-05:00",
  "source_language": "en",
  "is_evergreen": true,
  "location": "St. Paul, MN",
  "zip_code": "55102",
  "latitude": 44.9537,
  "longitude": -93.0900,
  "tags": {
    "service_area": ["ramsey-county"],
    "topic": ["legal", "housing", "community"],
    "safety": []
  },
  "source": {
    "kind": "editorial",
    "organization": null,
    "individual": null,
    "source_url": null,
    "attribution_line": "Compiled by Root Editorial Staff with confirmations from each listed organisation",
    "extraction_confidence": null
  },
  "meta": {
    "kicker": "Resources",
    "byline": "Root Editorial Staff",
    "deck": null,
    "pull_quote": null
  },
  "field_groups": {
    "datetime": null,
    "schedule": [],
    "person": null,
    "items": [
      {
        "name": "Southern Minnesota Regional Legal Services (SMRLS)",
        "detail": "Weekday intake line 8:30–5:00. Income-qualified. Full representation in court."
      },
      {
        "name": "Volunteer Lawyers Network (VLN)",
        "detail": "Tuesday walk-in clinic, 5:30–7:30 PM, Ramsey County Law Library. No income cap for 30-min consults. Emergency-eviction hotline otherwise."
      },
      {
        "name": "Tenant Resource Center",
        "detail": "Wednesday drop-in, 3–6 PM, East Side neighbourhood office. Case managers negotiate directly with landlords."
      },
      {
        "name": "Mid-Minnesota Legal Aid",
        "detail": "Federal-housing specifically. Intake via online form."
      }
    ],
    "contacts": [
      { "contact_type": "phone", "contact_value": "651-222-5863", "contact_label": "SMRLS intake" },
      { "contact_type": "phone", "contact_value": "612-752-6655", "contact_label": "VLN emergency-eviction hotline" },
      { "contact_type": "phone", "contact_value": "651-789-5700", "contact_label": "Tenant Resource Center" },
      { "contact_type": "website", "contact_value": "https://mylegalaid.org/", "contact_label": "Mid-Minnesota Legal Aid (online intake)" }
    ],
    "link": null,
    "media": [],
    "status": null
  },
  "editorial": {
    "revision_of_post_id": null,
    "duplicate_of_id": null
  }
}
```

### 16.9 Update — minimal `light` weight

```json
{
  "title": "Sabathani Community Center Extends Tax-Help Hours Through April 29",
  "post_type": "update",
  "weight": "light",
  "priority": 60,
  "body_raw": "The free tax-preparation program at Sabathani Community Center has extended its hours through April 29 to accommodate the surge of late-filers and amended-return requests. Drop-ins accepted Monday–Thursday 4–8 PM and Saturday 10–2; no appointment needed. The program is staffed by IRS-certified volunteers and can handle federal and state returns plus amended returns back three years. Bring last year's return, all current-year tax forms, photo ID, and Social Security cards for everyone on the return. Most visits take about an hour. The service is free and open to any filer with household income under $67,000.",
  "body_medium": null,
  "body_light": "Sabathani free tax help extended through April 29. Drop-in M–Th 4–8, Sat 10–2.",
  "published_at": "2026-04-20T14:00:00-05:00",
  "source_language": "en",
  "is_evergreen": false,
  "location": "Minneapolis, MN",
  "zip_code": "55409",
  "latitude": 44.9194,
  "longitude": -93.2785,
  "tags": {
    "service_area": ["hennepin-county"],
    "topic": ["community", "employment", "legal"],
    "safety": []
  },
  "source": {
    "kind": "organization",
    "organization": {
      "name": "Sabathani Community Center",
      "website": "https://sabathani.org/",
      "instagram_handle": "sabathanicc",
      "twitter_handle": null,
      "facebook_handle": "SabathaniCommunityCenter",
      "address": "310 E 38th St, Minneapolis, MN 55409",
      "phone": "612-827-5981",
      "populations_served": null,
      "already_known_org_id": null
    },
    "individual": null,
    "source_url": "https://sabathani.org/programs/tax-help-2026/",
    "attribution_line": "Sabathani Community Center program page",
    "extraction_confidence": 93
  },
  "meta": {
    "kicker": "Taxes",
    "byline": "Sabathani Community Center",
    "deck": null,
    "pull_quote": null
  },
  "field_groups": {
    "datetime": null,
    "schedule": [
      { "day": "Monday",    "opens": "16:00", "closes": "20:00" },
      { "day": "Tuesday",   "opens": "16:00", "closes": "20:00" },
      { "day": "Wednesday", "opens": "16:00", "closes": "20:00" },
      { "day": "Thursday",  "opens": "16:00", "closes": "20:00" },
      { "day": "Saturday",  "opens": "10:00", "closes": "14:00" }
    ],
    "person": null,
    "items": [],
    "contacts": [
      { "contact_type": "phone", "contact_value": "612-827-5981", "contact_label": "Tax help line" },
      { "contact_type": "address", "contact_value": "310 E 38th St, Minneapolis, MN 55409", "contact_label": "Sabathani Community Center" }
    ],
    "link": {
      "label": "What to bring",
      "url": "https://sabathani.org/programs/tax-help-2026/#what-to-bring",
      "deadline": "2026-04-29T20:00:00-05:00"
    },
    "media": [],
    "status": null
  },
  "editorial": { "revision_of_post_id": null, "duplicate_of_id": null }
}
```

### 16.10 Revision — correcting an earlier post

Same envelope as §16.4, but with `editorial.revision_of_post_id` set and `meta.updated` populated:

```json
{
  "title": "Public Comment on County Transit Plan Extended to Monday",
  "post_type": "action",
  "weight": "medium",
  "priority": 82,
  "body_raw": "Hennepin County has extended the public-comment window on its 2027–2030 transit investment framework from Friday 4:30 PM to Monday April 28 at 4:30 PM. The county's Transportation Department said the extension was granted in response to requests from three neighbourhood associations along the proposed Nicollet BRT corridor, which said they needed more time to collect resident feedback after last Thursday's public hearing. ... (full body) ...",
  "body_medium": "Hennepin County extended public-comment on the 2027–2030 transit plan to Monday April 28, 4:30 PM. Three neighbourhood associations requested the extension after Thursday's public hearing.",
  "body_light": "Transit-plan comment extended to Mon April 28, 4:30 PM.",
  "published_at": "2026-04-24T16:00:00-05:00",
  "source_language": "en",
  "is_evergreen": false,
  "location": "Minneapolis, MN",
  "zip_code": "55487",
  "latitude": 44.9774,
  "longitude": -93.2658,
  "tags": {
    "service_area": ["hennepin-county"],
    "topic": ["transit", "voting", "community"],
    "safety": []
  },
  "source": { "kind": "organization", "organization": { "name": "Hennepin County Transportation Department", "website": "https://www.hennepin.us/transportation", "already_known_org_id": "<UUID from prior ingest>" }, "source_url": "https://www.hennepin.us/transportation/public-input/transit-framework-2027-2030", "attribution_line": "Hennepin County public-input portal", "extraction_confidence": 96 },
  "meta": { "kicker": "Civic Action", "byline": "Hennepin County", "deck": null, "updated": "Updated April 24: comment window extended to Monday" },
  "field_groups": {
    "link": { "label": "Submit comment online", "url": "https://www.hennepin.us/transportation/public-input/transit-framework-2027-2030/comment", "deadline": "2026-04-28T16:30:00-05:00" },
    "contacts": [ { "contact_type": "email", "contact_value": "transportation@hennepin.us", "contact_label": "Email comment" } ],
    "datetime": null, "schedule": [], "person": null, "items": [], "media": [], "status": null
  },
  "editorial": {
    "revision_of_post_id": "<post_id of the April 20 original>",
    "duplicate_of_id": null
  }
}
```

Editorial, on receiving this, will:
1. Insert the new post.
2. Mark the prior post `status = archived`.
3. Chain the `revision_of_post_id` pointer so `/admin/posts/<new_id>` shows the revision history.
4. Auto-reflow any active-edition slot that contained the old post.

---

## 17. Glossary

- **Broadsheet:** the weekly per-county layout Root Editorial generates from eligible posts. Templates decide which posts go into which slots.
- **Edition:** a specific instance of a broadsheet for a specific county + date range. Has lifecycle: `draft` → `in_review` → `approved` → `published`.
- **Envelope:** the top-level JSON structure a single POST carries. One post per envelope.
- **Field group:** structured supplement attached to a post — `datetime`, `schedule`, `items`, `person`, `link`, `media`, `contacts`, `status`. Each maps to its own table.
- **Hard failure:** validation failure that rejects the submission entirely (422).
- **Idempotency key:** per-submission UUID that allows safe retry.
- **Ingested:** `submission_type = "ingested"`. What Signal sets; what distinguishes Signal-produced posts from editor-authored or reader-submitted.
- **Kicker:** 1–4 word topical label that renders above the title.
- **Pseudo-county:** a `counties` row with `is_pseudo = true`. Statewide is the canonical one.
- **Post type:** one of 9 values — `story`, `update`, `action`, `event`, `need`, `aid`, `person`, `business`, `reference`.
- **Service area:** geographic tag. `{county-slug}-county` or `statewide`. ≥1 required per post.
- **Soft failure:** validation concern that lands the post `in_review` but doesn't reject — editor clears.
- **Statewide:** the pseudo-county for MN-wide content. Posts tagged `statewide` show in the Statewide edition, not automatically in every county.
- **Weight:** `heavy` / `medium` / `light`. Drives template selection.

---

## 18. References

- `docs/architecture/ROOT_SIGNAL_DATA_CONTRACT.md` — the authoritative on-the-wire contract (2026-04-19, 441 lines).
- `docs/architecture/POST_TYPE_SYSTEM.md` — design rationale for the 9-type taxonomy.
- `docs/architecture/SIGNAL_INBOX.md` — admin UI design for the `in_review` queue.
- `docs/architecture/DATABASE_SCHEMA.md` — authoritative schema reference.
- `docs/architecture/DATA_MODEL.md` — domain model overview.
- `docs/architecture/PII_SCRUBBING.md` — privacy/PII policy.
- `docs/architecture/ABUSE_REPORTING.md` — content moderation policy (post-publication; not a Signal concern).
- `docs/guides/ROOT_SIGNAL_MEDIA_INGEST.md` — media processing pipeline design reference.
- `docs/guides/POST_EDITION_LIFECYCLE.md` — content-hash dedup design.
- `docs/guides/SEED_DATA_ENRICHMENT_PLAN.md` — current seed-data enrichment; the informal "contract acceptance" tool is `data/audit-seed.mjs`.
- `docs/DECISIONS_LOG.md` — 2026-04-20 entry covers the Statewide pseudo-county, 0-slot lifecycle gate, Plus additional decisions relevant to ingest.
- `docs/TODO.md` — active work queue; item #1 (ingest endpoint) now covers individual-source schema and Signal Inbox as folded scope.
- `docs/status/2026_04_22_ROOT_SIGNAL_INTEGRATION_GAPS.md` — companion to this request doc; internal tracking.

Root Signal side (`/Users/Commons/Developer/Fourth Places/rootsignal`, branch `dev` at 2026-04-22):
- `docs/editorial-surface-area-request.md` — your existing request for the Editorial integration.
- `docs/plans/2026-03-13-feat-rich-annotated-briefs-plan.md` — brief-with-`[signal:UUID]`-citations design; directly relevant to Situation → Editorial `story` mapping.
- `docs/architecture/story-weaver.md` — Situation / Story materialisation.
- `docs/brainstorm-reactive-pipeline.md` — event-driven pipeline direction.
- `modules/rootsignal-common/src/types.rs:193-239` — `NodeMeta`.
- `modules/rootsignal-common/src/types.rs:1492-1543` — `SituationNode` + `DispatchNode`.
- `modules/rootsignal-common/src/types.rs:126-153` — `ActorNode`.
- `modules/rootsignal-api/src/graphql/` — current GraphQL surface.

---

**End of specification.** Root Signal's implementation of this integration is the critical next step in Root Suite — some interpretation of this will ship. Push back on specifics if the implementation path calls for it; the contract described here is the shape Editorial accepts.
