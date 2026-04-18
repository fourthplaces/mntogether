# Post ↔ Edition Lifecycle & Dedup (design reference)

**Status:** Reference doc. Captures the current behavior, the edge cases that
motivate it, and open questions that will need answers before Root Signal
goes live. Partially implemented; see "What's built" and "What's open" below.

**Audience:** Anyone working on the layout engine, Root Signal integration,
the post-detail admin page, or future dedup work.

**Last updated:** 2026-04-18.

---

## The question this doc exists to answer

When a post lives in the database, the relationship between it and the
weekly broadsheet editions isn't 1:1. A post can appear in:

- Zero editions (draft, not yet eligible, or rejected by the layout engine).
- Exactly one edition (typical case — posted this week, slotted this week).
- Multiple editions simultaneously (statewide tags, multi-county service areas).
- Multiple editions across time (evergreen posts, or events scheduled in the
  future).

Each of those has different editorial semantics. This doc traces how they
flow through the code today, what edge cases aren't handled yet, and what
a principled dedup story looks like for Root Signal.

---

## Data model summary

Relevant tables (all in `packages/server/migrations/`):

```
editions          — one per (county, weekly period)
  id, county_id, period_start, period_end, status, ...

edition_rows      — visible rows stacked top-to-bottom in an edition
  id, edition_id, template_slug, sort_order

edition_slots     — posts and widgets placed within rows
  id, edition_row_id, kind, post_id, widget_id,
  post_template, slot_index, sort_order

posts
  id, status, post_type, weight, priority, is_evergreen, is_urgent,
  published_at, zip_code, ...

taggables         — polymorphic tag links
schedules         — polymorphic one-off and recurring schedules
locationables     — polymorphic location links (→ locations → zip_counties)
```

**Key observation**: `edition_slots` has no unique constraint on `post_id`.
The data model already supports multi-edition reuse — the question is
whether the layout engine chooses to do it.

---

## How a post becomes eligible for an edition

The source of truth is `load_county_posts` in
`packages/server/src/domains/editions/activities/layout_engine.rs`.

A post is eligible for `county_X`'s edition starting on `period_start` iff
**all** of the following hold:

1. **Active status**: `p.status = 'active'`

2. **County match** — any one of:
   - The post's primary location's zip maps to `county_X` via `zip_counties`.
   - The post has a `service_area` tag matching `county_X`'s slug.
   - The post has a `statewide` tag (matches every county).
   - The post has no location *and* no `service_area` tag (the "ambient"
     fallback — editors writing county-agnostic content).

3. **Recency** — any one of:
   - `is_evergreen = true` (bypasses all time filters).
   - `published_at IS NULL`.
   - `published_at >= period_start - 7 days` (one-week-fresh window).
   - **One-off event with `dtstart` in `[period_start, period_start + 8 weeks)`**
     (added 2026-04-18; see "Future-event eligibility" below).

The engine then ranks eligible posts by `priority DESC` and hands them to the
template-filling phase.

---

## Edge cases and how each is handled today

### Evergreen posts

`is_evergreen = true` bypasses the recency filter. Used for reference
directories, business listings, anything meant to stand.

- **Migration 221** added the column and backfilled posts with
  `post_type IN ('reference', 'business')` as evergreen.
- **Consequence**: an evergreen post is eligible forever, so it can
  legitimately appear in every weekly edition. Nothing prevents the layout
  engine from picking the same evergreen post week after week — and for
  things like "Meals on Wheels directory" that's the intended behavior.
- **Open question**: do we ever *want* to not re-slot? If the same
  directory has appeared in 6 consecutive editions, editors may prefer
  rotation. Not handled today; not asked for yet.

### Statewide posts

A post tagged `statewide` is eligible for every county's edition. Nothing
in the layout engine coordinates across counties — each county's edition
is generated independently. So a statewide post naturally shows up in all
87 counties' editions the week it's eligible.

This is fine. It matches the editorial model: statewide content is
equally relevant everywhere.

### Location-specific posts across multiple counties

