# Root Signal → Root Editorial Data Contract

**Status:** Authoritative. Supersedes `docs/architecture/ROOT_SIGNAL_SPEC.md` (2026-03-10) and `docs/guides/ROOT_SIGNAL_INGEST_SPEC.md` (draft). Both are now marked superseded with a banner pointing here.
**Last updated:** 2026-04-19.
**Audience:** Root Signal engineering, Root Editorial editors, CMS implementers.

---

## 1. Roles

**Root Signal** is the *producer*. It ingests raw source material (press releases, agency bulletins, organization websites, social posts, community submissions) and produces well-shaped posts that meet the contract in this document. It is the origin of >99% of posts in the system.

**Root Editorial** is the *consumer*. It receives posts from Signal, stores them, lets editors review/curate, generates weekly editions, and publishes. Editors can manually create posts through the admin CMS, but this is rare (<1%) and generally reserved for time-sensitive local additions the Signal pipeline missed.

**Critical distinction:** Root Signal produces complete posts, not drafts. "Complete" means every required field and every post_type-appropriate field group is populated to the minimum quality bar in §4 and §7. A post that ingests with just a title and a one-line body is a bug, not a partial post.

---

## 2. Submission envelope

Signal submits posts one at a time to `POST /Posts/create_post`. The envelope:

