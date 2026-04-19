# Root Signal → Root Editorial: Post Ingest Specification

> ⚠️ **SUPERSEDED 2026-04-19.** Merged into the authoritative contract at
> [`docs/architecture/ROOT_SIGNAL_DATA_CONTRACT.md`](../architecture/ROOT_SIGNAL_DATA_CONTRACT.md),
> which adds the individual-source model, organization dedup rules, per-post-type field-group
> requirements, a 250-char floor on `body_raw`, and a full validation-rules section.
>
> This file is kept for historical reference only. Do not cite in new work.

**Status:** Draft — to be finalized before live Signal integration.
**Audience:** Root Signal engineering team, Root Editorial editors.

This document specifies what Root Signal should produce for each post so that Root Editorial's layout engine can render a complete, visually balanced broadsheet.

---

## The post contract

Every post Root Signal writes must include:

| Field | Required | Description |
|---|---|---|
| `title` | ✓ | 60–80 chars. Headline. |
| `body_raw` | ✓ | Full editorial body. See length targets below. |
| `body_heavy` | conditional | Long-form version — required if `weight = heavy`. |
| `body_medium` | conditional | Mid-length version — required if `weight = medium` or `heavy`. |
| `body_light` | ✓ | Ticker/digest version — 1 sentence. Required for all weights. |
| `post_type` | ✓ | One of 9 values (see below). Drives visual variant. |
| `weight` | ✓ | `heavy` \| `medium` \| `light`. Drives layout column width. |
| `priority` | ✓ | 0–100. Higher = more prominent placement. |
| `service_area` | ✓ | At least one tag: `{county-slug}-county` or `statewide`. |
| `published_at` | ✓ | ISO timestamp. |
| `is_evergreen` | optional | Default false. True for standing content (references, business listings). Bypasses 7-day eligibility filter. |

**Editor-only fields** (Root Signal should never set these):
- `pencil_mark` — editorial emphasis overlay set by editors in admin UI
- `is_urgent` — set only for genuinely urgent safety content

---

## Post types (9 total)

| Type | Used for | Templates | Weight mix |
|---|---|---|---|
| `story` | Narrative reporting, features | feature, gazette, digest | heavy / medium / light |
| `update` | News updates, advisories | feature-reversed, alert-notice, ledger, digest, ticker, whisper-notice | heavy / medium / light |
| `action` | Call-to-action items | feature-reversed, alert-notice, ledger, ticker | medium |
| `event` | Scheduled events | feature, card-event, gazette, bulletin, ledger, ticker | medium / light |
| `need` | Community asks | generous-exchange, pinboard-exchange, bulletin, ledger, digest, ticker | medium / light |
| `aid` | Community offers | generous-exchange, pinboard-exchange, bulletin, ledger, digest, ticker | medium / light |
| `person` | Profiles, interviews | feature, gazette, spotlight-local, bulletin | heavy / medium / light |
| `business` | Local business spotlights | gazette, spotlight-local, bulletin | medium / light |
| `reference` | Directories, resource lists | feature, gazette, directory-ref, bulletin, ledger, quick-ref | heavy / medium / light |

---

## Body length targets

Each post body must be produced at **three length tiers** so the renderer can select the right one based on the template placement. The layout engine picks a template based on `weight` + `post_type` compatibility; the renderer then picks the body field based on the template's needs.

| Tier | Field | Target | Max | Used by templates |
|---|---|---|---|---|
| Heavy | `body_heavy` | **800** | 1400 | feature (2-col) |
| Mid-heavy | `body_heavy` | 300 | 500 | feature-reversed |
| Medium | `body_medium` | 200 | 280 | gazette, bulletin, spotlight-local, generous-exchange, alert-notice |
| Light | `body_light` | 60 | 120 | digest, ledger, quick-ref, whisper-notice |
| Ticker | `body_light` | 30 | 80 | ticker, ticker-update |

**Characters, not words.** A typical sentence averages 100–120 chars including spaces.

**Heavy weight = 800+ chars minimum.** The feature template renders in 2 columns at the 2/3-width cell on wide screens. 800 chars ≈ 400 chars per column ≈ a full paragraph per column. Anything shorter looks sparse.