A post can legitimately be in multiple counties' editions via
`service_area` tags, not via location. A single post's location maps to
one zip → one county (zip_counties is 1:1). But a post with zip in
Hennepin *and* `service_area: ramsey` appears in both Hennepin and
Ramsey editions.

There's no deliberate "place this post in county X AND county Y" UI —
editors express it by tagging service areas. Whether that's the right
affordance is an editorial question, not a technical one.

### Future-dated events

**Fixed 2026-04-18.** Before this fix: a post about "community dinner on
May 2" published Apr 12 became ineligible by Apr 19 (7-day `published_at`
window expired), even though the event was still future and the Apr 26
edition would be the most relevant one to run it in.

The eligibility query now includes a fourth branch that keeps a post
eligible if it has a one-off schedule (`rrule IS NULL`) with
`dtstart >= period_start` and `dtstart < period_start + 8 weeks`.

**Why these choices:**

- `rrule IS NULL` — only one-off events. Recurring schedules are usually
  operating hours on business/reference posts, which already pass the
  evergreen branch. Adding them here would over-include.
- 8-week horizon — editorial intuition for a community paper. Events
  much further out aren't "news" yet; editors can bump priority if they
  want earlier coverage.

**Consequence:** A future-event post is now eligible for every edition
from publish date through one week after the event. The engine may or
may not slot it in every one of those editions — see "Multi-edition reuse"
below.

### Non-evergreen, non-event posts

The 7-day `published_at` window means most "regular" posts are eligible
for at most one edition (the one published the week of the post) and
possibly two (if `published_at` falls early enough in a week to overlap
two edition period starts). The engine doesn't currently coordinate
across consecutive editions; if the same post is eligible in two weeks'
slots, nothing prevents it from being slotted in both.

In practice: rare, because most posts have publish dates tightly
clustered with the edition they were written for.

---

## Multi-edition reuse: current behavior vs. what editors might want

**Today**: the layout engine runs independently per edition. It picks
eligible posts by priority, fills row templates, and writes
`edition_slots`. It does not look at other editions' slottings when
deciding what to include.

**Implications:**

- Evergreen posts: no problem. Re-slotting the same reference directory
  every week is the intended behavior.
