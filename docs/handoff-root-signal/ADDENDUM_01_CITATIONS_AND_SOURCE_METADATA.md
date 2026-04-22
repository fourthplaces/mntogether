# Addendum 01 — Citations and Source Metadata

**Status:** Addendum to [`ROOT_SIGNAL_API_REQUEST.md`](ROOT_SIGNAL_API_REQUEST.md). Additive only — does not replace or modify any section of the original spec.
**Scope:** Extends the source model to support multiple citations per post with richer per-citation metadata. Clarifies rendering-path assumptions that were implicit in the original spec.

---

## Why this addendum exists

A post-handoff code audit surfaced two real gaps in the original spec's treatment of sources:

1. **The original spec has one source per post.** Editorial's `post_sources` table supports 0..N sources per post natively (polymorphic, keyed on `(post_id, source_type, source_id)` — see `packages/server/migrations/000148_create_post_sources.sql`). Root Signal's own `Citation` / `Evidence` model (from `rootsignal-common/src/types.rs:356-368`) carries multiple citations per Signal, with richer metadata than our envelope accepts. Flattening to one source loses information both sides support natively.

2. **The original spec doesn't transmit source metadata beyond URL + attribution line.** Root Signal computes `retrieved_at`, `content_hash`, `snippet`, and `confidence` per citation — all useful on Editorial's side (dedup, audit trail, editor verification) and all currently discarded.

Editors need to be able to click open a post in the CMS and see every source that initiated it — not just the one we picked as primary. That use case is what this addendum enables.

---

## 1. Optional `citations[]` field on the envelope

Add a new optional top-level field to the submission envelope (see original §4):

```json
{
  "title": "…",
  "post_type": "…",
  …
  "source": { … },          // Unchanged. Still required. Represents the primary citation.
  "citations": [ … ],       // NEW. Optional. Full list of citations; primary is additionally in `source`.
  …
}
```

**When to use `citations[]`:**

