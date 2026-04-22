# Decisions Log

> Architectural decisions made during development. Captures the *why* so future sessions don't re-derive context that was expensive to reach.

> **Ordering:** Most recent sessions first. Within a session, decisions
> are grouped by topic, not chronology.

---

## 2026-04-22 — Session: Root Signal handoff package, tag-taxonomy rework, integration commitments

### Integration transport is push over HTTPS

**Decision:** Root Signal pushes fully-formed post envelopes to Editorial's `POST /Posts/create_post` endpoint — Bearer token, `X-Idempotency-Key`, JSON body, 201/422 responses. Pull (Editorial polls Root Signal's GraphQL) was considered and rejected: the Signal→Editorial mapping layer encodes editorial judgement (which signal becomes which `post_type`, how `briefing_body` maps to body tiers, which dispatches become revisions) and lives better close to source data. Push also makes validation errors synchronous at submission time rather than asynchronous dead-letter queues.

**Implication:** Root Signal owns an outbound HTTP client, retry loop, idempotency-key mint, and local delivery-tracking table. Editorial owns the ingest handler, structured 422 validation, org/individual dedup, and the `ServiceClient` auth extractor. Spec lives in `docs/handoff-root-signal/ROOT_SIGNAL_API_REQUEST.md` §3, §14.

### Individual sources are in scope from the start, not phased

**Decision:** The `source_individuals` table + consent-gated dedup ladder is built alongside the ingest endpoint — not deferred. Root Signal should not have to gate its pipeline on Editorial's schema landing "later." Individual submissions with `consent_to_publish = false` land as `in_review` until an editor clears them.

**Implication:** `source_individuals (id, display_name, handle, platform, platform_url, verified_identity, consent_to_publish, consent_source, consent_captured_at)` is part of the initial ingest build. Dedup ladder mirrors orgs: `(platform, handle)` → `platform_url` → insert. Editorial returns `individual_id` on 201 so Root Signal can persist and pass back as `already_known_individual_id`.

### Editorial processes media server-side, not hotlinked

**Decision:** Editorial fetches each `source_image_url`, validates content by magic bytes, strips EXIF, normalises to WebP, content-hashes for dedup, and stores to MinIO. `post_media.media_id` points at the internal record; the public site renders from internal URLs. An earlier plan staged this as "hotlink first, process later" — rejected because hotlink introduces URL-expiry failures and EXIF leakage from day one.

**Implication:** `source_image_url` can expire after submission without breaking the published post. Root Signal doesn't need to re-host or keep CDN URLs alive. SSRF protection (refuse localhost, private IPs, link-local, `file://`) is enforced at fetch time. Spec lives in request doc §9.

### Revisions auto-reflow affected editions

**Decision:** When Root Signal submits a post with `editorial.revision_of_post_id`, Editorial archives the prior post, chains the revision, and **automatically re-runs layout for any active edition that contained the old post.** An earlier plan had phased this as manual-first (editor clicks "Regenerate layout") with auto-reflow as follow-up — rejected because it bifurcates the editorial workflow and leaves stale posts in editions until someone clicks.

**Implication:** Revisions flow end-to-end with no editor intervention required to get the new version into the right slot. Editors see the updated layout on next open. Spec lives in request doc §12.1.

### `[signal:UUID]` citations render as inline superscripts linking to Root Signal

**Decision:** Editorial parses `[signal:UUID]` tokens in body tiers at render time, producing superscript citations with popovers linking to Root Signal's public signal detail page. Configurable via env var; falls back to unlinked superscripts if the URL pattern isn't set at deploy. An earlier plan phased this as "preserve tokens as inert text in Phase 1, render in Phase 3" — collapsed into a single committed capability.

**Implication:** Root Signal should preserve `[signal:UUID]` tokens when generating body tiers from `briefing_body`. The citation URL pattern is exchanged at integration kickoff, not at spec time. Spec lives in request doc §15.2.

### `population` tag kind dropped entirely

**Decision:** Removed `population` as a tag kind from the schema, the data contract, and the handoff docs. The kind was never seeded with a vocabulary in `data/tags.json`, and the rationale for seeding one didn't survive scrutiny: people aren't single-bucket identities. Any audience-relevance concern fits better in the open-ended `topic` tag kind where slugs can emerge organically.

**Implication:** `tags.kind` CHECK constraint tightens to `('topic', 'service_area', 'safety', 'neighborhood')`. The data contract's `tags.population` row is removed. Handoff worked examples no longer set `population: []`. Tracked in `docs/TODO.md` #1.7 as part of the tag-vocabulary cleanup.

### Safety tags are access-policy modifiers, not content descriptions

**Decision:** Reframed the `safety` tag kind from "safety/emergency flags" (which conflated content with access) to strict access-policy modifiers — policies at a service that, if unstated, would cause someone to hesitate before seeking it. The vocabulary expanded from 3 slugs to 29, grouped by category of hesitation (identity/docs, cost, privacy, procedure, cultural affirmation, accessibility, substance use, minors, law enforcement, family logistics). Slugs like `extreme-cold-shelter` — descriptions of what the service *is*, not how it's delivered — are explicitly excluded. `know-your-rights` was dropped from safety because it's a topic/content concept, not a policy modifier.