- Future-event posts: will likely be slotted for all ~8 editions leading
  up to the event, if priority keeps it near the top of the eligible
  pool. Editors may want this ("we're reminding people about the
  dinner") or may not ("seeing the same event card four weeks running
  feels stale").
- Regular posts: effectively one-edition-each today, by virtue of the
  7-day window.

**Decision not yet made**: whether the engine should actively rotate
posts that have already appeared. Options sketched but not built:

1. **Rotation penalty.** When selecting eligible posts, subtract a
   penalty from `priority` for each recent slotting. Something like
   `effective_priority = priority - 10 * editions_slotted_last_4_weeks`.
   Would gently push frequently-slotted evergreen content down the
   pool without excluding it.
2. **Hard cooldown.** A post can't be slotted again within N editions
   of its last appearance. Simpler, less graceful.
3. **Editor override.** Stick the policy onto the post row itself:
   `rotation_policy: 'every_week' | 'rotate' | 'once'`. Most editorial
   control, most UI.

None of these are built. If editors start complaining about repetition,
start with option 1 — it's the least invasive and doesn't require new
UI.

---

## Seeing the relationship: admin UI

The post detail page's hero now shows every edition currently slotting
this post — parent relationship, mirroring the Notes section's child
relationship.

Wired via:
- `Post::find_edition_slottings(post_id)` —
  `packages/server/src/domains/posts/models/post.rs`.
  Joins `edition_slots → edition_rows → editions → counties`.
- `POST /Post/{id}/edition_slottings` HTTP endpoint.
- GraphQL `Post.editionSlottings: [PostEditionSlotting!]!`.
- Rendered in `PostDetailHero.tsx` as a chip row: county name, period
  range, edition status badge. Each chip links to the edition.

The hero does **not** show "eligible but not slotted" — that would
require re-running the layout engine's eligibility query per edition,
which is expensive. If editorially valuable later, add it as a
separate deferred query.

---

## Root Signal dedup: the adjacent problem

When Root Signal goes live, it will receive a prompt each week asking for
posts for upcoming editions. Without coordination, it will produce
near-duplicate posts about the same underlying events week after week —
"Community dinner May 2" authored again on Apr 19, Apr 26, May 3 with
slight wording differences each time.

This is the same class of problem as multi-edition reuse, but at the
data level instead of the placement level. Two shapes for solving it:

### Option 1: Server-side content fingerprint (recommended)

When Signal submits a candidate post, the server checks for an existing
post that represents the same underlying event/content. If found, refresh
its eligibility rather than create a new post.

**Concrete shape:**

1. Add to `posts` table:
   - `source_url TEXT` — canonical external URL if any.
   - `content_hash TEXT` — hash of normalized title + key identifying
     fields (date, location, org).

2. In the `create_extracted_post` path
   (`packages/server/src/domains/posts/activities/create_post.rs`),
   before inserting:
   - Compute `content_hash` from the candidate.
   - `SELECT id FROM posts WHERE content_hash = $1 OR source_url = $1 LIMIT 1`.
   - If hit: either update `published_at = now()` (extends 7-day window)
     or bump priority to keep the existing post eligible. Skip the
     insert.
   - If miss: insert normally.

**Advantages:**

- Deterministic. Doesn't trust the LLM to remember what it said last
  week.
- Adds real data to the post record (`source_url`, `content_hash`) that
  can also power a "view source" affordance and detect accidental
  editor-created duplicates.
- Natural pairing with the `revision_of_post_id` system: if Signal
  writes a meaningfully-different version (e.g. event details changed),
  it can create a new post linked back to the original as a revision.

**Cost:** one migration, hashing function, dedup check in the ingest
path. ~1–2 days of work.

### Option 2: Tell Signal what we already have (nice-to-have on top)

When asking Signal for posts for an edition, include the list of
currently-eligible evergreen + future-event post titles/summaries as
"already covered." Signal's prompt excludes them from output.

**Advantages:**
- Saves LLM tokens — Signal doesn't generate novel content we'd just
  reject.
- Less work per week (fewer candidate posts to evaluate).

**Disadvantages:**
- Trust-based. The LLM might still produce near-duplicates.
- Requires Signal-side prompt cooperation.

**My read**: Option 1 is the foundation because it's deterministic and
works even if Signal forgets. Option 2 is a token-saving optimization on
top, worth doing once Option 1 is in place.

---

## What's built (as of this doc's date)

- **Future-event eligibility** in `load_county_posts` — 8-week horizon
  for one-off event schedules.
- **`Post.editionSlottings` GraphQL field** + hero UI to see "which
  editions include this post."
- **`media_references`** polymorphic table (migration 231) — shows
  which editions reference media; unrelated to this doc but part of the
  same "what uses what?" story.

## What's open

- **Root Signal dedup.** Option 1 from above is the proposal of record.
  Not built. Depends on Signal actually submitting posts via a defined
  path (`create_extracted_post`) — that wiring also doesn't exist yet.
- **Rotation policy for multi-edition reuse.** Nothing built. Punt
  until editors report seeing the same content week-over-week as a
  problem.
- **"Eligible but not slotted" admin view.** Nothing built. Consider
  when editors start asking "why did this post not show up in last
  week's edition?"
- **Revision linkage on dedup.** When Option 1 ships, decide whether a
  matching-hash ingest should always update in place, or sometimes
  create a new post linked via `revision_of_post_id`. Likely content-
  dependent: small wording changes → update in place, meaningful
  information update → new revision.

---

## Decision record

- Layout engine eligibility for future events is now time-windowed on
  `schedules.dtstart`, not just `published_at`. Editors get a ~8-week
  runway before an event.
- Parent-relationship visibility on post detail page is "which editions
  currently slot this post" — fast, concrete, read-only. "Eligible but
  not slotted" is explicitly punted to avoid recomputation cost.
- Root Signal dedup will be server-side content-hash based (Option 1)
  when it's built. Option 2 is a later optimization, not a replacement.
- No active cross-edition rotation policy today. Add only if editors
  report repetition as a problem, not preemptively.
