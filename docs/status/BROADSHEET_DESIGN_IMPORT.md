# Broadsheet Design Import — Postmortem

**Date:** 2026-03-10
**Scope:** Import the broadsheet newspaper design system from the standalone prototype (`mntogether-temp/next`) into the production CMS-backed web app.

---

## What We Did

Ported a complete newspaper-style design system from a self-contained Next.js prototype into the production MN Together web app. The prototype used hardcoded mock data; the production version renders from CMS edition data via GraphQL.

### Backend (Rust/Axum)

- Built `public_current_broadsheet` endpoint — unauthenticated, serves full edition data (posts, contacts, tags, urgent notes, widgets) for a county
- Made the layout engine weight-aware: `find_compatible_post_template()` now assigns templates based on slot weight (heavy/feature, medium/gazette, light/digest+ticker) instead of defaulting everything to gazette
- Added GraphQL schema types and resolvers for the public broadsheet query
- Added null-coercion resolver for `BroadsheetPost` (urgentNotes, tags, contacts) to prevent GraphQL errors on nullable arrays

### Frontend (Next.js)

- Created `BroadsheetRenderer` — the main orchestrator that takes a GraphQL edition result and renders the full newspaper: BroadsheetHeader, Masthead, Rows with distributed slots
- Built the data pipeline: `row-map.ts` (CMS row slugs to grid layouts), `templates.ts` (post template + type to component), `prepare.ts` (GraphQL data to broadsheet Post interface)
- Moved standard pages into `(app)/` route group so the broadsheet homepage gets a clean layout without Header/Footer
- Ported all CSS (3,600+ lines), design tokens, fonts, and the paper texture system

### Layout Fixes

- EditionBar rendered before Masthead (matching prototype)
- Green `--deep-forest` background on body (including overscroll)
- PostcardWelcome above the newspaper, SiteFooter below
- Removed stray `border-top: 2px solid var(--ink) !important` from `.row--rule` that was hitting every row
- Renamed `edition-bar` to `broadsheet-header`
- Fixed nested `<a>` hydration error in `MReadMore` (changed to `<span>`)
- Corrected `postsPerCell` values in row-map (three-column was `[2,2,2]`, should be `[1,1,1]`)

---

## What Went Wrong

### 1. Everything defaulted to Gazette
The layout engine's `find_compatible_post_template()` always preferred `gazette` regardless of slot weight. Every post rendered identically — no visual hierarchy. Root cause: the function was written as a placeholder and never updated when the weight system was finalized.

### 2. postsPerCell values were guessed, not derived
The initial `row-map.ts` had values like `[2,2,2]` for three-column (CMS only allocates 3 posts, 1 per column). This caused the first cell to greedily consume 2 posts, leaving the third column empty. Should have been derived directly from the migration's slot definitions.

### 3. `!important` border leaked across all rows
`.row--rule { border-top: 2px solid var(--ink) !important; }` was applied to every row via `<Row rule>` in BroadsheetRenderer. The `!important` overrode per-component border styling. The `rule` prop should never have been set unconditionally.

### 4. Globals.css declarations leaked into broadsheet
The `.site-footer` class in `globals.css` had `border-top: 1px solid var(--color-border)` — fine for standard pages but wrong inside the broadsheet's green footer. The broadsheet and global CSS share class names but need different styling. This will likely come up again.

---

## What's Done vs Outstanding

### Done (100%)

| Area | Count | Notes |
|------|-------|-------|
| Post renderers | 45 components | All 6 families (Feature, Gazette, Bulletin, Ledger, Ticker, Digest) |
| Chrome/layout | 8 components | NewspaperFrame, Row, Cell, Masthead, BroadsheetHeader, PostcardWelcome, SiteFooter, DebugLabels |
| Widgets | 9 components | SectionSep and ResourceBar wired to CMS; weather placeholder |
| Pencil decorations | 6 components | SVG hand-drawn marks |
| CSS | 3,623 lines | Full port of design tokens, families, grid, texture |
| Type system | Complete | Post interface, RowVariant, CellSpan, display helpers |
| Transform layer | Complete | `preparePost()` bridges GraphQL to broadsheet types |
| Template registry | 7 families | feature, feature-reversed, gazette, ledger, bulletin, ticker, digest |
| Row layout mapping | 7 CMS templates | hero-with-sidebar, hero-full, three-column, two-column-wide-narrow, classifieds, ticker, single-medium |

### Outstanding

#### Specialty components not in template registry
9 components exist in code but aren't auto-rendered from CMS data:
- AlertNotice, BroadsheetSpotlight, BroadsheetTickerNotice, CardEvent, DirectoryRef, GenerousExchange, PinboardExchange, QuickRef, WhisperNotice

These need a decision: are they CMS-driven (need post template mappings) or editorially placed (manual slot assignment)?

#### Detail pages not routed
14 detail page components (`components/broadsheet/detail/`) plus 5 hours-visualization widgets are ported but not mounted to any route. `broadsheet-detail.css` hasn't been ported yet either. This is the next major piece — clicking a post on the broadsheet should open its detail page.

#### Weather widgets need data
WeatherForecast, WeatherAlmanac, WeatherThermo, WeatherLine exist as components but render nothing. They need a weather data source (API integration or CMS widget config).

#### Row variants not fully exercised
The prototype showcases `pair-stack`, `trio-mixed`, and other layout variants that the CMS doesn't currently produce. The row-map handles them as fallbacks but they haven't been visually tested with real data.

#### CSS class name collisions between globals and broadsheet
`.site-footer`, and potentially other shared class names, are styled differently in `globals.css` vs `broadsheet.css`. The `(app)/` route group isolates the Header/Footer, but shared class names in the footer and other components could collide. May need a namespace strategy (e.g., `.bs-footer` vs `.app-footer`) if this keeps happening.

---

## Key Files

| File | Role |
|------|------|
| `packages/server/src/domains/editions/activities/layout_engine.rs` | Weight-aware post placement algorithm |
| `packages/server/src/api/routes/editions.rs` | Public broadsheet API endpoint |
| `packages/web-app/components/broadsheet/BroadsheetRenderer.tsx` | Main rendering orchestrator |
| `packages/web-app/lib/broadsheet/row-map.ts` | CMS row slug to grid layout mapping |
| `packages/web-app/lib/broadsheet/templates.ts` | (postTemplate, postType) to component resolution |
| `packages/web-app/lib/broadsheet/prepare.ts` | GraphQL data to broadsheet Post transform |
| `packages/web-app/app/broadsheet.css` | Full design system CSS (3,623 lines) |
| `packages/web-app/app/(app)/layout.tsx` | Route group isolating standard pages from broadsheet |