**Implication:** Safety vocabulary is closed and reserved; unknown safety slugs hard-fail on ingest. Additions propagate through the integration channel, not per-submission. Existing 3 slugs (`no_id_required`, `ice_safe`, `know_your_rights`) need normalisation to hyphen-case (`no-id-required`, `ice-safe`) and the `know-your-rights` slug migrates to topic tags. Full vocabulary in `docs/handoff-root-signal/TAG_VOCABULARY.md` §3.

### Topic vocabulary is intentionally open and meant to grow

**Decision:** The `topic` kind is open-ended vocabulary. Root Signal is instructed to **propose new slugs freely** whenever a post doesn't cleanly fit an existing one — Editorial auto-creates the tag and flags the post `in_review` for editor confirmation. Explicitly rejected an earlier handoff draft that said "Signal should not invent new topics lightly — prefer mapping to the nearest existing slug": this would have caused overfitting to Editorial's currently incomplete list and stifled legitimate vocabulary growth.

**Implication:** Topic-tag coverage gaps in the civic-content domain are expected and treated as normal product evolution, not exceptions. The current 26-slug starter list is a floor, not a ceiling. Same logic applies to `neighborhood` (once that kind is introduced).

### No legacy or retired vocabulary carried through to the handoff

**Decision:** Editorial has no real data yet — this is a clean implementation, not a live system with historical rows to preserve. Any "retired tag kind" (e.g., `reserved`, `post_type`-as-a-tag, `structure`, `audience_role`) is dropped from the schema and the docs entirely. The handoff docs contain no "historical" or "legacy" framing; what's described is what exists.

**Implication:** `tags.kind` CHECK constraint enforced on the final clean set. Seed data + the audit baseline regenerate accordingly. Handoff docs never mention "in schema but no longer assigned" — that framing was a mistake to preserve.

### Job and Condition signals have no Editorial destination

**Decision:** Editorial does not have (and is not adding) a `job` post_type. Root Signal may build a `Job` signal type for other consumers, but Job signals must not be emitted to the Editorial ingest endpoint (they would 422 on unknown `post_type`). `Condition` signals — persisting environmental state — are similarly not publishable on their own; if a Condition becomes newsworthy, package it as a `story` through a Situation. Force-fitting Jobs as `post_type: "action"` with deadline = application cutoff was considered and rejected: it flattens context and produces low-quality posts.

**Implication:** Editorial's 9-type post_type taxonomy (`story`/`update`/`action`/`event`/`need`/`aid`/`person`/`business`/`reference`) is settled. Root Signal's proposed 4-type expansion (Profile → `person`, LocalBusiness → `business`, Opportunity → `action`, Job → dropped) is accepted as stated. See `docs/handoff-root-signal/TAXONOMY_EXPANSION_BRIEF.md` §2.

### HMAC body signing and the feedback webhook are out of scope

**Decision:** Not building HMAC body signing — Bearer token over HTTPS is sufficient for the threat model, and idempotency keys cover the replay-protection concern for this integration. Not building an Editorial → Root Signal feedback webhook on published/rejected/edited events — useful training signal for Root Signal in principle, but not required for the core integration. If either becomes valuable later, added then; not built speculatively.

**Implication:** Explicit omission from the handoff docs. Neither appears in the spec as a future-phase commitment; if revisited, it's a new decision and a new spec update. Captured in `docs/TODO.md` #1.10 as "intentionally omitted."

### Handoff docs are a specification, not a collaboration debate

**Decision:** The Root Signal handoff docs are written as an integration specification. Every decision that was an "it depends" during drafting is committed to one side or the other — Editorial builds it, or it's a concrete ask of Root Signal with the shape they should implement. No open questions, no phased rollout plan, no "Phase 1/2/3" framing, no project timeframes. The intro framing is explicit: "this is the critical next step in Root Suite. Some interpretation of this will be implemented."

**Implication:** Everything Editorial is building to match the spec lives in `docs/TODO.md` #1 as concrete subsections. `docs/status/2026_04_22_ROOT_SIGNAL_INTEGRATION_GAPS.md` tracks the internal build against the external spec. The handoff docs themselves never reference incomplete internal work — everything is presented as in place, because it will be by the time Root Signal returns with an implementation.

### Rate limits and auth credentials picked, not negotiated

**Decision:** 15 req/sec sustained, 50 req/sec burst per API key (token bucket). API key format `rsk_{env}_<32-char-url-safe-base64>`; stored as SHA-256 hash; rotation via overlapping keys with `rotated_from_id` chain. Issued through secure channel at integration kickoff. These were `TBD` in an earlier draft; committed rather than asked.

**Implication:** Limits are calibrated for expected MN civic-content volume; tune at the operator layer based on real traffic without spec changes. `dev-cli apikey {issue,rotate,revoke,list}` is part of the ingest-endpoint build. Spec lives in request doc §14.

---

## 2026-04-20 — Session: Root Signal contract, Statewide, layout polish, lifecycle gate

### Root Signal is the *producer* of posts, not an enrichment service

**Decision:** The authoritative model is "Root Signal produces posts;
Root Editorial consumes them" (<1% of posts are editor-authored).
Consolidated the prior two in-conflict specs
(`ROOT_SIGNAL_SPEC.md` framed Signal as enrichment-only;
`ROOT_SIGNAL_INGEST_SPEC.md` framed it as production) into a single
canonical doc at `docs/architecture/ROOT_SIGNAL_DATA_CONTRACT.md`.
Both old files now carry a "SUPERSEDED" banner pointing at the new
one.