```json
{
  "title": "Highway 169 Bridge Deck Work Scheduled for April 3–5",
  "post_type": "update",
  "weight": "light",
  "priority": 50,
  "body_raw": "<full body text, at minimum the body_heavy content — see §4>",
  "body_heavy": "<weight-conditional, see §4>",
  "body_medium": "<weight-conditional, see §4>",
  "body_light": "<always required>",
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
    "population": []
  },
  "source": {
    "kind": "organization",
    "organization": {
      "name": "Minnesota Department of Transportation",
      "website": "https://www.dot.state.mn.us/",
      "instagram_handle": null,
      "twitter_handle": "mndotnews",
      "facebook_handle": "MnDOT",
      "already_known_org_id": null
    },
    "source_url": "https://www.dot.state.mn.us/d1/projects/hwy169bridge/",
    "attribution_line": "MnDOT District 1 press release",
    "extraction_confidence": 88
  },
  "meta": {
    "kicker": "Roads",
    "byline": "MnDOT District 1",
    "deck": null,
    "pull_quote": null
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

Field semantics, required/optional flags, and length targets are specified in §3–§9. The envelope shape is stable — new fields may be added but nothing in §3 will be removed without a version bump.

---

## 3. Post contract — full field inventory

Grouped by concern. "Req" column: **Y** = required on every post; **C** = conditional (see rules); **N** = optional.

### 3.1 Core identity

| Field | Req | Type | Shape / rules |
|---|---|---|---|
| `title` | Y | string | 20–120 chars. Headline case. No trailing period. |
| `post_type` | Y | enum | One of the 9 values in §7. Drives validation of field groups. |
| `weight` | Y | enum | `heavy` \| `medium` \| `light`. See §4 and §11. |
| `priority` | Y | int | 0–100. Placement ordering within weight class. See §4.4. |
| `source_language` | Y | string | ISO 639-1. Default `en`. Translations use `translation_of_id`. |
| `status` | — | enum | Always inserted as `active`. Editor-only after that. See §10. |

### 3.2 Body text (three tiers + source)

| Field | Req | Type | Shape / rules |
|---|---|---|---|
| `body_raw` | Y | text | Full editorial body. Minimum 250 chars for **all weights** including `light` — see §4.1. |
| `body_heavy` | C | text | Required if `weight = heavy`. See §4 for targets. |
| `body_medium` | C | text | Required if `weight ∈ {heavy, medium}`. See §4. |
| `body_light` | Y | text | Always required — used in ticker/digest templates. See §4. |
| `body_ast` | N | jsonb | Plate.js rich-text AST. Signal may omit; Root Editorial generates from `body_raw` on first load if missing. |

### 3.3 Timeline

| Field | Req | Type | Shape / rules |
|---|---|---|---|
| `published_at` | Y | timestamptz | When the source material was originally published (not when Signal processed it). |
| `is_evergreen` | N | bool | Default `false`. Set `true` for references, directories, standing business listings. Bypasses the 7-day eligibility filter. **Never evergreen a news story.** |

### 3.4 Location

| Field | Req | Type | Shape / rules |
|---|---|---|---|
| `location` | N | string | Human-readable ("Aitkin, MN"). |
| `zip_code` | N | string | Primary ZIP if the post has a single-point location. |
| `latitude` / `longitude` | N | decimal | Point coordinates. Required if the post will appear on a map widget. |
| `tags.service_area` | Y | string[] | At least one. Format: `{county-slug}-county` or `statewide`. See §6. |

### 3.5 Taxonomy

| Field | Req | Type | Shape / rules |
|---|---|---|---|
| `tags.topic` | Y | string[] | At least one topic from the allowed set in `data/tags.json`. Drives topic-grouping in editions. |
| `tags.population` | N | string[] | Audience tags (`youth`, `seniors`, `immigrant-families`, etc.). |
| `tags.safety` | N | string[] | Safety/emergency flags (`ice-resistance`, `sanctuary-resource`, etc.). |

### 3.6 Source (see §5 for full treatment)

| Field | Req | Type | Shape / rules |
|---|---|---|---|
| `source.kind` | Y | enum | `organization` \| `individual` \| `editorial` (for that <1% of editor-written posts). |
| `source.source_url` | C | string | Required if `kind ∈ {organization, individual}`. The canonical URL of the original post / press release. |
| `source.attribution_line` | Y | string | Human-readable ("MnDOT District 1 press release", "Instagram: @localfoodshelf"). Rendered on the post detail page. |
| `source.extraction_confidence` | N | int 0–100 | Signal's confidence that the extracted fields (title/body/dates/etc.) faithfully represent the source. Low values (<60) surface for editor review before publish. |
| `source.organization` | C | object | Required if `kind = organization`. See §5.1. |
| `source.individual` | C | object | Required if `kind = individual`. See §5.2. |

### 3.7 Editorial metadata (`post_meta`)

| Field | Req | Type | Shape / rules |
|---|---|---|---|
| `meta.kicker` | Y | string | 1–4 words. Topic label above the title ("Roads", "Housing", "Spotlight"). |
| `meta.byline` | Y | string | Who wrote/produced the source content ("MnDOT District 1", "@foodshelfmn", "Root Editorial Staff"). |
| `meta.deck` | C | string | 1–2 sentence standfirst. Required for `weight = heavy`. 120–240 chars. |
| `meta.pull_quote` | N | string | 1 sentence, 60–180 chars. Used by feature-story templates. |
| `meta.timestamp` | N | timestamptz | Override for display timestamp. Defaults to `published_at`. |
| `meta.updated` | N | string | "Updated: April 3" — freeform string shown when the post has been revised. |

### 3.8 Field groups (see §7 for which are required per post_type)

| Field group | Table | Cardinality | Used by post_types |
|---|---|---|---|
| `datetime` | `post_datetime` | 0..1 | `event` (required), `action` (deadline-linked) |
| `schedule` | `post_schedule` | 0..N | `reference`, `business` (operating hours), `need`/`aid` (volunteer windows) |
| `person` | `post_person` | 0..1 | `person` (required), occasionally `story` (when profiling someone) |
| `items` | `post_items` | 0..N | `need`, `aid` (item lists), `reference` (directory entries) |
| `contacts` | `post_contacts` | 0..N | All resource-oriented types benefit. Required for `reference` and `business`. |
| `link` | `post_link` | 0..1 | `action` (required — the CTA), `event` (register link), `reference` (more-info link) |
| `media` | `post_media` | 0..N | All types. At least one hero image recommended for `story`/`person`/`business`/heavy weights. |
| `status` | `post_status` | 0..1 | `need`/`aid` (`open` \| `closed`). |

### 3.9 Lifecycle & trace

| Field | Req | Type | Shape / rules |
|---|---|---|---|
| `submission_type` | Y | enum | Signal sets `ingested`. Editor submissions use `admin` / `reader_submitted` / `org_submitted`. |
| `is_urgent` | N | bool | **Editor-only.** Signal must not set. |
| `pencil_mark` | N | enum | **Editor-only.** `star` \| `heart` \| `smile` \| `circle` \| null. |
| `revision_of_post_id` | N | uuid | For corrections — see §11. |
| `duplicate_of_id` | N | uuid | Set by editors after merge review. Signal should set when it detects a near-duplicate of a known post. |

---

## 4. Body length targets

### 4.1 Three tiers + body_raw

| Tier | Column | Min | Target | Max | Used by templates |
|---|---|---|---|---|---|
| Body (full) | `body_raw` | **250** | 500–1500 | — | The post detail page. Always full text. |
| Heavy | `body_heavy` | 600 | 800 | 1400 | feature (2-col hero) |
| Mid-heavy | `body_heavy` | 250 | 400 | 600 | feature-reversed, generous-exchange |
| Medium | `body_medium` | 150 | 200 | 280 | gazette, bulletin, spotlight-local, alert-notice |
| Light | `body_light` | 40 | 60 | 120 | digest, ledger, quick-ref, whisper-notice |
| Ticker | `body_light` | 30 | 45 | 80 | ticker, ticker-update |

**The 250-char floor on `body_raw` applies to every post regardless of weight.** A `light` post appears on the broadsheet as a one-liner, but its detail page is a full article. A one-sentence post is an incomplete post. This is the single most common gap in the current seed.

### 4.2 Why all three tiers, not just one

The layout engine assigns post templates based on `weight` × `post_type` compatibility and the shape of the row it's filling. The same post may be placed into a 2-col feature slot on one edition (needs `body_heavy`) or a single-line ticker slot on another (needs `body_light`). Signal cannot know which slot ahead of time. **Produce all three tiers that are required for the weight, plus `body_raw` for the detail page, every time.**

### 4.3 Characters, not words

Target numbers above are character counts including spaces and punctuation. A typical English sentence averages 100–120 chars.

### 4.4 Priority scoring rubric

| Range | Meaning | Typical use |
|---|---|---|
| 90–100 | Breaking / urgent | Active emergencies, shelter openings, imminent deadlines |
| 70–89 | High importance | Major new programs, time-sensitive events, significant local news |
| 50–69 | Standard | Ongoing resources, regular events, routine updates |
| 30–49 | Lower priority | Reference listings, evergreen content |
| 0–29 | Filler | Brief, non-urgent, low editorial value |

Priority is Signal's recommendation. The layout engine uses it for ordering within a weight class; editors can override.

### 4.5 Weight distribution expectations

Target mix across a weekly pool, per county:

| Weight | % of posts | Editorial role |
|---|---|---|
| `heavy` | 10–20% | Above-the-fold features. 1–3 per county per week. |
| `medium` | 40–60% | Core reporting. The bulk of the paper. |
| `light` | 30–50% | Tickers, briefs, classifieds. |

A batch that's 100% medium produces a wall of identical gazette cards with no visual pacing. Signal should consciously vary weight across the batch.

---

## 5. Sources

Every non-editorial post has a source. Two kinds; the data model handles both.

### 5.1 Organization sources

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

**Dedup rules on ingest:**

1. If `already_known_org_id` is set and resolves to an existing `organizations` row, use it. Signal sets this when it has high confidence from prior ingests.
2. Otherwise, look up by `website` domain (case-insensitive, strip `www.` and trailing slash). If a row matches, link to it and enrich any NULL fields the submission fills in (phone, address, etc.).
3. Otherwise, look up by exact `name`. If a row matches, link. Enrich NULLs.
4. Otherwise, insert a new `organizations` row and link.

**"Up to date" handling.** On every match, if the submission carries newer metadata than the stored row (e.g., a changed website), the CMS flags the post for editor review with a `source_stale` notice rather than silently overwriting. Editors decide.

The post gets linked to the organization via `post_sources → sources → organizations`. Note: `posts.organization_id` as a direct FK was dropped in migration 122; resolution happens through the source graph so the same link model works for multi-source posts (e.g., a story carried on the org's website AND their Instagram — two `post_sources` rows, one organization). If editorial friction emerges from the join, re-adding `posts.organization_id` is a known future simplification.

### 5.2 Individual sources

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

Individuals live in a parallel `source_individuals` table (to be added — see §12 open questions). They're not people-subjects-of-a-post (that's `post_person`); they're people-who-originated-a-post.

**Consent is load-bearing.** `consent_to_publish = false` means Signal saw the material but has no consent signal; the post ingests as `status = draft` and sits in an editor queue until consent is confirmed or the post is rejected. Signal must not ingest individual-sourced posts with `consent_to_publish = true` without a real basis.

### 5.3 Editorial (origin = editor)

Used for the <1% of posts an admin writes by hand. `source.kind = editorial`; no external URL; `submission_type = admin`. Byline may be "Root Editorial Staff" or the editor's name.

---

## 6. Service area tagging

Every post needs at least one `service_area` tag. Three valid patterns:

1. **County-specific:** `hennepin-county`, `aitkin-county`, etc. Slug = `counties.name` lowercased + kebab-case, with `.` stripped (e.g., `"Lac qui Parle"` → `lac-qui-parle-county`, `"St. Louis"` → `st-louis-county`). Full list in `data/tags.json`.
2. **Statewide:** `statewide` — relevant everywhere in MN.
3. **Multiple counties:** e.g., `["hennepin-county", "ramsey-county", "dakota-county"]` for metro-wide content.

Posts missing this tag are eligible only when they also have no location data (and even then, the layout engine mostly ignores them). Always provide at least one.

---

## 7. Field-group requirements by post_type

Each post_type has a *minimum shape*. Below that shape, the detail page looks broken and Signal should reject the post rather than ingest it.

| post_type | Required beyond §3 core | Recommended | Rendering notes |
|---|---|---|---|
| `story` | `meta.deck`, `meta.byline`, at least one media item if `weight=heavy` | `meta.pull_quote`, `source_attribution`, related post references | Feature + gazette templates lean heavily on deck + pull quote. |
| `update` | `contacts` OR `link` (reader needs a way to act) | `datetime` if dated, media | "News you should know" — at minimum point the reader somewhere. |
| `action` | `link` (the CTA itself; `link.deadline` if time-bound) | `contacts`, `datetime` | Without `link`, this post has nothing to do. |
| `event` | `datetime`, `location`, `contacts` OR `link` for RSVP | `schedule` for recurring, media | Calendar-pickable; needs start+end at minimum. |
| `need` | `items` (what's needed), `contacts`, `status` (`open`/`closed`) | `schedule` for drop-off windows, `link` | Asks need to be actionable. |
| `aid` | `items` (what's available), `contacts`, `status` | `schedule`, `link` | Offers need to be contactable. |
| `person` | `person` field group (`name`, `role`, `bio`, `quote`), media | Related posts, source_attribution | Spotlights are person-first; the group is load-bearing. |
| `business` | `contacts`, `schedule` (operating hours), `location` | Media, `link` to business site | Standing listings. Mark `is_evergreen=true`. |
| `reference` | `items` (directory entries), `contacts` for each entry | `schedule`, `link` per entry | The resource list IS the post. Mark `is_evergreen=true`. |

**If a required field group is missing, Signal must either populate it before ingest or downgrade the post_type.** Example: an `event` without a `datetime` is not an event; re-classify as `update` and ingest.

---

## 8. Media

Media is a separate ingestion pipeline with its own deferred design: `docs/guides/ROOT_SIGNAL_MEDIA_INGEST.md`. Until that lands:

- Signal submits `field_groups.media[]` entries with `source_image_url`, `caption`, `credit`, `alt_text`.
- The CMS does *not* currently download images. `image_url` on the post points at the external source.
- When the media pipeline ships, the ingestion path will download, hash, store in MinIO, and link via `media.id` on `post_media`. The contract above changes only in that `image_url` becomes a Root Editorial-hosted URL; the JSON shape doesn't change.

For now: Signal produces external URLs; editors upload internal replacements through the admin Media Library on a best-effort basis.

---

## 9. Validation rules

The `POST /Posts/create_post` endpoint runs these checks. Failures return 422 with a structured error list.

**Hard failures (post rejected):**
- Missing any field marked **Y** in §3.
- `body_raw` < 250 chars.
- `body_heavy` missing when `weight = heavy`, or < 600 chars.
- `body_medium` missing when `weight ∈ {heavy, medium}`, or < 150 chars.
- `body_light` < 30 chars or > 200 chars.
- Unknown `post_type`.
- Zero `service_area` tags.
- `is_urgent` or `pencil_mark` set (editor-only).
- `source.kind = individual` with `consent_to_publish = true` but no `platform_url`.
- Post-type-required field groups missing (per §7). Example: `event` with no `datetime`.

**Soft failures (post accepted, flagged for editor review):**
- `extraction_confidence < 60`.
- `source.kind = organization` where dedup found a name match but not a website match (possible bad match).
- Known duplicate detected (`duplicate_of_id` set by Signal; editor confirms merge).
- `meta.deck` missing on a `heavy` post (required but easy to generate).

Flagged posts land with `status = in_review` instead of `active`, and the edition generator excludes them until an editor acts.

---

## 10. Editor-only fields

These fields belong to Root Editorial. Signal must not set them, and ingest rejects attempts to set them. The CMS UI is the only way they change.

- `status` (transitions: `active` → `filled` / `rejected` / `expired` / `archived`; `in_review` → `active` / `rejected`)
- `is_urgent`
- `pencil_mark`
- Edition slotting (`edition_slots` rows — how a post is placed in a broadsheet)
- Section assignment (topic groupings within an edition)

---

## 11. Revisions, corrections, and deduplication

### 11.1 Corrections (revisions)

When Signal needs to update an existing post — source content changed, a correction was issued, a fact was wrong — it submits a new post with `revision_of_post_id` pointing at the original. The CMS:

1. Creates the new post row.
2. Marks the previous row's `status = archived` and sets its `revision_of_post_id` chain so `/admin/posts/{id}` can show the revision history.
3. Re-runs edition slotting if the previous post was slotted in an active edition.

Signal should issue revisions for material changes only; typo fixes belong to editors.

### 11.2 Duplicates

Signal ingests from many sources; the same event can surface from a county press release, an org's Instagram, and a reader submission within hours. Signal sets `duplicate_of_id` when it detects a near-duplicate against the post history. An editor confirms the merge in the admin (taking the best content from each and retiring the others).

If Signal is unsure, leave `duplicate_of_id` null; the admin has a "find duplicates" tool editors can run.

### 11.3 Translations

`translation_of_id` points at the canonical-language version. `source_language` on each row identifies the language. Translations are a future feature; Signal should not attempt translations until the spec lands.

---

## 12. Open questions

Carried over from the prior specs and updated for this scope.

1. **Individual source table.** §5.2 assumes `source_individuals` exists. It does not yet. Design an individual source model (schema, dedup by platform handle, consent tracking). Track as a standalone ticket before Signal ingests individual-sourced posts.
2. **Cadence.** Weekly batch, continuous stream, or real-time webhook? Affects rate limiting and de-dup windows.
3. **Extraction-confidence threshold for soft-failures.** Is 60 the right floor, or does it vary by post_type?
4. **Multi-county scope.** Single post with multiple `service_area` tags, or metro-region slugs like `metro-east`? Current answer: multiple tags; revisit if tag-spray becomes unwieldy.
5. **Image licensing.** The media pipeline (§8) needs a policy on fair use, credit propagation, and hotlink-vs-upload. Punted to the media ingest doc.
6. **Byline vs source attribution.** When do these diverge? Current stance: byline = who wrote it; attribution = who we credit for making the material available. Usually identical for orgs; diverge for aggregated content.
7. **Priority ownership.** Signal assigns initial priority; editors can override. Should edits propagate back to Signal as training signal, or stay local?

---

## 13. Changelog

- **2026-04-19** — Initial authoritative draft. Merges ROOT_SIGNAL_SPEC.md + ROOT_SIGNAL_INGEST_SPEC.md; adds individual sources, organization dedup rules, field-group requirements per post_type, validation flow, and tightened body_raw floor (250 chars on all weights).
- **2026-03-10** — ROOT_SIGNAL_SPEC.md (superseded).
- **(undated)** — ROOT_SIGNAL_INGEST_SPEC.md draft (superseded).

---

## Related docs

- Seed data enrichment plan: `docs/guides/SEED_DATA_ENRICHMENT_PLAN.md`
- Media ingest (deferred): `docs/guides/ROOT_SIGNAL_MEDIA_INGEST.md`
- Layout engine internals: `packages/server/src/domains/editions/activities/layout_engine.rs`
- Post schema: `packages/server/migrations/000216_expand_post_types.sql`, `000221_evergreen_and_height_overrides.sql`, `000231_media_references_and_media_fks.sql`
