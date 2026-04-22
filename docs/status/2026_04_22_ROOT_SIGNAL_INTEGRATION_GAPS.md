# Root Signal Integration Gaps — 2026-04-22

**Status:** Internal punch list. Companion to [`docs/handoff-root-signal/ROOT_SIGNAL_API_REQUEST.md`](../handoff-root-signal/ROOT_SIGNAL_API_REQUEST.md).
**Audit date:** 2026-04-22.
**Audit scope:** Full read of `mntogether` docs + `packages/server/src/` + migrations 000001–000236 + `data/seed.mjs` + `data/audit-seed.mjs`, plus `rootsignal` repo on `dev` branch (head `7ffd18e0`).

This document enumerates every gap between the published contract (`docs/architecture/ROOT_SIGNAL_DATA_CONTRACT.md`) and what is currently implemented. It is the execution punch list that the request document hands off to engineering.

---

## Quick summary

**Critical path** (all Editorial-side; must land before Root Signal returns with the built integration):

- Ingest endpoint + `ServiceClient` auth + structured 422 + idempotency + org dedup + individual dedup (`source_individuals` table) + editor-only field rejection + tag resolution + `service_areas`/`post_locations` population
- Signal Inbox admin UI
- Server-side media pipeline (fetch, EXIF strip, WebP, content-hash, MinIO → `post_media.media_id`)
- Content-hash dedup on `posts.content_hash`
- Revision auto-reflow of active editions
- `[signal:UUID]` citation rendering in body tiers
- Tag vocabulary cleanup: drop `population` kind entirely, rework `safety` as access-policy modifiers, clean topic kind of mixed-in neighborhood slugs, normalise safety to hyphen-case
- `tags.kind` CHECK constraint tightened to `('topic', 'service_area', 'safety', 'neighborhood')`

**Out of scope** (not building): HMAC body signing (Bearer + HTTPS is sufficient), feedback webhook (Editorial → Root Signal lifecycle events), per-day rate-limit quotas, Job post_type, Condition post_type.

> **2026-04-22 alignment.** The handoff package (`docs/handoff-root-signal/`) is written as a specification to Root Signal with every Editorial-side commitment presented as in place. This doc tracks the actual build work so reality matches the spec by the time Root Signal returns.

---

## 1. Editorial-side gaps

### 1.1 P0 — No contract-compliant `POST /Posts/create_post` handler

**Evidence.** The route is wired up at `packages/server/src/api/routes/posts.rs:2452`, but the handler at `:1513-1532` is an `AdminUser`-gated stub that accepts a 7-field `CreatePostRequest` (`:145-153`):

```rust
pub struct CreatePostRequest {
    pub title: String,
    pub body_raw: String,
    pub post_type: Option<String>,
    pub weight: Option<String>,
    pub priority: Option<i32>,
    pub is_urgent: Option<bool>,
    pub location: Option<String>,
}
```

Missing: everything else in the envelope — body tiers, source, tags, field groups, meta, editorial/revisions, extraction_confidence, published_at, and so on.

**What's needed.** A new handler (or a materially expanded existing one) that:
- Accepts the full envelope per `docs/architecture/ROOT_SIGNAL_DATA_CONTRACT.md` §2.
- Requires a new `ServiceClient` auth extractor (not `AdminUser`).
- Implements hard-fail / soft-fail validation per §9.
- Returns 201 with structured result, or 422 with structured error list.
- Runs the organisation dedup ladder per §5.1.
- Populates all field-group tables via CTE chain (pattern already used by `seed.mjs`).
- Honours `X-Idempotency-Key` header.

**Effort.** 2 weeks — handler, request/response types, validation, activities, dedup, tests.

**Tracked at.** `docs/TODO.md` #1.

---

### 1.2 P0 — No machine-token auth extractor

**Evidence.** `packages/server/src/api/auth.rs` defines only `AuthenticatedUser`, `AdminUser`, `OptionalUser`. All three look for a human-session JWT in `X-User-Token` or `Authorization: Bearer`. There is no API-key / service-credential path.