- The post is corroborated by multiple distinct sources (e.g., a county press release + an agency Instagram post + a local reporter's article about the same event).
- Root Signal wants to transmit per-citation metadata (`retrieved_at`, `content_hash`, `snippet`, `confidence`) beyond what the singular `source` field carries.

**When to omit it:**

- Post has a single source. Omit `citations[]` entirely. The existing `source` object is sufficient and Editorial treats it as the post's sole citation.
- Do not emit `citations: []` (empty array) or `citations: [source]` (one-item redundant echo). Either omit the field or populate it with two or more citations.

**Maximum length:** 10 citations per post. Submissions with more are hard-failed with `too_many_citations` (a new §11.3 error code).

---

## 2. Citation object shape

Each entry in `citations[]`:

```json
{
  "source_url": "https://www.hennepin.us/transportation/public-input/…",
  "retrieved_at": "2026-04-22T14:05:00Z",
  "content_hash": "sha256:8f3a9c2b7e1d4f6a5c8b9e2d7f3a6c1b…",
  "snippet": "Hennepin County is accepting public comment on its 2027–2030 transit investment framework through 4:30 PM Friday.",
  "confidence": 92,
  "is_primary": true,
  "kind": "organization",
  "organization": {
    "name": "Hennepin County Transportation Department",
    "website": "https://www.hennepin.us/transportation",
    "already_known_org_id": null,
    "instagram_handle": null,
    "twitter_handle": "HennepinCounty",
    "facebook_handle": "HennepinCounty",
    "address": null,
    "phone": null,
    "populations_served": null
  },
  "individual": null,
  "platform_context": {
    "platform": "web",
    "platform_id": null
  }
}
```

Field-by-field (all **Req** flags use the same Y/C/N convention as the original spec):

| Field | Req | Type | Rule |
|---|---|---|---|
| `source_url` | Y | string | Canonical URL where this citation was fetched. HTTPS. Same rules as the existing `source.source_url` (§7). Editorial uses it for dedup keying and audit trail. |
| `retrieved_at` | Y | timestamptz | When Root Signal last fetched the URL (UTC, ISO 8601). Editorial uses this to detect stale citations and expire re-check cadence. |
| `content_hash` | Y | string | `sha256:<hex>` digest of the fetched body (HTML or normalised text — Root Signal's choice, just be consistent per-URL). Editorial uses this for change detection: matching hash across submissions means the source hasn't changed; differing hash means re-scrape. |
| `snippet` | N | string | 50–300 chars of extracted text. Shown to editors on the post's sources panel as "what Root Signal saw when it extracted." Helps editors verify the citation is relevant. |
| `confidence` | N | int 0–100 | Root Signal's confidence that this citation is relevant to the post. Used for primary-source selection when `is_primary` is not set on any entry (Editorial picks the highest). |
| `is_primary` | N | bool | Set on exactly one citation in the array; if set, Editorial uses that entry as the primary for `post_source_attribution`. If unset on all entries, Editorial picks by `confidence` descending, then by array order. |
| `kind` | Y | enum | `organization` \| `individual`. Editorial's `kind = "editorial"` is not valid here (same rule as original §7.3). |
| `organization` | C | object | Required if `kind = "organization"`. Same shape as original `source.organization`. |
| `individual` | C | object | Required if `kind = "individual"`. Same shape as original `source.individual`. |
| `platform_context` | N | object | Per-platform identifiers that aren't captured by URL alone (e.g., Instagram post IDs). See §3 below. |

### 2.1 The relationship between `source` and `citations[]`

When both are present:

- `source` MUST equal exactly one of the entries in `citations[]` — specifically, the one with `is_primary: true` (or the first entry if none is marked primary).
- Editorial validates this equivalence on ingest: the `source_url` in `source` must match the `source_url` of the primary citation. Mismatch hard-fails with `citation_primary_mismatch`.
- This redundancy is intentional: tools that only read the singular `source` field (including editors glancing at the envelope) see the primary citation without having to parse `citations[]`.

When `citations[]` is omitted: the single `source` object is processed exactly as specified in the original §7. No new behaviour.

---

## 3. Platform-specific context

Some citations carry identifiers that aren't captured by the URL alone:

- Instagram posts have a post shortcode (`/p/<code>/`) that's extractable from the URL, but reels, stories, and saved carousels also have separate IDs.
- Facebook events, posts, reels each have their own numeric ID schemes.
- TikTok videos have `video_id` distinct from the URL path.
- Newspaper articles sometimes have DOI / article UUIDs useful for persistence.

The optional `platform_context` object on a citation carries these:

```json
"platform_context": {
  "platform": "instagram",        // Required if platform_context is present. Matches `individual.platform` enum.
  "platform_id": "Cxy_abc123",    // Optional. The platform's native post identifier.
  "post_type_hint": "reel"        // Optional. "post" | "reel" | "story" | "video" | "article" | "event" | etc.
}
```

Editorial stores these alongside the citation. Not rendered on the public post detail page; surfaced in the admin sources panel for editor verification.

Omit `platform_context` entirely for web citations where the URL is the complete identifier.

---

## 4. Editorial-side behaviour

### 4.1 What happens on ingest

For each citation (or just the single `source` if `citations[]` is omitted):

1. Run the dedup ladder on `organization` or `individual` (unchanged from §7.1 / §7.2 of the original spec).
2. Create one `post_sources` row: `(post_id, source_type, source_id, source_url, first_seen_at = retrieved_at, last_seen_at = retrieved_at, content_hash, snippet, confidence, platform_id, platform_post_type_hint)`. The `content_hash`, `snippet`, `confidence`, `platform_id`, `platform_post_type_hint` columns are new and ship alongside the citations feature on Editorial's side.
3. If the citation is primary (either `is_primary: true` or Editorial-picked): populate `post_source_attribution` from its `organization.name` / `individual.display_name` + `source.attribution_line` from the envelope.

### 4.2 Response shape additions

On 201 success, Editorial's response (original §11.1) gains a `citation_ids` field when `citations[]` was submitted:

```json
{
  "post_id": "e8c9d1a4-…",
  "status": "active",
  "organization_id": "f1a2b3c4-…",
  "individual_id": null,
  "citation_ids": [
    "c1111111-1111-1111-1111-111111111111",
    "c2222222-2222-2222-2222-222222222222",
    "c3333333-3333-3333-3333-333333333333"
  ],
  "idempotency_key_seen_before": false
}
```

`citation_ids` are the UUIDs Editorial assigned to the new `post_sources` rows, in the same order as the submitted `citations[]`. Root Signal stores these if it wants to reference individual citations on future operations (e.g., marking one as stale in a revision). When `citations[]` is omitted, `citation_ids` is omitted from the response.

### 4.3 What editors see

Editorial's admin post detail page gains a **Sources panel** listing every `post_sources` row for the post: organisation/individual name, URL, retrieved_at, snippet, confidence, platform context. One row per citation. Editors can click through to the source URL. The primary citation is visually distinguished (used for public attribution); editors can change the primary assignment via a UI control.

Public-site post detail pages render the primary citation's `attribution_line` as they always have. Multi-source posts do not fan out citations to the public page by default; editors can opt into an "all sources" footnote on a per-post basis via an admin toggle.

---

## 5. Revisions and citations

Expanding on §12.1 of the original spec:

When Root Signal submits a revision (`editorial.revision_of_post_id` set), `citations[]` is processed per the normal flow — each citation becomes a new `post_sources` row on the revised post. Citations are **not** automatically copied from the prior post; Root Signal should emit the full citation set for the revised post, including any unchanged ones.

If a citation has the same `content_hash` as one attached to the prior post, Editorial treats the underlying source as unchanged and updates only `last_seen_at`. This lets Root Signal express "we re-verified these sources and nothing changed about them" cheaply.

---

## 6. Validation

New hard-failure error codes added to §11.3:

| Code | Meaning |
|---|---|
| `too_many_citations` | `citations[]` length > 10. |
| `citation_primary_mismatch` | `source.source_url` does not match the `source_url` of the primary citation in `citations[]`. |
| `citation_hash_format` | `content_hash` is not in the form `sha256:<64-hex-chars>`. |
| `citation_missing_required` | A citation entry is missing one of the Y fields (`source_url`, `retrieved_at`, `content_hash`, `kind`, plus `organization` or `individual` per `kind`). |
| `citation_editorial_forbidden` | Citation with `kind = "editorial"`. Editorial-origin posts are not ingested; same rule as original §7.3. |
| `invalid_retrieved_at` | `retrieved_at` is not valid ISO 8601 or is in the future. |

No soft-failure changes specific to citations in this addendum.

---

## 7. Complete worked example

An event post cited from three sources: the county's official press release, the county's Instagram, and a reporter's article:

```json
{
  "title": "Hennepin County Transit Plan Public Hearing Thursday",
  "post_type": "event",
  "weight": "medium",
  "priority": 78,
  "body_raw": "Hennepin County holds a public hearing on its 2027–2030 transit investment framework Thursday evening at the Government Center. County staff will present the draft, then take public comment on the record. Written comments carry the same weight as verbal and are accepted through Friday 4:30 PM via the public-input portal or by email to the Transportation Department. The plan proposes new bus-rapid-transit corridors along Nicollet, Central, and West Broadway, and adjusts frequency on several suburban routes. Rider feedback on the frequency changes and resident feedback from neighbourhoods along the new BRT corridors is specifically requested. Transit advocates have publicised the hearing on multiple platforms; the county's own press office and Transportation Commissioner have posted directly; and the Star Tribune's transportation reporter has published a breakdown of what the plan changes from the prior cycle.",
  "body_medium": "Hennepin County holds a public hearing Thursday 6:30 PM at the Government Center on its 2027–2030 transit investment framework. Proposed BRT lines on Nicollet, Central, West Broadway. Written comments open through Friday 4:30 PM.",
  "body_light": "Transit plan hearing Thu 6:30 PM, Gov Ctr. Comments close Fri 4:30.",
  "published_at": "2026-04-20T08:00:00-05:00",
  "source_language": "en",
  "is_evergreen": false,
  "location": "Minneapolis, MN",
  "zip_code": "55487",
  "latitude": 44.9774,
  "longitude": -93.2658,
  "tags": {
    "service_area": ["hennepin-county"],
    "topic": ["transit", "voting"],
    "safety": []
  },
  "source": {
    "kind": "organization",
    "organization": {
      "name": "Hennepin County Transportation Department",
      "website": "https://www.hennepin.us/transportation",
      "twitter_handle": "HennepinCounty",
      "facebook_handle": "HennepinCounty"
    },
    "source_url": "https://www.hennepin.us/transportation/public-input/transit-framework-2027-2030",
    "attribution_line": "Hennepin County public-input portal",
    "extraction_confidence": 95
  },
  "citations": [
    {
      "source_url": "https://www.hennepin.us/transportation/public-input/transit-framework-2027-2030",
      "retrieved_at": "2026-04-20T09:00:00Z",
      "content_hash": "sha256:8f3a9c2b7e1d4f6a5c8b9e2d7f3a6c1b2d8e3c4b5a9f7e6d1c4b8a3e2f5d9c7b",
      "snippet": "Hennepin County is accepting public comment on its 2027–2030 transit investment framework through Friday 4:30 PM.",
      "confidence": 95,
      "is_primary": true,
      "kind": "organization",
      "organization": {
        "name": "Hennepin County Transportation Department",
        "website": "https://www.hennepin.us/transportation",
        "twitter_handle": "HennepinCounty",
        "facebook_handle": "HennepinCounty"
      }
    },
    {
      "source_url": "https://instagram.com/p/Cxy_abc123/",
      "retrieved_at": "2026-04-21T15:30:00Z",
      "content_hash": "sha256:2c7e5a8f1d9b3c6e4a7f2d8b1c5e9a3f7b4d2c8e5a9f1b6d3c7e4a2f8b5d1c9e",
      "snippet": "The county transit framework hearing is Thursday evening. Public comment matters — show up or submit online by Friday. Link in bio.",
      "confidence": 88,
      "kind": "organization",
      "organization": {
        "name": "Hennepin County Transportation Department",
        "website": "https://www.hennepin.us/transportation",
        "instagram_handle": "hennepincounty_official"
      },
      "platform_context": {
        "platform": "instagram",
        "platform_id": "Cxy_abc123",
        "post_type_hint": "post"
      }
    },
    {
      "source_url": "https://www.startribune.com/hennepin-transit-framework-2027/601234567",
      "retrieved_at": "2026-04-21T17:05:00Z",
      "content_hash": "sha256:4d9b3c6e1a2f8b5d7c9e4a2f8b1c5e9a3f7b4d2c8e5a9f1b6d3c7e4a2f8b5d1c",
      "snippet": "The plan's most consequential shift is deprioritising suburban routes with low ridership in favor of new urban BRT lines — a trade-off that's already drawing criticism from outlying communities.",
      "confidence": 82,
      "kind": "individual",
      "individual": {
        "display_name": "Sarah Chen",
        "handle": "sarachen",
        "platform": "other",
        "platform_url": "https://www.startribune.com/staff/sarah-chen",
        "verified_identity": true,
        "consent_to_publish": true
      },
      "platform_context": {
        "platform": "other",
        "post_type_hint": "article"
      }
    }
  ],
  "meta": {
    "kicker": "Civic Action",
    "byline": "Hennepin County"
  },
  "field_groups": {
    "datetime": {
      "start_at": "2026-04-23T18:30:00-05:00",
      "end_at": "2026-04-23T20:30:00-05:00",
      "cost": "Free",
      "recurring": false
    },
    "link": {
      "label": "Submit comment online",
      "url": "https://www.hennepin.us/transportation/public-input/transit-framework-2027-2030/comment",
      "deadline": "2026-04-24T16:30:00-05:00"
    },
    "contacts": [
      { "contact_type": "email", "contact_value": "transportation@hennepin.us", "contact_label": "Email comment" }
    ],
    "schedule": [],
    "person": null,
    "items": [],
    "media": [],
    "status": null
  },
  "editorial": { "revision_of_post_id": null, "duplicate_of_id": null }
}
```

---

## 8. What this addendum does NOT change

- The singular `source` field remains required on every submission. Root Signal cannot submit a post with `citations[]` but no `source`.
- The 9 post types, body-tier requirements, tag vocabularies, validation rules, auth, idempotency, revision handling, and all worked examples in the original spec are unchanged.
- Submissions that use the original `source`-only shape (no `citations[]`) are accepted exactly as before.
- The `source.kind = "editorial"` rejection still applies; citations must be organization or individual.

---

## 9. Out of scope for this addendum

These are notes to the Root Signal team about Editorial-side work that this addendum implies but does not specify, so there's no ambiguity about what Root Signal is being asked to build:

- **Editorial builds the admin sources panel** to surface all citations to editors. Not a Root Signal concern.
- **Editorial adds the `content_hash`, `snippet`, `confidence`, `platform_id`, `platform_post_type_hint` columns to `post_sources`.** Not a Root Signal concern.
- **Editorial extends its dedup to incorporate `content_hash`** as a secondary dedup signal. Not a Root Signal concern.
- **Editorial exposes citations on the GraphQL Post type** for the admin UI. Not a Root Signal concern.

---

## 10. Post-handoff audit notes

This addendum was produced after a code-level audit that caught two classes of things worth flagging for the Root Signal team:

1. **Gaps the original spec genuinely had.** The citations work above is the real answer to one of them.
2. **Agent-surfaced findings that turned out to be wrong.** Several fields were reported as "missing from the schema" during the audit and later confirmed to be present — `meta.pull_quote` (added in migration `000216:82`), `post_person.photo_media_id` (added in migration `000231:80`). Those aren't gaps. Calling them out here so they don't surface again in your review.

Two Editorial-internal inconsistencies were also found and will be addressed on our side before Root Signal's implementation lands; not part of this addendum or the original spec but worth knowing:

- The GraphQL `Post.sourceUrl` field points at a column (`posts.source_url`) that was dropped in migration `000213`. A resolver fix reads it from `post_sources[0].source_url` instead. Does not affect Root Signal submissions.
- A mid-tree migration (`000197`) partially reworked the tag taxonomy (introduced `county` / `city` / `language` / `platform` / `verification` kinds, marked `service_area` for deletion) but the runtime code (layout engine, seed) still uses `service_area`. `service_area` is what Root Signal should use on submissions, as specified in the original spec. The internal cleanup is on Editorial's TODO; Root Signal's contract is unaffected.