**Implication:** Any future Signal-integration design should extend
DATA_CONTRACT.md, not the old specs. The ingest validation rules,
body length floors, and per-post-type field-group requirements all
live there.

### body_raw floor is 250 chars on every weight — light included

**Decision:** Even `weight = light` posts need a ≥250-char
`body_raw`. Light is a broadsheet-layout signal (the post renders
in a small card), not a content-depth signal — the detail page
always shows the full body.

**Implication:** Signal must produce a usable body_raw for every
post, not just a one-sentence teaser. Ingest validation rejects
below-floor submissions. The enrichment plan encodes the same rule
for seed data.

### No calendar dates in seed titles or body prose

**Decision:** Seed dates are `NOW()`-relative via `offsetDays`.
Hardcoded dates like "April 3–5" drift every time the seed runs on
a different day. Seed titles/bodies use duration language ("three-
day closure") or weekday-relative references; the `datetime` /
`schedule` field groups are the source of truth for specific dates.

**Implication:** Codified in `SEED_DATA_ENRICHMENT_PLAN.md` §Pass 1
quality bar and in the Highway 169 worked example. When Signal
eventually produces posts for real ingestion, the same rule applies
to any ambient "this week" phrasing.

### Organization linking via `post_sources`, not a direct FK

**Decision:** Migration 122 dropped `posts.organization_id`. We kept
that decision — edition_ops and the data contract both use the
`post → post_sources → sources → organizations` graph to resolve a
post's organization. This preserves multi-source posts (a story
carried on an org's website AND its Instagram = two `post_sources`
rows, one organization).

**Implication:** Every org-link query goes through the source
graph. No shortcut JOIN from `posts` to `organizations`. The seed
pipeline's `organizationName` convenience field feeds the graph via
`post_source_attribution.source_name` matching on the org's exact
`name`; seed throws loudly on unknown org names rather than silently
orphaning.

### Statewide is a *pseudo-county* (B.2), not a virtual edition (B.1)

**Decision:** Migration 236 adds an `is_pseudo BOOLEAN` column on
`counties` and inserts a "Statewide" row (fips_code='statewide').
The layout engine's `load_county_posts` branches on `is_pseudo` and
calls a narrower `load_statewide_posts` helper that pulls only
posts explicitly tagged `service_area='statewide'`. Alternative
considered: compose a statewide edition on the fly from tagged
posts without any DB record (B.1); rejected because editors lose
editorial control over ordering and the admin workflow doesn't
apply uniformly.

**Implication:** All code that iterates "all counties" must decide
whether pseudo counties should be included. Batch-generate
includes them (correct — pseudo counties want editions); dashboard
"N of 87" roll-ups exclude them (correct — statewide is surfaced
in its own callout). `default_edition_title` branches on pseudo
so the title reads "Statewide — Week of…" rather than "Statewide
County — Week of…".

### Lifecycle gate: editions with 0 populated slots can't transition

**Decision:** `require_populated_edition` guard fires on
`review_edition`, `approve_edition`, and `publish_edition` — any of
them rejects an edition with zero slots where `post_id OR widget_id
IS NOT NULL`. Draft stays writable so regeneration still works.

**Implication:** The "Aitkin approved-but-empty" artifact that
surfaced editorial review can't happen again. If a regeneration
clears slots and fails to repopulate, the editor can't push the
empty edition forward — the only remediation is to regenerate
the layout. Pre-existing empty records aren't healed automatically;
they need a manual regenerate or a status reset.

### Anchor cells clamp their bodies like non-anchors

**Decision:** `prepare.ts` used to set `clamp: 0` for anchor cells
on the theory that wider anchor columns "don't need clamping." But
`.clamp-0` isn't a valid CSS class, so anchor bodies rendered un-
clamped and overflowed their cells (alert-urgent / gaz-story /
gaz-request all hit this). Flipped the anchor sentinel from `0` to
`undefined` so anchor cards pick up the template's configured
clamp value like non-anchor cells.

**Implication:** 22 card templates fixed from one change. Any new
template that uses `d.clamp ?? N` with idiomatic defaults now
works correctly in both anchor and non-anchor positions. Features
still get `undefined` (they use `<MRichBody>` which ignores clamp
anyway).

### Whole-tile click is an onClick wrapper, not an overlay

**Decision:** Broadsheet cards are fully clickable via a
`<ClickableTile>` wrapper in `BroadsheetRenderer` that (a) bails on
native interactive targets (anchor/button/input), (b) bails when
text is selected, (c) bails on modifier keys, (d) otherwise calls
`router.push()`. Superseded the earlier `.post-link__overlay`
absolute-positioned `<a>` pattern, which captured all pointer
events and broke text selection + nested-link hover states.

**Implication:** Any future broadsheet card template should use
`<MTitle>` / `<MInlineTitle>` for link participation via
`PostDetailLinkProvider` context — the inner anchor handles
keyboard nav, the wrapper handles the "click empty space" case.
Don't layer another overlay on top.

### Layout engine cell-cohesion pre-pass

