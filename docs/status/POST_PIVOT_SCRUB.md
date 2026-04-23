# Post-Pivot Scrub — scope document

**Status:** Scope-only. Enumerates pre-pivot residue to remove. Does not execute. Sibling of [`docs/TODO.md` §1.12 / §1.13](../TODO.md) and the framing in [`CLAUDE.md` §Reading This Codebase](../../CLAUDE.md).

**Rationale.** Pre-pivot artefacts keep resurfacing in audits, seeds, and LLM analyses because they leave fingerprints across surfaces (seed JSONs, GraphQL schema, migrations, docs). The runtime code is mostly clean after the 189–193 pivot migrations; the *surrounding* artefacts still echo the old design. One thorough scrub pass makes subsequent drift small enough to manage.

**Not in scope for the scrub.** The migrations themselves. They're an append-only history log; they do not get edited. The rule is "runtime wins over migrations" ([`CLAUDE.md`](../../CLAUDE.md)) — after the scrub, everything *except* migrations reflects current design. Migrations stay as-is.

---

## How to run a scrub pass

Each item below is structured as:

- **What** — concrete thing to remove.
- **Verify dead** — how to confirm nothing still uses it before deleting.
- **Risk** — blast radius if removal misses a consumer.
- **Commit shape** — suggested commit boundary.

Work through the list by surface area (A → E). Each surface is roughly independent; hold a stable build between surfaces.

---

## A. GraphQL schema — dead Post fields

The `Post` type in [`packages/shared/graphql/schema.ts:225-278`](../../packages/shared/graphql/schema.ts) still exposes pre-pivot fields. The Rust server's `Post` model + activities no longer read/write them; the resolvers either pass NULL through or reference dropped columns.

### A.1 `Post.category: String`

- **What:** Remove `category` field from `type Post`, `type PostConnection` entries, and any input types. Also remove the `category` arg from `find_active_*` / `find_near_*` methods in `packages/server/src/domains/posts/models/post.rs` (lines ~1280–1441 — they accept a `category: Option<&str>` parameter used for `service_offered` tag filtering, which is a dead pre-pivot tag kind).
- **Verify dead:** `grep -rn 'category' packages/admin-app/src packages/web-app/app` — if nothing consumes it, safe. `service_offered` tag kind confirmed deprecated (migration 000197:714).
- **Risk:** Low. Category filtering was pre-pivot taxonomy; post-pivot uses `tags.topic`.
- **Commit shape:** One commit scrubbing schema + resolver + Rust methods.

### A.2 `Post.urgency: String`

- **What:** Remove `urgency` field from `type Post` and related input types. Also drop the `urgency` kwarg on any `update_content` resolver paths. Current runtime uses `is_urgent: BOOLEAN` (migration 000213:28-29 replaced the freeform text).
- **Verify dead:** `grep -rn 'urgency' packages/admin-app packages/web-app packages/shared` — any occurrence is either a UI string or the dead field.
- **Risk:** Low.
- **Commit shape:** Combined with A.1 if changes cluster.

### A.3 `Post.sourceUrl: String`

- **What:** Either remove the field entirely or fix the resolver to read from `post_sources[0].source_url`. The underlying `posts.source_url` column was dropped in migration 000213:22; the public post detail page at `PostDetailView.tsx:462-476` renders a "Source" sidebar card when this field is non-null, which is currently always null for ingested posts.
- **Decision:** Fix, don't remove — public rendering needs a source URL. Resolve from the primary entry in `post_sources` (after [Addendum 01](../handoff-root-signal/ADDENDUM_01_CITATIONS_AND_SOURCE_METADATA.md) lands, use the `is_primary` flag).
- **Risk:** Medium. Public-site card disappears if the resolver change is wrong.
- **Commit shape:** Separate commit. Bundle with the §1.10 citations build once that work starts.

### A.4 Other potentially-dead GraphQL types

Quick scan candidates (verify before touching):

- `PostConnection.category: String!` — likely paired with A.1.
- `UrgentNote.sourceUrl` — different surface (notes feature); verify notes UI actually renders it before concluding dead.
- Any `tldr` / `custom_description` / `custom_title` fields — drop on sight.

### A.5 Dead Post input types

