# Decisions Log

> Architectural decisions made during development. Captures the *why* so future sessions don't re-derive context that was expensive to reach.

> **Ordering:** Most recent sessions first. Within a session, decisions
> are grouped by topic, not chronology.

---

## 2026-04-18 — Session: Docs Tidy Pass

### Flatten `phase4/` — temporal grouping doesn't survive

**Decision:** Move all 11 `docs/architecture/phase4/*.md` files up to
`docs/architecture/`. Don't replace with another subfolder.

**Reasoning:** The `phase4` grouping was meaningful when active work
was organized into phases, but most of those docs are now either
implemented or permanently deferred. Going forward, a flat
`architecture/` folder subdivided in the README index by purpose
(Core / Data & Schema / Features / Deferred) is honest about the
state of each doc without imposing a temporal fiction.

### `TESTING_WORKFLOWS.md` → `TESTING_GUIDE.md`

**Decision:** Rename. The content was already rewritten when Restate
was removed; the filename was still history.

**Reasoning:** File names should match content. Left a one-line note
in `docs/TODO.md`'s stale-docs table to close the loop; removed it
after the rename shipped.

### `DOCKER_ARCHITECTURE.md` belongs under `setup/`, not `architecture/`

**Decision:** Docker is a dev-environment concern, not system
architecture. Moved it next to the other setup docs.

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