**What's needed.** A `ServiceClient` extractor:
- Validates `Authorization: Bearer rsk_live_<token>`.
- Looks up the key in a new `api_keys` table (columns: `id`, `prefix`, `hash`, `scopes`, `created_at`, `rotated_from_id`, `revoked_at`, `last_used_at`).
- Returns `ServiceClient { key_id, client_name, scopes }`.
- Gates the ingest endpoint on scope `posts:create`.
- Logs key_id on every request for audit.

CLI tooling for `dev-cli` to issue / rotate / revoke keys.

**Effort.** 3 days including migration + CLI.

---

### 1.3 P0 — No structured 422 response shape

**Evidence.** `packages/server/src/api/error.rs`:

```rust
pub enum ApiError {
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    BadRequest(String),
    Internal(anyhow::Error),
}
// response: {"message": "..."}
```

No 422 variant, no structured `errors[]` list. The Signal team can't programmatically act on the current error shape.

**What's needed.** Extend `ApiError` with `Validation(Vec<FieldError>)` and return the per-field error codes documented in `docs/handoff-root-signal/ROOT_SIGNAL_API_REQUEST.md` §11.3:

```rust
pub struct FieldError {
    pub field: String,   // dotted path
    pub code: String,    // stable code from taxonomy
    pub detail: String,  // human-readable
}
```

Response body:
```json
{ "message": "Validation failed", "errors": [ { "field": "body_raw", "code": "below_min_length", "detail": "..." } ] }
```

**Effort.** 1 day for the shape. The actual error generation lives inside the ingest handler (1.1).

---

### 1.4 P0 — No idempotency-key handling

**Evidence.** Grep for `idempotency` in `packages/server/` returns nothing.

**What's needed.**
- New table: `api_idempotency_keys (key_uuid UUID PRIMARY KEY, api_key_id UUID NOT NULL, request_hash TEXT NOT NULL, post_id UUID NOT NULL, created_at TIMESTAMPTZ NOT NULL, INDEX (api_key_id, created_at))`.
- Retention: 24 hours (cron / scheduled cleanup).
- On each ingest call:
  1. Compute SHA-256 of normalised request body.
  2. `SELECT` from `api_idempotency_keys` WHERE `key_uuid = $1`.
  3. If hit + `request_hash` matches: return stored `post_id`, flag `idempotency_key_seen_before = true`, do not insert.
  4. If hit + `request_hash` differs: return 409 `idempotency_conflict`.
  5. If miss: process normally, insert `(key_uuid, api_key_id, request_hash, post_id)`.

**Effort.** 2 days including migration, logic, tests.

---

### 1.5 P0 — No ingest-time organisation dedup

**Evidence.** The dedup ladder is specified in `docs/architecture/ROOT_SIGNAL_DATA_CONTRACT.md` §5.1 but the only place in code that implements it is `data/seed.mjs:276-294` — and that's a seed-specific flow that trusts `organizationName` to exist in `organizations.json`.

**What's needed.** In the ingest activity, before inserting a post:
1. If `source.organization.already_known_org_id` resolves → use it.
2. Else, normalise website (`https://...`, strip `www.`, lowercase, trim trailing `/`), look up by `organizations.website`.
3. Else, look up by exact `organizations.name`.
4. Else, insert new row.
5. On match: enrich NULL fields from submission, but flag `in_review` with `source_stale` if any non-NULL field differs from submission.

Returns `organization_id` in response.

**Effort.** 3 days. Model methods + activity + tests.

---

### 1.6 P0 — Editor-only fields not rejected on ingest path

**Evidence.** `is_urgent`, `pencil_mark`, and `status` columns exist on `posts` with no CHECK constraint preventing a Signal submission from setting them. The ingest endpoint must reject them.

**What's needed.** In the ingest handler's validation pass, reject with 422 `editor_only_field` if `is_urgent`, `pencil_mark`, or `status` is present. Enforcement at handler layer is acceptable; no DB CHECK constraint needed.

**Effort.** Rolled into 1.1.

---

### 1.7 P0 — Tag resolution on ingest