- `UpdatePostInput.category`, `UpdatePostInput.urgency`, `UpdatePostInput.sourceUrl` — mirror removals from A.1/A.2/A.3 into the mutation input shape. Admin-app forms will need corresponding field removal.

---

## B. Seed data (`data/*.json`)

### B.1 `data/organizations.json` — dead fields

- **What:** Remove pre-pivot trust-signaling fields from every org entry:
  - `year_founded` — age isn't a trust signal Editorial cares about.
  - `populations_served` — duplicates what Root Signal extracts and/or the `source.organization.populations_served` envelope field.
  - `employees` — never read.
  - `volunteers_needed` — pre-pivot "matching" feature, dead.
  - `ice_resistance_focus` — duplicated by the `ice-safe` + `immigration-status-safe` safety tags.
  - `county` — duplicated by tagging via `service_area`.
  - `sources` — organisation-side source list; post-level `post_sources` is the current model.
- **Keep:** `name`, `website`, `phone`, `address`.
- **Verify dead:** Grep `data/seed.mjs` — the loader writes to `organizations (name, description, ...)` and silently ignores the extra fields. Confirm by running `make seed` before/after; row counts identical.
- **Risk:** None (fields already silently dropped).
- **Commit shape:** One commit. Use `jq` or a Python script to strip the fields across all entries consistently.

### B.2 `data/posts.json` — audit for stale keys

- **What:** Current post keys per audit: `title`, `postType`, `weight`, `priority`, `bodyHeavy`, `bodyMedium`, `bodyLight`, `publishedOffsetDays`, `zipCode`, `location`, `tags`, `meta`, `sourceAttribution`, `organizationName`, `contacts`, `items`, `datetime`, `link`, `person`, `media`, `scheduleEntries`, `status`, `pencilMark`, `is_urgent`, `submissionType`, `isEvergreen`, `_comment`. All current. No scrub needed on shape.
- **What to watch:** `tags.topic` still contains the 5 mixed-in neighborhood slugs + `restaurant` — scrub those uses per TODO §1.7, and either relocate to a new `neighborhood` kind or drop entirely.
- **Verify dead:** Run `make audit-seed` — baseline JSON would flag format mismatches.
- **Risk:** Low.
- **Commit shape:** One commit, rebaseline `audit-seed.baseline.json` after.

### B.3 `data/widgets.json` — verify alignment

- **What:** Confirm widget types in the JSON match current `widgets.widget_type` CHECK constraint (`number`, `pull_quote`, `resource_bar`, `weather`, `section_sep`, `photo` — migration 000217).
- **Verify dead:** `grep widget_type` in widgets.json vs the migration constraint.
- **Risk:** Low.
- **Commit shape:** Rolled into B.2 if needed.

---

## C. Tag taxonomy — deadweight kinds

Migration 000197 introduced many kinds; the pivot only kept `topic`, `service_area`, `safety`. The leftover kinds exist as rows in `tag_kinds` and/or `tags` tables but are not referenced by runtime code.

### C.1 Drop dead `tag_kinds` rows

- **What:** Delete rows from `tag_kinds` for: `audience_role`, `reserved`, `structure`, `platform`, `verification`, `language`, `county`, `city`. Keep: `topic`, `service_area`, `safety`, `neighborhood` (reserved per [DATA_MODEL §5.2](../architecture/DATA_MODEL.md)).
- **Verify dead:** `grep -rn "kind = '<kind>'"` across `packages/server/src/` — confirm no runtime code queries by these kinds. `grep -rn '<kind>'` in admin-app + web-app — confirm no UI reference.
- **Risk:** Low. Migration 000213 already deleted the `tags` rows for some of these; this just cleans the kind definitions. `county` / `city` / `language` tags in `tags` may still exist — audit separately.
- **Commit shape:** New migration (the house rule allows adding migrations, just not editing them — see `.claude/hooks/guard-migrations.sh`). Name like `000237_drop_pre_pivot_tag_kinds.sql`.

### C.2 Tighten `tags.kind` CHECK constraint

- **What:** Add a CHECK constraint on `tags.kind` allowing only the current set. Currently there's no CHECK — `tags.kind` is free-form TEXT.
- **Proposed:** `CHECK (kind IN ('topic', 'service_area', 'safety', 'neighborhood'))`
- **Verify dead:** Run a query against the live DB: `SELECT DISTINCT kind FROM tags` — any kind not in the allowed set is a scrub blocker. Clean those rows first, then add the constraint.
- **Risk:** Medium. A failed CHECK breaks ingest. Stage in a migration that scrubs first, adds constraint second.
- **Commit shape:** Follows C.1.