These targets are stored in the database (`post_template_configs.body_target` / `body_max`) and can be tuned without a schema change:

```sql
SELECT slug, weight, body_target, body_max FROM post_template_configs ORDER BY weight, body_target DESC;
```

---

## Weight distribution expectations

The layout engine's three-phase selection (hero / mid / dense) expects a realistic mix:

| Weight | % of weekly pool | Editorial role |
|---|---|---|
| `heavy` | 10–20% | Above-the-fold features. The lead stories. |
| `medium` | 40–60% | Mid-page content. Core reporting. |
| `light` | 30–50% | Tickers, classifieds, brief updates. |

**If Signal produces only medium posts**, the broadsheet will be a wall of uniform-density gazette cards with no hero features at top and no ticker pacing at bottom. Weight diversity is critical.

**Target total editorial weight per county:**
- Weight formula: `heavy × 3 + medium × 2 + light × 1`
- Each county has a `target_content_weight` setting (default 66, editor-adjustable)
- 66 ≈ 6 heavy + 14 medium + 20 light = typical 40-post week
- Signal should aim for this total; the layout engine flexes ±30%

---

## Service area tagging

Every post needs at least one `service_area` tag. Three valid patterns:

1. **County-specific:** `hennepin-county`, `aitkin-county`, etc. Use the `{county-slug}-county` format. County slugs match `counties.name` lowercased + kebab-case (e.g. `"Lac qui Parle"` → `lac-qui-parle-county`). The full list of 87 MN counties lives in `data/tags.json`.
2. **Statewide:** `statewide` — posts relevant everywhere in MN (state hotlines, statewide policy news).
3. **Multiple counties:** Posts can carry multiple service_area tags, e.g. `["hennepin-county", "ramsey-county"]` for metro-wide content.

**Posts with no service_area tag are only eligible when they also have no location data.** Don't omit the tag — always provide at least one.

---

## Evergreen content

Set `is_evergreen = true` for:
- Reference directories (county resource lists, hotline compilations)
- Business listings (standing local business profiles)
- Recurring annual events (if they're produced once and surfaced yearly)

Evergreen posts bypass the layout engine's 7-day `published_at` eligibility filter, so they stay in the eligible pool indefinitely.

**Don't mark news stories evergreen** — they should age out naturally.

---

## Validation & ingestion flow

Recommended Signal → Editorial pipeline:

```
Root Signal batch
  ↓
[Validation stage] — reject posts missing required fields, malformed tags,
                    or with body lengths below the weight tier's minimum.
  ↓
POST /Posts/create_post  (per post)
  ↓
Post lands with status='active' in Root Editorial
  ↓
Weekly cron: /Editions/generate_edition for each county
  ↓
[Editorial review] — editors set pencilMark/is_urgent where warranted
  ↓
/Editions/publish_edition
```

**Validation errors to surface:**
- Missing body tier for the post's weight (heavy without body_heavy, etc.)
- Body below minimum (heavy <800 chars, medium <200 chars)
- Missing or unknown post_type
- Missing or unknown service_area tag
- `pencil_mark` or `is_urgent` set by Signal (these are editor-only)

---

## Open questions

1. **Does Signal assign priority scores, or should Root Editorial compute them from engagement signals?**
2. **How should Signal handle posts that span multiple counties?** Multiple service_area tags, or one "metro" tag with a secondary scope?
3. **What's the cadence?** Weekly batch vs. continuous stream vs. real-time webhook?
4. **Versioning:** If Signal updates an existing post (corrects a fact, adds a source), does it overwrite or create a revision? Root Editorial already has `revision_of_post_id` for this.
5. **Media attachments (photos, audio):** How does Signal supply these, and what sizes/formats?

---

## Related docs

- Layout engine internals: `packages/server/src/domains/editions/activities/layout_engine.rs`
- Post template compatibility matrix: `docs/status/BROADSHEET_GENERATION_POSTMORTEM.md`
- Schema definitions: `packages/server/migrations/000216_expand_post_types.sql`, `000221_evergreen_and_height_overrides.sql`