**Decision:** Before `fill_slot_group`'s greedy priority-ordered
fill, count candidates per `post_type` in the pool and bias the
sort so the type with the most candidates starts the cell. Lone
high-priority minority-type posts no longer orphan themselves
inside a cell of a different type (dig-story + dig-request mixed
next to pure dig-update cells, etc.). 4 unit tests lock in the
new behavior; existing `fill_slot_group` had none before.

**Implication:** Editors see more cohesive cells without manual
reshuffling. If a post gets pushed to an adjacent row because its
type is in minority, that's by design — it'll cluster with peers
there.

### Success alerts cluster content; padding is on the parent

**Decision:** Two separate fixes to success alerts:
(1) `flex justify-between gap-4` on the content (not
`justify-between` alone) so short messages + dismiss buttons don't
sit across a 1000px whitespace gap in a full-width banner. (2) The
parent column carries `px-6`; alerts use no horizontal margin —
`w-full + mx-6` combine under box-sizing: border-box to overflow
the container by 48px, which is how the "Edition approved" banner
stretched wider than the rows beneath.

**Implication:** Any new alert inside a `max-w-*` column should
inherit horizontal padding from the parent, not carry its own
horizontal margin. This applies to the dashboard flash alerts,
edition-page alerts, and any future admin-ui banner.

### Sidebar Tooltip mounts after hydration

**Decision:** `SidebarMenuButton` defers wrapping its child in
`<Tooltip>` until after `useEffect` fires. Base UI's
`TooltipTrigger` uses `React.useId()` for ARIA wiring, and those
ids diverged between server and client (likely the Suspense
boundary + urql query in `AdminSidebar` shifting the useId
counter) — every sidebar nav button hit a "this won't be patched
up" hydration warning. Skipping the Tooltip wrapper during SSR +
first client render sidesteps the issue entirely; tooltips only
show when the sidebar is collapsed to icons-only, so post-mount
attachment is invisible.

**Implication:** If Base UI fixes its ARIA-id stability we can
revert the deferred mount. Any other Base UI component with
`useId()`-based ARIA wiring nested inside admin-app SSR boundaries
may hit the same issue and need the same treatment.

### "Up to date" = `status === 'published'`, nothing weaker

**Decision:** Both the dashboard UI counter (`"N of 87 counties
published"`) and the `countyDashboard` resolver's `isStale` logic
use `status === "published"` as the only way a county counts as
current. Approved editions are *not* counted as up-to-date — an
editor approving is an internal signal, not a public-site signal.

**Implication:** "Approved but not published" shows up as non-
zero in the dashboard breakdown but doesn't contribute to the
headline coverage counter. Keeps the public-readiness number
honest.

---

## 2026-04-19 — Session: Preview URLs, Org Links, DnD Fix

### Platform presence is `organization_links` rows, not tags

**Decision:** Migration 232 adds `organization_links (org_id, platform,
url, is_public, display_order)`. Stripped `organization` from the
platform tag kind's `allowed_resource_types` and deleted its existing
`taggables` attachments. The 46 platform tag rows themselves stay as
a read-only lookup table for the admin Links picker (display name,
emoji, color).

**Implication:** Tags carry no per-attachment payload. When a future
feature needs "a thing + a url/flag/ordering per thing," the answer
is a dedicated table, not a tag kind + sidecar data.

### Default link visibility comes from `source_type`

**Decision:** When `is_public` is omitted on create, the server
defaults it to TRUE for orgs, FALSE for individuals.