### C.3 Audit safety tag slug format

- **What:** The three existing safety tags use underscores (`no_id_required`, `ice_safe`, `know_your_rights`). Normalise to hyphen-case (`no-id-required`, `ice-safe`) per [`TAG_VOCABULARY.md`](../handoff-root-signal/TAG_VOCABULARY.md). Drop `know_your_rights` — moved to topic domain per the 2026-04-22 decisions.
- **Verify dead:** `grep 'no_id_required\|ice_safe\|know_your_rights'` — confirm no runtime code pattern-matches these exact strings.
- **Risk:** Low (no real data yet; seed posts don't reference safety tags).
- **Commit shape:** Combined with C.1 in the migration.

### C.4 Seed the expanded safety vocabulary

- **What:** Per [`TAG_VOCABULARY.md` §3](../handoff-root-signal/TAG_VOCABULARY.md), seed all 29 policy-modifier slugs into `tags` under `kind = 'safety'`.
- **Commit shape:** Combined with C.1 / C.3 migration, or as `data/tags.json` update + reseed.

---

## D. Posts-table columns worth a second look

### D.1 Probably clean — spot-check anyway

Run `grep -rn '<column>' packages/server/src packages/shared` for each, confirm referenced. Columns present on `posts` per the current model (`packages/server/src/domains/posts/models/post.rs:17-87`): `id`, `title`, `body_raw`, `body_ast`, `body_heavy`, `body_medium`, `body_light`, `post_type`, `weight`, `priority`, `is_urgent`, `is_evergreen`, `pencil_mark`, `status`, `source_language`, `location`, `zip_code`, `latitude`, `longitude`, `submission_type`, `submitted_by_id`, `extraction_confidence`, `deleted_at`, `deleted_reason`, `published_at`, `created_at`, `updated_at`, `revision_of_post_id`, `translation_of_id`, `duplicate_of_id`, `embedding`, `search_vector`, `is_seed`. These all look current.

### D.2 Post-ingest columns to add (per Addendum 01)

Part of the normal build, not scrub — but listed here for surface completeness: `post_sources.content_hash`, `snippet`, `confidence`, `platform_id`, `platform_post_type_hint`. See [TODO §1.10](../TODO.md).

---

## E. Documentation — stale headers and pivot-era descriptions

### E.1 `docs/architecture/DATABASE_SCHEMA.md`

- **What:** Self-describes as "covers through migration 171." Schema is at 236. Either regenerate from a live DB snapshot, or re-write narratively against the current schema, or mark the whole doc `SUPERSEDED` and point at [`DATA_MODEL.md`](../architecture/DATA_MODEL.md).
- **Risk:** Low — already self-flagged as stale.
- **Commit shape:** Rewrite or supersede in one commit. Recommend supersede — a narrative schema doc is hard to keep current, and `DATA_MODEL.md` + the Rust models in the codebase already cover current state.

### E.2 `docs/architecture/ROOT_EDITORIAL_PIVOT.md`

- **What:** Dated 2026-02-24, describes the pivot rationale. Still a useful historical document. Add a banner at the top noting "This doc is the pivot rationale from Feb 2026; for current design, see [DATA_MODEL.md](DATA_MODEL.md)." Do not delete — it's valuable history.
- **Risk:** None.
- **Commit shape:** Trivial.

### E.3 `docs/architecture/ROOT_SIGNAL_SPEC.md`, `docs/guides/ROOT_SIGNAL_INGEST_SPEC.md`

- **What:** Already carry "SUPERSEDED" banners pointing at [`ROOT_SIGNAL_DATA_CONTRACT.md`](../architecture/ROOT_SIGNAL_DATA_CONTRACT.md). Verify banners are accurate; consider whether to delete these entirely now that the handoff folder is the primary reference.
- **Risk:** Low.
- **Commit shape:** Audit + either banner-refresh or delete.

### E.4 `docs/architecture/SIMPLIFIED_SCHEMA.md`

- **What:** Pre-pivot (describes a 3-type post system — `service`, `opportunity`, `business`). Either rewrite, supersede, or delete.
- **Risk:** None (no runtime consumer).
- **Commit shape:** Delete or mark superseded pointing at `DATA_MODEL.md`.

### E.5 `docs/architecture/EMBEDDING_FEATURES_REFERENCE.md`

- **What:** Per previous TODO notes, catalogues removed AI features. Review for currency.
- **Risk:** None.
- **Commit shape:** Refresh or leave as historical.

### E.6 `data/README.md`

- **What:** If it documents the seed shape, confirm it's current.
- **Risk:** None.

---

## F. Recommended execution order

One scrub session, roughly 2–3 days of focused work. Order:

1. **A** — GraphQL schema. Quickest. Front-loads risk: if anything downstream breaks, caught immediately.
2. **B** — Seed data. Low risk; visible effects in `make audit-seed`.
3. **C** — Tag taxonomy. Requires one new migration (000237 or whatever next number). Blocks ingest-endpoint tag-resolution work.
4. **D** — Posts-table columns. Mostly a spot-check, not actual deletion. Light work.
5. **E** — Docs. Can be done at any point, but doing it last means the preceding surface scrubs have already settled, reducing re-writes.

After F completes: ingest-endpoint build ([TODO §1.1](../TODO.md)) and Addendum 01 build ([TODO §1.10](../TODO.md)) can proceed against a clean substrate.

---

## G. Checklist

- [x] A.1 `Post.category` removed
- [x] A.2 `Post.urgency` removed
- [ ] A.3 `Post.sourceUrl` resolved via `post_sources` — deferred, bundled with the Addendum 01 citations build (Worktree 6)
- [x] A.4 Other dead Post fields (audit pass) — removed `PublicFilters.categories` + `FilterOption` as adjacent dead cruft; no `tldr` / `custom_*` fields found
- [x] A.5 Mutation input types aligned
- [x] B.1 `organizations.json` scrubbed
- [x] B.2 `posts.json` topic slugs cleaned
- [x] B.3 `widgets.json` aligned — widget_types already match the migration 217 CHECK (`section_sep`, `pull_quote`, `photo`, `resource_bar`, `number`)
- [x] C.1 Dead `tag_kinds` rows dropped (migration `000237`). Platform kept as the `organization_links` UI lookup table — see DATA_MODEL §5.3.
- [x] C.2 `tags.kind` CHECK constraint tightened — set is `topic | service_area | safety | neighborhood | platform`
- [x] C.3 Safety slug normalisation — underscore→hyphen, `know_your_rights` dropped
- [x] C.4 Expanded safety vocabulary seeded — 29 slugs in both the migration and `data/tags.json`
- [x] D.1 Posts-table columns spot-checked — `Post` Rust struct matches the post-pivot design
- [x] E.1 `DATABASE_SCHEMA.md` superseded, points at DATA_MODEL.md
- [x] E.2 `ROOT_EDITORIAL_PIVOT.md` banner added
- [ ] E.3 Superseded Root Signal specs cleaned or deleted — skipped this pass; banners already point at `ROOT_SIGNAL_DATA_CONTRACT.md`. Revisit when the handoff docs settle.
- [x] E.4 `SIMPLIFIED_SCHEMA.md` superseded
- [x] E.5 `EMBEDDING_FEATURES_REFERENCE.md` reviewed — still an accurate historical catalogue, no edits
- [ ] E.6 `data/README.md` reviewed — not opened this pass (seed shape changes were local to the JSONs, not to the loader conventions)

---

## H. What "done" looks like

- An agent reading the repo cold produces the same answer for "what's the current data model" as a human reading [`DATA_MODEL.md`](../architecture/DATA_MODEL.md).
- `grep` for any pre-pivot field name or kind across `packages/`, `data/`, `docs/` returns either runtime code (if genuinely current), migration files (which are history), or nothing.
- `make audit-seed` passes with no drift warnings.
- The GraphQL schema has no `?` fields that always resolve to null.
- `data/organizations.json` and `data/posts.json` round-trip through seed cleanly — every field either persists to the schema or is explicitly an input-only field.
- No doc in `docs/architecture/` describes the system as scoring trust, auto-summarising content, crawling external sites, or anything else that's now Root Signal's concern.