**Evidence.** `data/seed.mjs:132-156` seeds tags but ingest doesn't resolve submitted tag slugs to tag IDs.

**What's needed.** On ingest:
- For each `tags.topic[]`, `tags.service_area[]`, `tags.safety[]`:
  - Look up `tags WHERE (kind, value) = ($1, $2)`.
  - If found: insert `taggables` row.
  - If not found for `topic`: auto-create the tag with `display_name = value`, then insert `taggables`. Surface a soft notice (`in_review`) so editors can confirm the new topic into the canonical list.
  - If not found for `safety` or `service_area`: hard-fail.
  - If not found for `service_area`: hard-fail with `unknown_service_area` (these are controlled — 87 counties + `statewide`).

**Effort.** 1 day.

---

### 1.8 P0 — Service areas / post_locations tables unpopulated by ingest path

**Evidence.** Migration `000107` created `post_locations` and `service_areas` tables (with structured FKs). Seed uses tags instead; ingest contract uses tags too. The tables exist but are dead weight in the current path.

**Decision needed.** Three options:
1. Drop the tables if tags are the source of truth.
2. Populate them as secondary indices from tags.
3. Switch to them as the primary source of truth (would require schema migration + contract revision).

**Recommendation.** Option 2 in Phase 1 — populate on ingest as secondary indices, useful for future geo queries without breaking the tag-based contract. Revisit in Phase 3.

**Effort.** 1 day if Option 2. 0 days if Option 1.

---

### 1.9 P0 — `source_individuals` table does not exist

Promoted from P1 to P0 on 2026-04-22: Signal shouldn't have to gate its pipeline on our schema landing in a later phase, so the table + ingest wiring go in with the Phase 1 build.

**Evidence.** `docs/architecture/ROOT_SIGNAL_DATA_CONTRACT.md:280-298` specifies the shape. No table. Tracked at `docs/TODO.md` #2.

**What's needed.**

```sql
CREATE TABLE source_individuals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    display_name TEXT NOT NULL,
    handle TEXT,                                     -- "@jamielocal"
    platform TEXT CHECK (platform IN ('instagram','twitter','tiktok','facebook','bluesky','youtube','substack','other')),
    platform_url TEXT,
    verified_identity BOOLEAN NOT NULL DEFAULT false,
    consent_to_publish BOOLEAN NOT NULL DEFAULT false,
    consent_source TEXT,                             -- how consent was captured
    consent_captured_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (platform, handle)
);

-- Link in post_sources (new source_type):
-- post_sources.source_type = 'individual'
-- post_sources.source_id → a sources row whose type maps to an individual
```

Dedup ladder similar to orgs: by (platform, handle), then by platform_url, then create.

**Effort.** 2 days (migration + model + activity + tests).

---

### 1.10 P1 — Signal Inbox admin UI

**Evidence.** `docs/architecture/SIGNAL_INBOX.md` designs it; no UI exists. Tracked at `docs/TODO.md` #3. Without this, `in_review` posts pile up with no editor path.

**What's needed.** A `/admin/signal-inbox` page that:
- Lists posts with `status = in_review` newest first.
- Groups by flag reason (`source_stale`, `low_confidence`, `possible_duplicate`, `individual_no_consent`, `deck_missing_on_heavy`, etc.).
- Per-post actions: approve (→ `active`), reject (→ `rejected`), open-for-edit, merge (if duplicate).
- Shows extracted payload + source URL side-by-side.

**Effort.** 1 week.

---

### 1.11 P2 — Content-hash dedup

**Evidence.** Designed in `docs/guides/POST_EDITION_LIFECYCLE.md:229-295`. Not built.

**What's needed.**
- Migration adds `posts.content_hash TEXT` + index.
- Hash function: SHA-256 over `normalise(title) + '|' + normalise(source_url) + '|' + day_bucket(published_at) + '|' + service_area_slug_csv`.
- On ingest, check for existing row with matching hash. If hit: bump `published_at` (refresh eligibility), do not insert duplicate. Signal treats this as a successful submission; response carries the existing `post_id`.