**Implication:** Individuals' platform presence is treated as
operational context, not public info. If we ever add public pages
for individuals (we currently don't), revisit the default —
silently-private links that were never meant to ship could surface.

### No `label` column on organization_links (punted)

**Decision:** Schema is `(platform, url, is_public, display_order)`.
Considered a `label` field for "two Instagrams — main vs. book club"
cases; decided against for MVP.

**Reasoning:** Rare case; URL itself disambiguates in the admin list.
Adding a nullable `label` later is a trivial migration if editors
hit it.

### Admin-gated post preview at `/preview/posts/[id]`

**Decision:** New Rust `/Post/{id}/preview` handler (AdminUser), new
GraphQL `postPreview` query, new web-app route mirroring
`/preview/[editionId]`. The public `post` + `/posts/[id]` route is
unchanged.

**Implication:** "Admin-gated preview" is now a pattern — both
editions and posts use it. Future previewable entities (widgets,
maybe?) should mirror: `/Entity/{id}/preview` AdminUser handler,
`entityPreview` GraphQL query throwing `UNAUTHENTICATED`, web-app
`/preview/entities/[id]` page that surfaces 401 as "Admin Access
Required" instead of a 404.

### "Is this post publicly reachable?" = `status='active' AND ≥1 slotted edition is 'published'`

**Decision:** The admin "View" button uses this check to decide
between `/posts/[id]` (public) and `/preview/posts/[id]` (admin).

**Implication:** Reuse this predicate anywhere the answer to "can a
non-admin see this post?" matters — RSS generation, sitemap,
metrics on "published posts," etc. It's stricter than
`status='active'` alone, which was the old (wrong) answer.

### Post card navigation: title-as-link, not full-card overlay

**Decision:** `MTitle` reads a `PostDetailLinkContext` set by
`BroadsheetRenderer`'s SlotRenderer and renders as `<a>` when
provided. No card-wide overlay. Internal navigation (to detail page)
and external navigation (`readMore` → source URL) are separate
affordances.

**Implication:** **Do not reintroduce card-wide overlay anchors.**
First pass used one; it broke text selection, hover states on inner
elements, and link underlines inside the body. If a future feature
wants more of the card clickable (e.g. photo → detail), add specific
`<a>` wrappers to those elements, don't revive the overlay.

### DnD collision detection must filter by `data.type`, not id prefix

**Decision:** `slotCollisionDetection` looks up each hit's registered
droppable and accepts it as a "card" only when
`data.current.type === 'slot'`. The previous `!id.startsWith("drop-")`
heuristic matched row and section sortable droppables (their ids are
plain UUIDs) and silently hijacked drops into empty cells.

**Implication:** Any future collision-detection tweak should key on
`data.type`, not id patterns. Recurring-risk shape: whenever a new
`useSortable` is added to the same DndContext (e.g. for a new kind
of reorderable parent), its droppable joins the hit pool. ID-based
filters will silently regress.

**Gotcha:** `args.droppableContainers` is array-like, not a Map.
`.get(id)` doesn't exist — iterate and build a Map yourself.

### Commit discipline: wait for user confirmation

**Decision:** Added a "Commit Discipline" section to `CLAUDE.md`.
Programmatic verification — simulated pointer events, `curl`,
typecheck, automated click — is not a substitute for the user
actually performing the interaction. Commit only after they
confirm.

**Implication:** Workflow rule. If this log shows another "I
simulated it, committed, user found it broken" pattern, escalate:
the rule isn't sticking.

---

## 2026-04-18 — Session: Docs Tidy Pass

### Don't organize docs by phase/time

**Decision:** Flattened `docs/architecture/phase4/*` up to
`docs/architecture/`. Future organization subdivides by *purpose*
(Core / Data / Features / Deferred), not by when the work happened.

**Implication:** If a future session is tempted to create
`docs/architecture/phase5/`, don't. Temporal groupings stop being
accurate the moment work finishes — a doc written during "phase 4"
may describe a now-permanent architectural pillar or a fully
abandoned idea, and the folder name tells you neither.

---

## 2026-04-18 — Session: Post ↔ Edition Relationships

### Post detail shows "slotted into" not "eligible for"

**Decision:** The hero header's "In Editions" strip lists editions
that currently reference the post via `edition_slots`. It does not
list editions where the post *would be eligible* but was not picked
by the layout engine.

**Reasoning:** "Slotted into" is one join query. "Eligible but not
slotted" means re-running the layout engine's eligibility query per
edition — expensive and less directly useful. Add the eligibility
view later only if editors ask "why didn't this post show up last
week?" as an actual pattern.

### Future-event eligibility: 8-week horizon on one-off schedules

**Decision:** Add a new branch to `load_county_posts` eligibility:
a post with a `schedules` row where `rrule IS NULL AND dtstart ≥
period_start AND dtstart < period_start + 8 weeks` stays eligible
regardless of `published_at`. See `POST_EDITION_LIFECYCLE.md`.

**Reasoning:** Previously, eligibility was only gated by
`published_at` + `is_evergreen`. A post about "community dinner May 2"
published April 12 became ineligible by April 19 — even though
every edition between then and May 2 was more relevant than the
originating one. The fix considers `schedules.dtstart` so event-
anchored content stays alive through the event. Recurring schedules
(`rrule IS NOT NULL`) are excluded because those are usually
operating hours on evergreen posts, which already pass upstream.

The 8-week cap is editorial judgment for a weekly community paper —
events further out aren't "news" yet; editors can bump priority to
override if they really want earlier coverage.

### Multi-edition rotation: no active policy

**Decision:** The layout engine does not coordinate across editions.
A post eligible in three consecutive weeks may be slotted in all
three, with no cross-edition dedup or rotation penalty. Add a
`priority - k*recent_slot_count` penalty only if editors report
repetition as a problem.

**Reasoning:** Premature optimization. Evergreen posts *should*
appear every week (directories). Non-evergreen posts currently
self-cycle out via the 7-day `published_at` window. No real pain
yet. Designed two options (rotation penalty vs hard cooldown) and
filed them; implementing either speculatively is waste.

### Root Signal dedup: server-side content hash, not LLM-exclusion list

**Decision:** When Root Signal starts ingesting, dedup via
`source_url` + `content_hash` columns on `posts`. Check before
insert; on hit, refresh `published_at` on the existing row rather
than create a near-duplicate. Option 2 (tell Signal in the prompt
what we already have) is a token-saving optimization *on top of*
Option 1, not a replacement.

**Reasoning:** Deterministic server-side dedup works even if the LLM
forgets context. Trusting the prompt-exclusion-list to keep Signal
from re-generating the same content is fragile — works sometimes,
fails silently. Content hashing is a ~1-day feature. See
`POST_EDITION_LIFECYCLE.md`.

---

## 2026-04-17 — Session: Editorial Notes, First-Class

### Notes must be attached to an entity at creation

**Decision:** `CreateNoteMutation` requires `noteableType` +
`noteableId`. No orphan notes. Two dialog components:
`AddNoteDialog` (for post/org detail pages where the entity is in
scope) and `NewNoteDialog` (for the notes list page, adds an entity
picker on top).

**Reasoning:** A note with no entity is meaningless — notes are
flags / annotations / alerts *on* something. Forcing attachment at
creation makes the data model consistent with the product intent,
and avoids a "garbage collection" problem where abandoned orphan
notes accumulate forever.

### `sourceUrl` is not hand-settable from the admin

**Decision:** Both note-creation dialogs deliberately omit the
`sourceUrl` field. It's reserved for the future external-ingest
path (Root Signal attaching notes with provenance URLs).

**Reasoning:** Editor-created notes don't have a meaningful external
source. Exposing the field invited it to be repurposed as "related
link" or similar, which it isn't.

---

## 2026-04-17 — Session: Media Library Unification

### One table (`media_references`) tracks every use of media

**Decision:** New polymorphic `media_references` table links media
to its consumers: `post_hero`, `post_person`, `post_body`, `widget`,
`organization_logo` (last one unused pending Phase 4). Every write
path touching media (post save, widget save, etc.) reconciles the
full desired set idempotently.

**Reasoning:** Before this, "is this image used anywhere?" meant
scanning JSONB blobs and multiple denormalized URL columns. Now it's
a single `SELECT COUNT(*) FROM media_references WHERE media_id = $1`
and a media delete can surface exactly what it's about to orphan.

### `post_body` media refs: walk the bodyAst on save

**Decision:** `admin_update_post` walks `body_ast` recursively,
collects every `mediaId` string that parses as a UUID, and
reconciles `media_references` for `referenceable_type = 'post_body'`.
Always reconciles — even if `body_ast` is `None` — so removing all
body images correctly clears stale refs.

### No organization logos (for now)

**Decision:** Phase 4 of the Media Library plan proposed adding
`logo_media_id` + `logo_url` to the `organizations` table. Dropped.

**Reasoning:** The current design doesn't render organization logos
anywhere in the public broadsheet. "Add the column in case we need
it later" is spec creep dressed up as foresight. When a specific
component actually needs a logo, the migration is trivial (10
minutes). `media_references` already supports arbitrary
`referenceable_type` strings, so nothing architectural needs
pre-enabling.

### External URLs are gone — editor UX is library-only

**Decision:** Every image-bearing admin surface routes through the
`<MediaPicker>` dialog. No "paste URL" escape hatch anywhere (hero
photo, person photo, photo widgets, Plate body images). Seed data
with external Unsplash URLs was replaced with locally-committed
images uploaded through the same presigned-upload pipeline.

**Reasoning:** External URLs are a quiet liability — hotlinks break
when the source goes down, images mutate out from under us, and
tracking pixels can sneak in. Gating all image input through our
library gives us control + dedup + usage tracking for free.
Editor-side friction is low because the library *also* contains the
uploader.

### Build a download-and-import pipeline later, not now

**Decision:** A Root Signal ingest could legitimately hand us a
`source_image_url` we'd want to fetch + store. Designed in
`ROOT_SIGNAL_MEDIA_INGEST.md` but not built. Signal's ingest flow
isn't live yet; build the pipeline when there's content to ingest.

---

## 2026-04-17 — Session: Client-Side Image Processing

### Processing happens in the browser via Canvas, zero deps

**Decision:** `packages/admin-app/lib/image-processing.ts`. Resize
to 1240px on the longest edge, re-encode JPEG at 0.85 quality, strip
EXIF via `createImageBitmap({ imageOrientation: 'from-image' })`.
Applied to uploads before the presigned PUT.

**Reasoning:** The alternative (Rust-side processing via `image` +
`kamadak-exif`) is strictly better quality but requires one of:
proxying uploads through the server (new HTTP endpoint + bandwidth
cost) or a post-upload reprocess step (more moving parts, more
storage during the reprocess window). For a solo-editor CMS with
light upload volume, browser-side is fine.

### Step-down resizing beats single-shot Canvas

**Decision:** Halve the image iteratively until within 2× of target,
then do the final draw. Each iteration uses
`imageSmoothingQuality: 'high'`.

**Reasoning:** Single-shot `drawImage(src, target)` with large scale
ratios (4000→1240) produces jagged edges and moiré even with
smoothing cranked. Pyramid halving preserves detail because each
halving step has enough source pixels to average cleanly.

### Server-side processing is a future option, not future work

**Decision:** Write `SERVER_SIDE_IMAGE_PROCESSING.md` to capture the
conditions that would justify the move (Root Signal ingest wanting
a server-side pipeline, variants, format migration, tamper-proof
limits, etc.). Revisit when any of those triggers fires.

---

## 2026-04-17 — Session: `submission_type` Cleanup

### `ingested` means Root Signal; seed data means `admin`

**Decision:** Fix the stale enum: `ingested` is the post-migration-213
value meaning "came from Root Signal extraction" (previously
`scraped`). Seed posts are editor-originated dummy content — they
get `admin`. 28 existing seed rows were re-labeled via re-seed.

**Reasoning:** Before this, seed data randomly assigned `ingested`
or `admin` for demo variety, despite no real ingest having
happened. The admin UI cemented this confusion by labeling
`ingested` as "Ingested (legacy)" — which was backwards.
`scraped` was dead but still referenced in queries. Every layer
(DB constraint, UI labels, GraphQL filters, seed data, Rust
activities) had a different mental model of what the enum meant.
Made them all agree on the post-migration-213 names.

### `scraped` is not "might come back" — it's dead

**Decision:** Removed `scraped` from UI label maps and GraphQL
filters. No rows carry the value (migration 213 renamed all
existing data), and the CHECK constraint doesn't permit it.

---

## 2026-04-17 — Session: Plate.js Editor Integration

### MediaPicker is a Plate plugin via `render.aboveSlate`

**Decision:** The photo editor's shared MediaPicker is hosted via
`PhotoPickerPlugin` using Plate's `render.aboveSlate` hook — same
shape the built-in `DndPlugin` uses for its `react-dnd` provider.
Not a JSX wrapper around `<PlateEditor>`.

**Reasoning:** First attempt rendered a `<MediaPicker>` `<Dialog>`
inside each void element (photo_a, photo_b, photo_block). That
caused three-separate-Dialog-instances overhead + Base UI's
focus-trap cleanup colliding with Slate's void-element lifecycle.
Lifted once via JSX wrapper — worked but ugly. Correct answer was
the Plate plugin shape, which keeps the provider registration
co-located with the photo plugins that consume it. Canonical-first
is a better default than improvising.

### Body-image drop appends to the end, not the drop position

**Decision:** `onDrop` on `<PlateContent>` uploads image files and
inserts `photo_a` nodes at `[editor.children.length]`. Doesn't try
to compute a Slate path from the DOM drop point.

**Reasoning:** DOM-point → Slate path conversion is fiddly and
error-prone. The block-level DnD (already wired) lets editors drag
the new block into place in one motion. Predictable beats clever
here.

### Don't disable the body editor on save

**Decision:** Removed `disabled={saving}` from `<PlateEditor>` on
the post edit page. Inline title/kicker/deck fields aren't
disabled during save either; body editor being disabled is
inconsistent UX.

**Reasoning:** The actual bug: flipping `disabled` flips Plate's
`readOnly`, which torn down the DndPlugin's block-wrapper state
in a way that didn't recover. But even independently of that bug,
disabling the editor during a sub-second save does no one any favors.
Save is short enough that locking editing is just stutter.

---

## 2026-04 early — Session: Layout Engine Tuning

> Reconstructed from session transcripts by a subagent. Numerical
> specifics (post counts, template coverage) came from the recorded
> debugging sessions; included as context even though the reasoning
> has been summarized.

### Specialty-first template order + scoring boost

**Decision:** Sort templates specialty-first during selection and
apply a 2.0× scoring multiplier to specialty templates (alert-notice,
card-event, generous-exchange, etc.) so they win against generic
templates when both are compatible.

**Reasoning:**
- **Problem:** 7 of 16 specialty templates were dormant — never
  appeared in editions. Generic templates (gazette, bulletin)
  consumed the available posts first, leaving nothing for
  type-restricted specialty templates to bind to.
- **Alternative considered:** Bumping the existing novelty boost
  (1.5×) or just increasing row count. Both insufficient — generic
  templates with more fillable slots kept outscoring specialties
  even with novelty favor.
- **Trade-off:** Generic templates fill slightly later in the
  selection order, but edition variety jumped from 5–7 templates
  used per edition to 12–14. Specialty-first doesn't starve
  generics; they fill with the remaining pool.

### Dynamic slot counts + Phase 4 spillover + filler

**Decision:** Replace exact slot counts with min/max ranges on row
templates; add a Phase 4 spillover pass that greedily packs
remaining posts into catchall rows; seed 14 statewide filler posts
at low priority as a last resort for sparse broadsheets.

**Reasoning:**
- **Problem:** Small counties had posts orphaned because the engine
  was a strict constraint satisfier — rows emit only when every
  slot fills exactly. Aitkin (25 eligible posts) was placing only
  17–19 of them; the remainder sat on the bench despite fitting
  editorial criteria.
- **Alternative considered:** Add more templates. Rejected —
  template variety doesn't solve scarcity. Sparse pools still leave
  gaps that a strict engine won't close.
- **Three-part fix:**
  1. Count ranges (`count_min`/`count_max`) let templates declare
     "1 to 3 lights, any type" instead of "exactly 3."
  2. Phase 4 runs after strict placement finishes and packs
     leftovers into catchall rows — a "community digest" section
     instead of silent dropoff.
  3. Filler content (tax help, voter info, ice safety, etc.)
     tagged `statewide` at low priority — invisible on dense
     broadsheets, visible on sparse ones.
- **Trade-off:** Sparse editions end with lower-priority content.
  Dense editions never see filler because priority sorting keeps
  it out. Target is 100% placement of editorially-relevant content
  without compromising big-county quality.

---

## 2026-03-17 — Session: Prototype Gap Analysis

### Posts vs Widgets: Why they have different storage strategies

**Decision:** Keep posts as a wide relational table with optional field groups. Keep widgets as JSONB discriminated unions. Don't converge them.

**Reasoning:**
- Posts share ~90% of their fields across all 6 types. The type is an *editorial preset* (which field groups are open by default in the CMS form), not an architectural boundary. Any field group can be attached to any type.
- Widgets have ~0% field overlap between types. A `pull_quote` (quote, attribution) shares nothing with a `resource_bar` (label, items[]). A wide table would be mostly NULLs.
- The CLAUDE.md says "avoid JSONB" but widget data is the valid exception: truly type-discriminated content where the alternative (6 separate tables or a NULL-heavy wide table) is worse.
- **Partial overlap noted:** `stat_card` and `number_block` are both "big number + heading + blurb" styled differently. `section_sep` is "heading + blurb" with no number. This cluster motivated the widget template system (see below).

### Widget template system: Merge stat_card + number_block

**Decision:** Collapse `stat_card` and `number_block` into a single `number` widget type with visual variants (templates). Add a `widget_template` column to `edition_slots`.

**Reasoning:**
- Both are structurally identical: `number`, `title`, `body`, optional `color`. The difference is visual treatment (compact card vs colored tile).
- Same logic applies to `section_sep`: the prototype has two visual treatments (default + ledger-style centered). These are variants, not types.
- Widget templates parallel post templates. Posts already have `post_template` on edition_slots; widgets get `widget_template`. Kept as separate nullable columns — slot `kind` discriminates.

### SectionSep: Two variants, not two components

**Decision:** Delete `LedgerSectionBreak.tsx` (dead code). Add `variant` prop to `SectionSep` component. Both CSS classes already exist.

**Context:** `LedgerSectionBreak` was created during prototyping as `Post.led-section-break` — a separate component taking a `Post` type with `d.sub`. This was a prototyping mistake. It's never imported, never registered, takes the wrong type, and is just a centered/larger text variant of `SectionSep`.

### Section separators: Widgets, not section children

**Decision:** Section separators stay as widget records placed in edition slots.

**History (thrashed on this):** The CMS originally had every section auto-render a separator. Then we decoupled them into widgets so editors can place separators wherever they want, or omit them entirely. The current path is: Widget record -> edition_slot (kind=widget) -> edition_row (template=widget-standalone) -> BroadsheetRenderer detects layout variant -> skips Row/Cell wrapper -> renders SectionSep. Three table records and a special-case render path for a horizontal line. It's a Rube Goldberg, but the editorial flexibility justifies it.

**Future note:** The concept of "sections" as parents of rows may be reworked or removed once Root Signal integration clarifies broadsheet data flow. If sections go away, the widget-based separator approach is already correct and unaffected.

### Image widget: Needed but not yet specced

**Decision:** Add `image` widget type. Fields: `src`, `alt`, `caption`, `credit`. Referenced in prototype RT-02 (Photo Essay) but never implemented.

**Open question:** RT-02 uses `FeaturePhoto` which takes a *post* (with media field group) not a widget. The image widget may serve a different purpose — editorial images placed by the layout editor that aren't associated with a post. Clarify during implementation.

### Weight override: Post-level, not slot-level

**Decision:** Don't add weight override to `edition_slots`. Weight is set on the post itself.

**Reasoning:** The only scenario where slot-level weight matters is "same post, different weight in different editions" which is an edge case. The admin already has the post detail page where weight can be changed. Layout engine regeneration would clobber slot-level overrides anyway.

### Ticker strips: Keep as rows for now

**Decision:** Tickers render as rows with ticker-template posts. Don't add standalone ticker strips between sections.

**Reasoning:** A `full` row with ticker-template posts looks visually identical to a standalone ticker strip. If pacing feels wrong with real content, refactor then. The migration path is clean: extract ticker slots from rows into a dedicated structure.

### Field group hydration is the #1 priority

**Decision:** Everything else builds on field group data flowing through the broadsheet pipeline.

**Why:** 43 post components exist. 9 widget components exist. 3,623 lines of CSS exist. But the broadsheet GraphQL query only fetches base post fields. Components that need `person`, `items[]`, `datetime`, `media`, `source`, `meta`, `link`, or `status` render empty sections. Half the prototype's visual richness depends on this data being present. Without it, seed data, render hints, and row template variety are all inert.

### Render hints: Client-side only

**Decision:** Compute display fields (`paragraphs`, `cols`, `dropCap`, `month`, `day`, `when`, `circleLabel`, `count`, `tagLabel`, `readMore`, etc.) in a pure function in `web-app/lib/broadsheet/render-hints.ts`.

**Reasoning:** These are presentation transforms, not business logic. Keeping them client-side means no backend changes, no API contract changes, and the function is trivially testable. If a mobile client needs them later, it can reimplement — the logic is ~100 lines of date formatting and string splitting.

### Prototype spec files (reference)

Three spec files from the prototype repo define the data contracts:
- `POST-DATA-MODEL.md` — 10 field groups, render hints interface, type-to-template compatibility matrix, tag system
- `ROW-DATA-MODEL.md` — broadsheet/section/row/cell/slot hierarchy, 7 row variants, layout engine algorithm, editor controls
- `ROW-TEMPLATES.md` — 31 proven row templates (RT-01 through RT-31) plus 14 additional combinations, with exact character limits for every field in every template

These are the "visual fidelity target." Implementation should match the field coverage and character discipline documented there. The spec lists `LedgerSectionBreak` as a standalone component — this is the prototyping mistake noted above.

### Deferred features

Explicitly punted for post-MVP:
- **Abuse Reporting** — backend stubs exist, everything else missing
- **Map Page** — plan written, not started
- **Email Newsletter** — designed, not started, most infrastructure-heavy
- **Weather Widgets** — 4 components ported, no data source API
