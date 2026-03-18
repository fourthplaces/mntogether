# Decisions Log

> Architectural decisions made during development. Captures the *why* so future sessions don't re-derive context that was expensive to reach.

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