**Effort.** 2 days.

---

### 1.12 P2 — Media ingest pipeline

**Evidence.** `docs/guides/ROOT_SIGNAL_MEDIA_INGEST.md` — fully designed; not built.

**What's needed.** Server-side fetch + EXIF strip + WebP + content-hash + MinIO store, with SSRF guards. Populate `post_media.media_id` to point at `media` table row; keep `image_url` for reference.

**Effort.** 2 weeks.

---

### 1.13 P3 — Feedback webhook to Signal

**Scope:** Editorial POSTs to a Signal-provided URL on events like `published`, `rejected`, `edited`, `duplicate_merged`, `correction_published`. Training signal for Root Signal's ranking/quality layer.

**Effort.** 1 week. Design with Signal.

---

### 1.14 Doc-hygiene items

1. `docs/architecture/ROOT_SIGNAL_SPEC.md` and `docs/guides/ROOT_SIGNAL_INGEST_SPEC.md` carry "superseded" banners pointing at the data contract, per `ROOT_SIGNAL_DATA_CONTRACT.md:3`. **Verify the banners are present; add if missing.**
2. `docs/architecture/POST_TYPE_SYSTEM.md` was originally written for a 6-type system; migration `000216` expanded to 9. Audit for stale 6-type references.
3. `docs/TODO.md` should reference this gaps doc + the request doc in the Active Work Queue.
4. `docs/README.md` index should link `docs/handoff-root-signal/` (done 2026-04-22).

---

## 2. Root Signal-side gaps

(Our read of what Signal would need. Subject to their correction.)

### 2.1 Outbound HTTP client against Editorial

**Status.** No outbound HTTP emitter exists — Signal's API surface is GraphQL read-only today (`modules/rootsignal-api/src/`). Subscriptions exist but are reader-pull, not producer-push.

**Shape.** ~300 LOC Rust. Reuse `reqwest`. Needs: backoff/retry, idempotency-key mint, request/response serde types, dead-letter queue for permanent failures.

### 2.2 Editorial mapping layer

**Status.** No mapper exists. Signal's `SituationNode` / `DispatchNode` / signal union types don't know about Editorial's 9-type taxonomy.

**Shape.** Table-driven mapper that converts Signal-internal types to the envelope in §4 of the request doc. Editorial judgement lives here (which Concern becomes a `story` vs an `update`, which Gathering gets heavy weight, etc.).

### 2.3 Per-type body-tier generation

**Status.** Signal generates `briefing_body` (single rich markdown with `[signal:UUID]` citations). Does not separately produce `body_heavy` / `body_medium` / `body_light`.

**Shape.** Extend the LLM pipeline to produce all three body tiers plus `body_raw` for every outbound post, respecting the length targets in the contract (§4.1).

### 2.4 Delivery tracking

**Status.** Signal doesn't track what it has sent to Editorial (because it doesn't send anything today).

**Shape.** Table `editorial_deliveries (signal_or_situation_id UUID, editorial_post_id UUID, idempotency_key UUID, status, attempts, last_attempt_at, error)`.

### 2.5 Taxonomy expansion (Profile, LocalBusiness, Opportunity, Job)

**Status.** Designed in `docs/editorial-surface-area-request.md:36-78`. Not implemented in `rootsignal-common/src/types.rs:289-354` (currently 6 types).

**Phase 1 blocker?** Partial. Without Profile + LocalBusiness, Editorial cannot receive `person` or `business` posts from Signal. Editorial's `person` and `business` post_types would stay empty until Signal adds those taxonomies.

### 2.6 Consent handling for Individual sources

**Status.** Consent-aware extraction not yet structured.

### 2.7 Signal detail URL stability

**Status.** No stable public URL for `[signal:UUID]` citation resolution today. Query layer exists (`signal(id)` GraphQL query) but no public HTML detail page.

**Needed.** A URL like `https://signal.example.com/signals/<uuid>` that renders a shareable signal detail page. Editorial's body-rendering will link to it.

### 2.8 Image URL re-hosting

**Status.** Signal's current image handling relies on source URLs that may expire (IG CDN). Until Editorial's media pipeline ships, Signal may need a short-term archival layer.

### 2.9 Source-attribution vs byline clarity

**Status.** Signal doesn't yet distinguish `byline` (who wrote it) from `attribution_line` (how we credit it). Contract §3.7 + §7.4.

---

## 3. Shared / decision-only gaps

Most of what used to live here got committed to one side or the other in the 2026-04-22 request-doc tightening. What remains:

| Gap | Resolver |
|---|---|
| Named contacts + on-call rotation for Phase 1 cutover | Exchanged at kickoff (§17 item 6 of the request doc) |
| Feedback webhook payload schema | Co-designed at the Phase 2 → Phase 3 transition (§13.2 of the request doc) |

---

## 4. Dependency graph (Phase 1)

```
┌──────────────────────────────────────────────────────────────┐
│ Push/pull decision (joint)                                   │
└──────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│ Editorial P0                                                 │
│  1.2 Machine-token auth ──► 1.1 Ingest handler ──► 1.3 422  │
│                               │      │                       │
│                               │      ├─► 1.4 Idempotency    │
│                               │      ├─► 1.5 Org dedup      │
│                               │      ├─► 1.6 Editor-only    │
│                               │      ├─► 1.7 Tag resolution │
│                               │      └─► 1.8 Service areas  │
└──────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│ Signal side                                                  │
│  2.1 Outbound client ──► 2.2 Mapper ──► 2.3 Body tiers      │
│                            │                                 │
│                            └─► 2.4 Delivery tracking         │
└──────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│ Phase 1 acceptance: 100 test posts, 10 invalid, org dedup    │
└──────────────────────────────────────────────────────────────┘
```

Phase 1 unblocks individual-source work (1.9) and the Signal Inbox (1.10), which unblock Phase 2.

---

## 5. Risk register

| Risk | Likelihood | Mitigation |
|---|---|---|
| Signal prefers pull; Editorial refactors to the pull path | Medium | §3.4 makes the push case; we do the pull spec if they push back. Added effort: ~1 week mapping layer on our side. |
| Body-tier generation on Signal's side degrades post quality vs single `briefing_body` | Medium | Review first 20 posts per type with Signal's team before Phase 1 acceptance. Tighten length targets if quality suffers. |
| Org dedup misses produce duplicate `organizations` rows | Low | Admin "merge organisations" tool already planned (TODO queue). Audit weekly for the first month. |
| `source_image_url` expiry breaks display when Phase 1 hotlinks | Medium | Accelerate Phase 3 media pipeline if we see >5% broken images after 30 days. |
| Rate limits too conservative; Signal hits 429 on legitimate traffic | Low | `Retry-After` + adjustable limit per API key. Telemetry on 429-vs-200 ratio per key. |
| Editor burnout from unchecked `in_review` queue | High if Signal Inbox (1.10) is late | Prioritise Signal Inbox concurrent with 1.1. Low-confidence threshold tuneable (§17 Q8). |
| Schema-seed drift continues (section 1.8) | Medium | Ingest-path should be the de-facto contract tester; add a daily CI job that ingests `posts.json` through the real endpoint once it exists. |

---

## 6. Acceptance criteria for closing this gaps doc

- [ ] All Editorial P0 items (1.1–1.8) merged to `main` with tests.
- [ ] `docs/handoff-root-signal/ROOT_SIGNAL_API_REQUEST.md` §17 Q1–Q4 answered by Signal.
- [ ] At least one test batch (100 real signals → 100 posts) runs end-to-end in staging.
- [ ] Runbook for API-key issuance / rotation / revocation in `docs/setup/`.
- [ ] Telemetry dashboard showing per-key request volume, 2xx/4xx/5xx split, dedup-hit rate.
- [ ] `docs/TODO.md` updated to reflect Phase 1 completion and Phase 2 as next active.

---

**Owner:** Root Editorial engineering.
**Next review:** 2026-05-06 (2 weeks out) or on Signal's response to §3 and §17 of the request doc.
