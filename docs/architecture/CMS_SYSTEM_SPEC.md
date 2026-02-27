# Root Editorial — CMS System Specification

> **Status:** Design spec, ready for CMS UI design phase
> **Date:** 2026-02-25
> **Preceded by:** [`POST_TYPE_SYSTEM.md`](POST_TYPE_SYSTEM.md) (design rationale with full decision reasoning)
> **What this is:** Conclusive specification for the post type system, data model, layout system, and CMS UI requirements. Written as a handoff to enable designing the Root Editorial CMS interface.

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Architecture: The Four Layers](#2-architecture-the-four-layers)
3. [Post Types (6 Form Presets)](#3-post-types-6-form-presets)
4. [Post Data Schema](#4-post-data-schema)
5. [Field Groups](#5-field-groups)
6. [Tags](#6-tags)
7. [Layout System](#7-layout-system)
8. [Post Templates and Character Limits](#8-post-templates-and-character-limits)
9. [CMS UI Requirements](#9-cms-ui-requirements)
10. [Root Signal Integration](#10-root-signal-integration)
11. [Design Principles and Flexibility](#11-design-principles-and-flexibility)
12. [Resolved Decisions](#12-resolved-decisions)
13. [Deferred Decisions](#13-deferred-decisions)
14. [Appendix A: Type Quick Reference](#appendix-a-type-quick-reference)
15. [Appendix B: Field Group Quick Reference](#appendix-b-field-group-quick-reference)
16. [Appendix C: Mapping from Design Playground](#appendix-c-mapping-from-design-playground)
17. [Appendix D: Example Broadsheet Data](#appendix-d-example-broadsheet-data)

---

## 1. System Overview

### What We're Building

Root Editorial is a CMS that produces a weekly/daily digital broadsheet (newspaper-style homepage) for community-serving content. Most content is auto-generated from Root Signal (a graph database of community signals), then reviewed and tweaked by a human editor who spends ~1-2 hours/week on it.

### The Core User Story

> "I have an hour or two a week to check and prep the edition going out for next week. The AI gives me a 95% ready broadsheet, then I tweak or add."

### What the Type System Is

**Types are CMS form presets. That's their primary job.** When an editor creates or edits a post, the type determines which input fields appear by default. Types also inform the layout engine (what "shape" is this post?) and the renderer (what visual templates can display it?), but the CMS form is the primary consumer.

**Types are soft, not rigid.** Any field group can be toggled onto any post regardless of type. Types just set sensible defaults. If a type turns out to be wrong for a post, changing it is a dropdown switch — the data doesn't change, just which fields are foregrounded.

**Types are cheap to add or remove.** A type is an entry in a config table: a name, a list of default-open field groups, a default weight, and a list of compatible post templates. Adding type #7 later means adding a row to this config. Removing a type means reassigning its posts to another type.

---

## 2. Architecture: The Four Layers

```
┌─────────────────────────────────────────────────────────────┐
│  ROOT SIGNAL                                                │
│  Graph database. Produces: Situations, Signals, Evidence.   │
│  Signal categories: Tensions, Needs, Aid, Notices,          │
│  Gatherings.                                                │
│  Supports natural-language queries.                         │
│  Intelligence: HIGH                                         │
├─────────────────────────────────────────────────────────────┤
│  GLUE LAYER  (likely lives on Root Signal's side)           │
│  Queries Root Signal. Maps signals to post types.           │
│  Sets initial priority + weight. Drafts body text via LLM.  │
│  Proposes tags. Produces a draft broadsheet of posts.       │
│  Intelligence: MEDIUM                                       │
├─────────────────────────────────────────────────────────────┤
│  ROOT EDITORIAL  (this CMS)                                 │
│  Editor reviews proposed broadsheet. Reorders rows.         │
│  Edits post content. Adds manual posts. Adjusts weight      │
│  and priority. Approves and publishes.                      │
│  Intelligence: HUMAN (editor judgment)                      │
├─────────────────────────────────────────────────────────────┤
│  NEXTJS MN TOGETHER  (reader-facing frontend)               │
│  Renders the published broadsheet. Applies post templates.  │
│  Handles detail pages, navigation, reader interactions.     │
│  Intelligence: LOW (just rendering)                         │
└─────────────────────────────────────────────────────────────┘
```

### Key Boundaries

- **Root Signal owns intelligence.** It understands community tensions, impact, urgency, relationships between signals. It can answer natural-language queries like "give me all high impact volunteer opportunities."
- **The glue layer owns translation.** It converts Root Signal's graph output into flat posts with types, tags, priority, and weight. It drafts body text. It may live on Root Signal's side — the CMS doesn't care, it just receives posts.
- **Root Editorial owns editorial judgment.** The human decides what to surface, what to spike, what to rewrite, and what to add manually. The CMS UI should make this fast and intuitive.
- **The layout engine owns slot-filling.** It takes posts with priority and weight, matches them to row slots. It is dumb by design. No content understanding, no topic inference, no magic.
- **The renderer owns visual treatment.** It takes a broadsheet structure and applies post templates. Also dumb — it just follows the template instructions.

### What the CMS Does NOT Do

- Does not query Root Signal directly (the glue layer does that)
- Does not understand community tensions or signal relationships
- Does not make smart layout decisions (the layout engine is mechanical)
- Does not decide what topics are important (priority comes from the glue layer, adjusted by the editor)

---

## 3. Post Types (6 Form Presets)

Six types. Each one represents a distinct CMS editing experience — a different form with different default field groups open.

### Why 6 and Not More

The rule: **don't make different types for what is essentially the same data structure.** If two types have the same fields and the same editorial workflow, they're the same type with different tags. Types exist where the editing experience is genuinely different — where the editor does different work and sees different inputs.

### Why This Can Change Later

Adding a 7th type requires: a new entry in the type config (name, default field groups, default weight, compatible templates). No schema migration, no layout engine changes, no renderer rewrites. If editors consistently create a certain kind of post and wish they had a dedicated form for it, add the type. If a type is rarely used, merge it into another. Types are config, not architecture.

---

### Type 1: Story

**What it is:** Long-form narrative content. Community voices, features, explainers, in-depth reporting.

**CMS form experience:** Rich text editor dominates the form. Multi-paragraph body with formatting toolbar (bold, italic, links, block quotes, headers). This is the "writing" mode — the editor reads, shapes, and rewrites prose.

**Default field groups open:** media, meta (kicker, byline)

**Default weight:** `heavy`

**Compatible post templates:** feature, gazette, digest

**Editor's mental model:** *"I need to read this and decide if the writing is ready."*

**Could originate from:** Root Signal Situation narrative (auto-drafted), editor's original writing, LLM draft from multiple signals, community submission.

---

### Type 2: Notice

**What it is:** Short, timely informational content. Covers the full spectrum from neutral updates ("MNsure deadline Friday") to critical alerts ("Boil water advisory"). The urgency level is handled by tags and priority, not by splitting into separate types.

**CMS form experience:** Short body field (a few sentences, not paragraphs). Timestamp field. Source attribution fields (who issued this — city, county, utility, community org). This is the lightest editing experience — "title, sentence or two, check the source, done."

**Default field groups open:** meta (timestamp), source

**Default weight:** `light`

**Compatible post templates:** gazette, ledger, bulletin, ticker, digest, feature-reversed (for urgent/alert treatment)

**Editor's mental model:** *"Is this current? How prominent should it be?"*

**How urgency works within Notice:**
- A routine update is a Notice with low priority and no special tags.
- An alert is a Notice with the `urgent` tag, high priority, and source attribution. The `urgent` tag triggers high-contrast visual treatment (dark background, red accent) and elevated placement.
- The editor can escalate any Notice to urgent, or de-escalate, by toggling the tag and adjusting priority. No type change needed.

**Why Alert was merged into Notice:** During design discussion, Alert was initially a separate type. On reflection, the CMS form is nearly identical (short body + source). The difference is urgency level, which is a tag. Keeping them as one type with a visual modifier is simpler and avoids the glue layer needing to make a severity judgment call — all notices arrive as Notice type, the editor (or a suggested-escalation indicator) decides prominence.

**Could originate from:** Root Signal Notices (any severity), automated deadline tracking, official government advisories, community safety reports.

---

### Type 3: Exchange

**What it is:** A community need or offer. Someone needs something (volunteers, goods, housing, services, money) or someone has something to give. Both directions use the same form because the fields are identical.

**CMS form experience:** Structured fields dominate — this is a "fill in the details" form, not a prose editor. Contact info, list of items needed/available, current status, optional location and schedule. The body field is present but secondary (a sentence or two of description, not paragraphs).

**Default field groups open:** contact, items, status

**Default weight:** `medium`

**Compatible post templates:** gazette, ledger, bulletin, ticker

**Editor's mental model:** *"Is this verified? Is it still active/available?"*

**Direction is a tag:**
- `need` tag → "Volunteers Needed", "Donations Needed", etc.
- `aid` tag → "Available", "Offering", etc.
- The renderer shows different verbiage and color treatment based on which tag is present (rust tones for needs, green/moss for aid).
- The layout engine can group needs and aids separately by filtering on these tags.

**Why this is one type, not two:** The CMS form is identical for needs and offers. Same fields, same editorial workflow (verify details, check status). Making them separate types would create two identical forms distinguished only by a label — that's what tags are for.

**Could originate from:** Root Signal Needs or Aid signals, community submissions, organizational postings, mutual aid networks.

---

### Type 4: Event

**What it is:** Something happening at a specific time and place. Community gatherings, workshops, meetings, drives, fairs, celebrations.

**CMS form experience:** Date/time picker is the prominent UI element. Location/address field with optional map. Description body. Recurring toggle (weekly ESL class vs. one-off food drive). Optional cost and registration link.

**Default field groups open:** datetime, location, contact

**Default weight:** `medium`

**Compatible post templates:** gazette, ledger, bulletin, ticker, feature

**Editor's mental model:** *"Is this still upcoming? Worth featuring?"*

**Recurring events:** The `datetime` field group has a `recurring` toggle. When on, the renderer shows a schedule pattern ("Mon 6-7:30pm") instead of a single date. The `recurring` reserved tag is also applied. For editorial lifecycle: recurring events auto-include in the broadsheet proposal each week. The editor can set an optional `until` date for auto-expiration, tag `closed` to end it, or simply spike it from a given week without closing it.

**Could originate from:** Root Signal Gatherings, community calendar submissions, organizational postings.

---

### Type 5: Spotlight

**What it is:** A feature profile of a person, business, or organization. "Neighbor to Know," business listings, organizational profiles. This covers both people and places — the form adapts based on which fields are filled.

**CMS form experience:** Structured profile fields: name, role/tagline, bio/description, photo, quote. For businesses: hours, location, contact, links. The editing experience is "curating a profile card" — assembling structured info about a subject.

**Default field groups open:** person, media, location, contact

**Default weight:** `medium` (can be overridden to `heavy` for feature profiles)

**Compatible post templates:** feature, gazette, bulletin

**Editor's mental model:** *"Is this the right week to feature this?"*

**People vs. places:** Distinguished by which fields are populated and by tags (`person` vs. `business`). If `person` fields are filled (name, role, bio, quote), render as a community profile. If business fields are filled (tagline, hours, location), render as a business listing. Both can coexist — a profile of a business owner includes person AND business fields.

**Could originate from:** Editor-created profiles, community nominations, business directory data, LLM-drafted profiles from Root Signal.

---

### Type 6: Reference

**What it is:** Evergreen, structured information. Directories, resource lists, guides. Low churn — updated periodically, not daily. The "clip and save" content.

**CMS form experience:** Structured item list is the primary input — rows of name + detail pairs (e.g., food shelf name + address + hours). Contact, location, hours/schedule fields. "Last updated" freshness label. The editor's job is verification: *"are these phone numbers still correct?"*

**Default field groups open:** items, contact, location, schedule, meta (updated)

**Default weight:** `medium`

**Compatible post templates:** gazette, ledger, bulletin

**Editor's mental model:** *"Is this still accurate? When was it last verified?"*

**Could originate from:** Curated directories, organizational resource pages, LLM-compiled reference guides, editor-maintained lists.

---

## 4. Post Data Schema

### Universal Fields (every post, every type)

```
Post {
  // ── Identity ──────────────────────────────────────────
  id:          string          // unique identifier
  type:        enum            // "story" | "notice" | "exchange" | "event" | "spotlight" | "reference"
  tags:        string[]        // free-form, multi-select (see §6 Tags)

  // ── Layout Metadata ───────────────────────────────────
  weight:      enum            // "heavy" | "medium" | "light"
  priority:    integer         // higher number = more important / closer to top of broadsheet

  // ── Universal Content ─────────────────────────────────
  title:       string          // every post has a title
  body:        string          // rich text (markdown or HTML) — can be one sentence or many paragraphs

  // ── Field Groups (all optional, see §5) ───────────────
  media:       MediaGroup?     // image, caption, credit
  contact:     ContactGroup?   // phone, email, website
  location:    LocationGroup?  // address, coordinates
  schedule:    ScheduleGroup?  // weekly hours entries
  items:       ItemsGroup?     // list of name+detail pairs
  status:      StatusGroup?    // availability state + verified date
  datetime:    DatetimeGroup?  // start, end, cost, recurring
  person:      PersonGroup?    // name, role, bio, photo, quote
  link:        LinkGroup?      // CTA label, url, deadline
  source:      SourceGroup?    // origin name + attribution
  meta:        MetaGroup?      // kicker, byline, timestamp, updated
}
```

### The `weight` Field

Weight tells the layout engine what column width this post needs. Three values:

| Weight | Meaning | Typical Slot | Example |
|--------|---------|-------------|---------|
| `heavy` | Full column or feature-width | Hero slot, wide column | Feature story with image, in-depth spotlight |
| `medium` | Standard card/column | Equal column, sidebar card | Exchange listing, event card, reference block |
| `light` | Compact/ticker | Ticker strip, classifieds | Brief notice, short exchange, update |

Each type has a default weight (set in the type config). The glue layer or editor can override per post. Weight only describes column width — the post template handles content height through truncation (see §8).

### The `priority` Field

A single integer. Higher = more important = closer to the top of the broadsheet. The glue layer sets this from Root Signal's impact/recency/severity scoring. The editor adjusts it by reordering rows or manually changing the number.

Priority is intentionally a single dimension. The glue layer is responsible for collapsing Root Signal's multidimensional scoring into one number. The layout engine just sorts by it.

---

## 5. Field Groups

Field groups are optional blocks of structured input fields. Every field group is available on every post type — type only determines which are **open by default** in the CMS form. The editor can always expand any collapsed group or collapse any open group.

### 5.1 media
```
{
  image:    string    // image URL or upload reference
  caption:  string    // image caption text
  credit:   string    // photographer / source credit
}
```
**Default open on:** Story, Spotlight

### 5.2 contact
```
{
  phone:    string
  email:    string
  website:  string
}
```
**Default open on:** Exchange, Event, Reference, Spotlight

### 5.3 location
```
{
  address:      string        // human-readable address
  coordinates:  [lat, lng]    // for map rendering (optional)
}
```
**Default open on:** Event, Reference, Spotlight

### 5.4 schedule
```
{
  entries: [
    { day: string, opens: string, closes: string }
    // e.g. { day: "Monday", opens: "9:00 AM", closes: "5:00 PM" }
  ]
}
```
**Default open on:** Reference, Spotlight (business)

Note: `schedule` is for recurring weekly hours (food shelf hours, business hours). For event timing, see `datetime`. These are different field groups because they serve different UI patterns — `schedule` renders as a weekly grid, `datetime` renders as a calendar date.

### 5.5 items
```
[
  { name: string, detail: string }
  // e.g. { name: "Winter coats (adult)", detail: "Sizes M-XXL, new or gently used" }
]
```
**Default open on:** Exchange, Reference

### 5.6 status
```
{
  state:     string    // freeform: "Available now", "Needed", "Closed", "By referral", etc.
  verified:  date      // when status was last confirmed
}
```
**Default open on:** Exchange

### 5.7 datetime
```
{
  start:      datetime
  end:        datetime    // optional
  cost:       string      // "Free", "$5", "Sliding scale", etc.
  recurring:  boolean     // true → renderer shows schedule pattern, not one-off date
  until:      date        // optional — auto-expire recurring events after this date
}
```
**Default open on:** Event

### 5.8 person
```
{
  name:    string
  role:    string    // "Community Organizer", "Owner", "Volunteer Coordinator"
  bio:     string    // short biographical text
  photo:   string    // image URL or upload reference
  quote:   string    // pull quote or testimonial
}
```
**Default open on:** Spotlight

### 5.9 link
```
{
  label:     string    // button text: "Sign the petition", "Register to vote", "Learn more"
  url:       string    // destination URL
  deadline:  date      // optional: "Action needed by March 12"
}
```
**Default open on:** (none — toggled open when needed, e.g., for CTA-style posts)

The `link` group plus an `action` tag is how Call to Action posts work. Any type can become action-oriented by opening this group and adding the tag. The renderer shows a prominent CTA button and optional deadline badge.

### 5.10 source
```
{
  name:          string    // "City of Minneapolis", "Hennepin County", "Community Report"
  attribution:   string    // additional context: "Public Works Department", "via StarTribune"
}
```
**Default open on:** Notice

### 5.11 meta
```
{
  kicker:      string      // topic label above title: "Community Voices", "Housing", "Environment"
  byline:      string      // author attribution
  timestamp:   datetime    // when published or last significant update
  updated:     string      // freshness label: "Updated weekly", "Updated Feb 2026"
}
```
**Default open on:** Story (kicker, byline), Notice (timestamp), Reference (updated)

---

## 6. Tags

### What Tags Are

Tags are free-form string labels attached to posts. They are not a closed set — new tags emerge as content demands them. An editor or the glue layer can attach any tag to any post.

### Three Kinds of Tags

**Topic tags** — what the post is about. Used for filtering, search, discovery.
> housing, food, health, education, immigration, legal, environment, transit, safety, employment, youth, seniors, disability, language-access, resettlement, childcare, animals, winter-gear, voting, census ...

This list is not exhaustive or fixed. It grows organically. The CMS should suggest existing popular tags when the editor is tagging (typeahead/autocomplete from the existing tag pool).

**Geographic tags** — where the post is relevant. Used for geographic filtering.
> north-minneapolis, phillips, lake-street, midtown, cedar-riverside, hennepin-county, ramsey-county, statewide ...

**Reserved tags** — a small fixed set that trigger specific visual or behavioral treatment in the renderer and layout system:

| Reserved Tag | Effect |
|-------------|--------|
| `urgent` | High-contrast visual treatment (dark background, red accent). Signals the layout engine to prefer higher placement. Applies to any type — an urgent Notice, an urgent Exchange, an urgent Event. |
| `recurring` | Renderer shows schedule pattern instead of one-off date. Signals the broadsheet builder to auto-include in future weeks. |
| `closed` | Greyed-out treatment. "Fulfilled" / "Expired" badge. Post remains in the system but is visually de-emphasized and can be filtered out of active broadsheets. |
| `need` | Direction indicator on Exchange type. Renderer uses "needed" language and warm/rust color tones. Layout engine can group needs together. |
| `aid` | Direction indicator on Exchange type. Renderer uses "available" language and green/moss color tones. Layout engine can group aid together. |
| `action` | Triggers CTA rendering — prominent link button, deadline display. Used alongside the `link` field group to make any post a call to action. |
| `person` | On Spotlight type: render as community member profile. |
| `business` | On Spotlight type: render as business/org listing. |

Reserved tags are visually distinct in the CMS tagging interface (e.g., colored chips, separate "modifier" section) so editors can distinguish them from free-form topic tags.

---

## 7. Layout System

### The Broadsheet Structure

A broadsheet (one edition of the homepage) is an ordered list of rows. Each row uses a row template that defines its column layout. Posts fill the slots within each row, rendered using post templates.

```
Broadsheet
  ├── Row 1  (row template: "hero-with-sidebar")
  │     ├── Slot A: [Post → feature template]
  │     ├── Slot B: [Post → ticker template]
  │     └── Slot C: [Post → ticker template]
  ├── Row 2  (row template: "three-column")
  │     ├── Slot A: [Post → gazette template]
  │     ├── Slot B: [Post → gazette template]
  │     └── Slot C: [Post → bulletin template]
  ├── Row 3  (row template: "classifieds")
  │     ├── Slot A: [Post → ledger template]
  │     ├── Slot B: [Post → ledger template]
  │     ├── Slot C: [Post → ledger template]
  │     └── Slot D: [Post → ledger template]
  └── ...
```

**Row order = editorial importance.** The topmost row is the most prominent. The editor reorders rows by dragging.

### Row Templates

A row template defines a column grid with typed slots. Each slot has a weight constraint (what size posts fit) and a count.

```
Row Template {
  id:           string
  name:         string       // human-readable: "Hero with sidebar"
  description:  string       // "Full-width feature with stacked sidebar items"
  slots: [
    {
      weight:   enum         // "heavy" | "medium" | "light"
      count:    integer      // how many posts fit in this slot group
      accepts:  string[]?    // optional type filter: ["exchange", "event"]
    }
  ]
}
```

**Slot `accepts` is optional.** Most row templates have weight-only slots (any type that fits the weight is eligible). Some editorial-purpose rows (e.g., "community classifieds") can restrict to specific types via `accepts`. Start with weight-only; add `accepts` filters only when the auto-generated broadsheet produces incoherent groupings.

**Example row templates:**

| Template | Slots | Use Case |
|----------|-------|----------|
| `hero-with-sidebar` | 1 heavy + 3 light | Lead story with sidebar briefs |
| `hero-full` | 1 heavy | Single dominant feature |
| `three-column` | 3 medium | Mixed content row |
| `two-column-wide-narrow` | 1 heavy + 1 medium | Story with related sidebar |
| `four-column` | 4 medium | Dense card grid |
| `classifieds` | 4-6 light | Compact listings (needs, offers) |
| `ticker` | 5-8 light | Horizontal strip of brief items |
| `single-medium` | 1 medium | Standalone card (event, spotlight) |

### Post Templates

A post template defines the visual treatment of a single post within a slot. Each template declares which types it can render.

| Post Template | Description | Compatible Types |
|--------------|-------------|------------------|
| `feature` | Premium editorial. Large typography, dramatic layout, image-heavy. | story, event, spotlight |
| `feature-reversed` | Dark/high-contrast treatment. Used for urgent notices. | notice |
| `gazette` | Top-border tabbed frame, colored accent. Standard card. | all 6 types |
| `ledger` | Left-border tabbed, classifieds feel, compact. | notice, exchange, event, reference |
| `bulletin` | Boxed card, community board feel. | notice, exchange, event, reference, spotlight |
| `ticker` | Ultra-compact single-line. | notice, exchange, event |
| `digest` | Headline-only, no body text. | story, notice, exchange |

**Post templates enforce character limits** (see §8).

### The Layout Engine Algorithm

The layout engine is **deliberately dumb**. It receives posts with priority and weight already set by the glue layer. It does not understand content, topics, urgency, or community impact. It just matches posts to slots.

```
INPUT:
  posts:          Post[]           // each with type, weight, priority, tags
  row_templates:  RowTemplate[]    // available row layouts
  post_templates: PostTemplate[]   // available visual treatments (with type compatibility)

ALGORITHM:
  1. Sort posts by priority (descending)
  2. Select row templates to fill the broadsheet
     (heuristic: use available rows that fit the weight distribution of posts)
  3. For each post (in priority order):
     a. Find available slots where post.weight matches slot.weight
     b. If slot has `accepts` filter, check post.type is in the list
     c. Find a compatible post template (one that supports post.type)
     d. Place post in slot with chosen post template
  4. Order rows by the highest-priority post in each row
  5. Output the broadsheet

OUTPUT:
  Broadsheet {
    rows: [
      {
        template:  RowTemplateId,
        slots: [
          { post: PostId, post_template: PostTemplateId },
          ...
        ]
      },
      ...
    ]
  }
```

### What the Editor Can Do Post-Generation

The layout engine's output is a **proposal**. The editor can:

| Action | Description |
|--------|-------------|
| **Reorder rows** | Drag rows up/down. Top = most prominent. |
| **Swap posts** | Move a post from one slot to another (if weight and type are compatible with the target slot). |
| **Change a post's weight** | Override the default — e.g., make a short story `medium` to fit a smaller slot. |
| **Change row template** | Switch a row from 2-column to 3-column, etc. Posts re-slot within the new template. |
| **Change post template** | Choose a different visual treatment for a post in its slot (e.g., gazette → bulletin). |
| **Add manual post** | Create a new post from scratch. Editor picks a type, fills the form, places it in a slot. |
| **Spike a post** | Remove a post from the broadsheet without deleting it. It remains in the system for future use. |
| **Edit post content** | Click into any post card to open its CMS form (determined by type). |
| **Adjust priority** | Directly set a post's priority number (advanced — most editors will just reorder rows). |

---

## 8. Post Templates and Character Limits

### The Truncation Model

Post templates render posts on the **homepage broadsheet** with strict size constraints. Each post template defines:

- **Target character length** for the body text
- **Minimum character count** (posts shorter than this are fine — the template handles empty space)
- **Maximum character count** (posts longer than this are truncated with ellipsis `…`)

These constraints only apply to the **homepage broadsheet rendering**. The **post detail page** (the full page you see when you click a post) shows the complete content with no truncation.

This means:
- Authors/editors can write as much as they want in the body field.
- The homepage always looks clean — cards never overflow their slots.
- The detail page is where the full story, full item list, full schedule, etc. lives.
- "Read more" / "See full post" links on truncated cards take readers to the detail page.

### Example Character Limits by Post Template

These are starting points — adjust based on visual testing:

| Post Template | Body Target | Body Max | Title Max | Notes |
|--------------|------------|----------|-----------|-------|
| `feature` | 400 chars | 600 chars | 80 chars | Premium layout has room for more text |
| `gazette` | 200 chars | 280 chars | 60 chars | Standard card, 2-4 sentences |
| `ledger` | 120 chars | 160 chars | 50 chars | Compact listing, 1-2 sentences |
| `bulletin` | 180 chars | 240 chars | 60 chars | Slightly more room than ledger |
| `ticker` | 0 chars | 0 chars | 50 chars | Title only, no body shown |
| `digest` | 0 chars | 0 chars | 60 chars | Title only, no body shown |
| `feature-reversed` | 200 chars | 280 chars | 60 chars | Alert treatment, concise |

### Truncation Rules

1. **Truncate at word boundary.** Never cut a word in half. Find the last complete word within the max limit, append `…`.
2. **Items lists truncate by count.** If a Reference has 12 items and the template only shows 4, show 4 items + "and 8 more…" link.
3. **Schedule truncates to summary.** If the full schedule has 7 entries and the template only has room for a summary, show "Mon-Fri 9am-5pm" or the 2-3 most relevant entries.
4. **Tags render as space permits.** Show 1-3 tags, collapse the rest into "+N more" if needed.

---

## 9. CMS UI Requirements

### 9.1 Broadsheet Dashboard (Primary View)

The main CMS screen. Shows the current broadsheet as an editable layout preview.

**What the editor sees:**
- A vertical stack of **rows**, each showing its row template visually (column layout)
- Within each row, **post cards** in their slots — showing post type icon, title, first line of body, tags, weight indicator, and priority badge
- Drag handles on rows (reorder rows) and on post cards (move posts between compatible slots)
- A "pool" or sidebar of **unplaced posts** — posts from the glue layer that didn't fit in the layout, or posts the editor spiked, available to drag into any slot
- Visual indicators for reserved tags: `urgent` posts have a red accent on their card, `closed` posts are greyed out, `action` posts show a CTA badge

**Editor actions from this view:**
- Drag rows to reorder
- Drag post cards between compatible slots
- Click a post card → opens post editor (see §9.2)
- Click row template selector → change the row's column layout
- Click "+" → add a new manual post (pick type first, then form opens)
- Right-click or menu on post card → spike, change weight, change post template

### 9.2 Post Editor (Per-Type Forms)

Opens when the editor clicks a post card. The form varies by type.

**Common elements (all types):**
- Title field (always at top, always visible)
- Body field (rich text, always visible — size varies by type; large for Story, small for Notice)
- Tags field (autocomplete from existing tag pool, reserved tags shown separately)
- Type selector (dropdown — changing type re-arranges which field groups are open)
- Weight selector (heavy/medium/light — shows current default with option to override)
- Priority display (shows current priority number — usually set by glue layer, rarely edited directly)

**Field groups below the common elements:**
- Shown as collapsible sections
- Default-open sections determined by type
- Collapsed sections show a subtle "+" or expansion affordance
- The editor can expand any collapsed section to add fields

**The CMS should allow for different editing UIs per type.** The Story form might look dramatically different from the Exchange form. Story foregrounds a large rich text editor with formatting toolbar. Exchange foregrounds structured fields in a form grid with the body as a secondary textarea below. The forms don't need to share a layout — they share a data model, not a UI.

**Specific form notes by type:**

| Type | Form Notes |
|------|-----------|
| **Story** | Rich text editor takes up most of the form. Image upload prominent. Kicker field above title. Drop cap toggle. Two-column toggle. The editor is here to write/edit — make the writing experience great. |
| **Notice** | Short body field (2-3 lines visible). Source attribution fields prominent (name + context). Timestamp auto-filled but editable. Urgency escalation button/toggle visible: "Mark as urgent" → adds `urgent` tag, changes card treatment on dashboard. |
| **Exchange** | Structured form grid: contact fields, items list (add/remove rows), status dropdown, location. Direction toggle or tag selector for `need`/`aid`. Body is secondary — a small textarea for optional description. |
| **Event** | Date/time pickers prominent. Location field with optional map preview. Recurring toggle that shows/hides schedule pattern fields. Cost field. Body is secondary. |
| **Spotlight** | Two modes visible: "Person" and "Place/Business" (or toggled by which fields are filled). Person: name, role, bio, photo upload, quote. Place: name/tagline, hours grid, location, contact. Both can coexist. |
| **Reference** | Items list is the primary UI — an editable table of name + detail rows. "Last updated" date field. Contact, location, schedule as supporting fields. Body is secondary. |

### 9.3 Row Template Picker

Accessible from the broadsheet dashboard (click on a row's header/template indicator).

**Shows:** Visual thumbnails of available row templates with their slot layouts. Highlights which templates are compatible with the current row's posts (based on weight and type). Selecting a new template re-slots the row's posts.

### 9.4 Post Template Override

Accessible from a post card's context menu or the post editor.

**Shows:** Visual thumbnails of compatible post templates for this post's type. The current template is highlighted. Selecting a new template changes the visual treatment on the broadsheet preview.

### 9.5 New Post Flow

1. Editor clicks "+" (new post) from the dashboard or the unplaced pool
2. **Type selector** appears — 6 options with icons and short descriptions
3. After selecting a type, the post editor opens with the appropriate form preset
4. After saving, the post appears in the unplaced pool (or the editor can place it directly into a slot)

### 9.6 Widgets (Non-Post Elements)

Some elements on the broadsheet are not posts. They're persistent UI elements:

| Widget | Description | How It's Managed |
|--------|-------------|------------------|
| **Weather** | Daily forecast by county/city. Runs once daily. | Automated system, no editor input. Config: which locations to show. |
| **Hotline Bar** | Emergency numbers (911, 211, 988, community hotlines). | Managed as a settings/config page, not as a post. Rarely changes. |
| **Stat Cards** | "2,847 volunteers this month" — key community metrics. | Could be automated from Root Signal data or manually updated. Design TBD. |
| **Section Separators** | Visual dividers between thematic sections of the broadsheet. | Part of the row template system — some row templates include section headers. |

Widgets are placed in the broadsheet layout like posts but have their own configuration UIs (not post editor forms). They occupy slots in row templates but are a separate content type from posts.

---

## 10. Root Signal Integration

### How Content Flows

```
1. Root Signal continuously updates its graph (tensions, signals, evidence)
2. On a schedule (daily? on-demand?), the glue layer queries Root Signal:
   - "What are the highest-impact signals this week?"
   - "What new needs and aid have emerged?"
   - "What events are coming up?"
   - "Are there situations that need narrative coverage?"
3. The glue layer translates responses into Post objects:
   - Maps signal categories to post types
   - Sets priority from Root Signal impact scores
   - Sets weight from type defaults (possibly overriding for high-impact items)
   - Drafts body text via LLM (using signal data + evidence as source material)
   - Applies tags from signal metadata
4. The glue layer proposes a broadsheet:
   - Runs the layout engine to slot posts into row templates
   - Outputs a Broadsheet object
5. Root Editorial (CMS) receives the proposed broadsheet
6. The editor reviews, tweaks, publishes
```

### Default Signal-to-Type Mapping

The glue layer performs this mapping. These are defaults — the glue layer can override based on context.

| Root Signal | Post Type | Tags | Notes |
|-------------|-----------|------|-------|
| Tension | Story | topic-derived | Seed for a narrative. LLM drafts body from tension investigation data. |
| Situation | Story | topic-derived | Narrative wrapper. LLM drafts from situation summary. |
| Need | Exchange | `need` + topic | Direct mapping. Items/contact/status from signal data. |
| Aid | Exchange | `aid` + topic | Direct mapping. Items/contact/status from signal data. |
| Notice (low severity) | Notice | topic | Standard informational post. |
| Notice (high severity) | Notice | `urgent` + topic | Urgent treatment suggested. Editor confirms. |
| Gathering | Event | topic, maybe `recurring` | Date/time/location from signal data. |

### What the CMS Stores About Origin

Each post can optionally track its Root Signal origin for traceability:

```
origin: {
  signal_id:     string?    // Root Signal signal ID, if originated from a signal
  situation_id:  string?    // Root Signal situation ID, if originated from a situation
  generated:     boolean    // true if body text was LLM-drafted, false if manually written
  draft_source:  string?    // description of what data the LLM used to draft
}
```

This is metadata for the editor's reference — "where did this post come from?" It doesn't affect rendering or layout.

---

## 11. Design Principles and Flexibility

### Types Are Config, Not Architecture

The type system is a configuration table:

```
TYPE_CONFIG = {
  story:     { default_weight: "heavy",  default_groups: ["media", "meta"],                              templates: ["feature", "gazette", "digest"] },
  notice:    { default_weight: "light",  default_groups: ["meta", "source"],                             templates: ["gazette", "ledger", "bulletin", "ticker", "digest", "feature-reversed"] },
  exchange:  { default_weight: "medium", default_groups: ["contact", "items", "status"],                 templates: ["gazette", "ledger", "bulletin", "ticker"] },
  event:     { default_weight: "medium", default_groups: ["datetime", "location", "contact"],            templates: ["gazette", "ledger", "bulletin", "ticker", "feature"] },
  spotlight: { default_weight: "medium", default_groups: ["person", "media", "location", "contact"],     templates: ["feature", "gazette", "bulletin"] },
  reference: { default_weight: "medium", default_groups: ["items", "contact", "location", "schedule"],   templates: ["gazette", "ledger", "bulletin"] },
}
```

Adding type #7 = adding a row. Removing a type = reassigning posts and deleting the row. No schema migrations, no layout engine changes.

### Field Groups Are Additive

Any field group can appear on any post. The type config just says which are open by default. This means:
- A Story can have schedule/hours (e.g., a story about a business that includes their hours)
- A Notice can have a link/CTA (e.g., "Sign up for MNsure by Friday" with a registration link)
- An Event can have items (e.g., "Bring: warm clothing, canned goods")
- Any post can have any combination of field groups

This makes the system resilient to edge cases. When a post doesn't fit neatly into a type, the editor just opens the field groups they need.

### Tags Are the Escape Hatch

If something needs special treatment that doesn't warrant a new type, make it a tag. Tags can trigger:
- Visual treatment in the renderer (color accents, badges, icons)
- Filtering in the layout engine (grouping, section assignment)
- Filtering in the CMS dashboard (show me all urgent posts, all housing posts, all closed posts)

Adding a new reserved tag with special behavior is a renderer change, not a data model change.

### Everything Is Overridable

| What | Default Set By | Overridden By |
|------|---------------|---------------|
| Post type | Glue layer (from Root Signal category) | Editor (dropdown in post editor) |
| Post weight | Type config default | Editor (or glue layer, per post) |
| Post priority | Glue layer (from Root Signal scoring) | Editor (reorder rows, or manual number) |
| Open field groups | Type config default | Editor (expand/collapse any group) |
| Post template | Layout engine (from type compatibility) | Editor (pick from compatible templates) |
| Row template | Layout engine (from post weight distribution) | Editor (pick any row template) |
| Row order | Layout engine (from priority sorting) | Editor (drag rows) |
| Tags | Glue layer (from Root Signal metadata) | Editor (add/remove any tag) |

### Avoid Premature Specificity

Several decisions were intentionally deferred:
- Weight has 3 values. If 3 is too coarse, add a 4th when a concrete layout problem demonstrates it.
- Row template slots are weight-only by default. Add `accepts` filters only when auto-generated broadsheets are incoherent.
- CTA/Action is a tag + field group, not a type. If editors create CTAs weekly and wish for a dedicated form, add it then.
- The `origin` metadata (Root Signal traceability) is optional and informational. It can grow fields later without affecting anything.

---

## 12. Resolved Decisions

Decisions finalized during the design process, with rationale. For the full discussion trail, see [`POST_TYPE_SYSTEM.md`](POST_TYPE_SYSTEM.md).

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | **6 types, not 12** | Collapsed based on structural similarity of CMS forms. Don't make different types for the same data structure. |
| 2 | **Types are form presets, not rigid schemas** | Type determines which fields are open by default. Any field group can be toggled on/off on any type. The data model is the same across types. |
| 3 | **Tags instead of subtypes** | Subtypes like "Request-volunteer" and "Offer-housing" are tags, not types. Avoids combinatorial explosion. New needs emerge organically as tags. |
| 4 | **Need + Aid → Exchange** | Same CMS form, same editorial workflow. Direction indicated by `need`/`aid` tag. Layout can filter by tag for grouping. |
| 5 | **Alert merged into Notice** | Same form (short body + source). Urgency is a tag + priority level, not a type distinction. Removes the need for the glue layer to make a severity call — all notices arrive as Notice, editor decides prominence. |
| 6 | **Profile + Local → Spotlight** | Same editorial intent (feature a person/place this week). Same form structure. `person`/`business` tag distinguishes them. Renderer adapts to populated fields. |
| 7 | **CTA collapsed into `link` field group + `action` tag** | Any type can become a CTA. Not different enough editing experience to warrant its own type. Revisit if editors want a dedicated form. |
| 8 | **Urgent is a reserved tag, not a type** | Can apply to any type. An urgent Exchange, an urgent Notice, an urgent Event. Tag triggers visual treatment; priority controls placement. |
| 9 | **Weight: 3 values (heavy/medium/light)** | Maps to column width. Templates handle height via truncation. Start simple, add values when concrete layout problems emerge. |
| 10 | **Layout engine is dumb** | Intelligence in Root Signal + glue layer. Layout engine just sorts by priority and matches weight to slots. Simple to build, test, debug. |
| 11 | **Post templates enforce character limits with ellipsis truncation** | Homepage cards have strict size constraints. Detail pages show full content. Editors write freely; templates handle presentation. |
| 12 | **Field groups available on all types** | Type just sets defaults-open. Nothing locked out. Handles edge cases where a post needs fields from multiple types. |
| 13 | **CMS forms can differ per type** | Story editor looks different from Exchange editor. They share a data model, not a UI. Each form should be optimized for its editing workflow. |
| 14 | **Row template slots: weight-only by default, optional `accepts` filter** | Start simple. Add type restrictions to slots only when auto-generation produces incoherent groupings. |
| 15 | **Recurring events: `recurring` flag + optional `until` date** | Renderer shows schedule pattern. Auto-includes in future broadsheets. Editor controls lifecycle via `until` date or `closed` tag. |
| 16 | **Glue layer likely lives on Root Signal's side** | CMS receives posts, doesn't query Root Signal directly. The CMS doesn't need to know about Root Signal's internals. |
| 17 | **System designed for adaptability** | Types are config rows. Tags are free-form. Field groups are additive. Everything is overridable. Decisions are easy to reverse when real usage reveals what's needed. |

---

## 13. Deferred Decisions

Things we explicitly chose not to decide yet, with guidance on when to revisit.

| Topic | Current State | Revisit When |
|-------|--------------|-------------|
| **Whether CTA needs its own type** | Collapsed into `link` field group + `action` tag on any type | Editors consistently create CTAs and wish for a dedicated form. If it happens more than once per edition, consider adding. |
| **Whether weight needs more than 3 values** | 3 values: heavy/medium/light | Layout engine produces layouts where posts don't fit well — e.g., something clearly between medium and light that doesn't work in either slot. |
| **Stat cards / number blocks** | Listed as widgets, design TBD | CMS UI design reaches the widget management screen. Could be automated from Root Signal or manually curated. |
| **Tag management UI** | Tags are free-form, CMS suggests from existing pool | Tag proliferation becomes a problem (duplicates, misspellings, too many similar tags). Then build aliasing, merging, and "official" tag curation. |
| **How the glue layer decides severity** | All notices arrive as Notice type. Editor manually escalates to urgent. | Glue layer is built and needs a heuristic. Recommended: default to non-urgent, flag candidates with a "suggested escalation" badge, editor one-click promotes. |
| **Editorial lifecycle for recurring events** | `recurring` flag + optional `until` date + `closed` tag | Recurring events clog the broadsheet. Then add auto-aging, frequency controls, or "last featured" tracking. |
| **The broadsheet generation schedule** | Assumed weekly with possible daily updates | Editorial workflow is designed. Could be weekly batches, daily incremental updates, or on-demand regeneration. |
| **Detail page layout system** | Out of scope — this spec covers the homepage broadsheet | Detail page design phase. Detail pages show full content, no truncation. They probably use a different, simpler layout (not the row/slot system). |

---

## Appendix A: Type Quick Reference

| Type | Default Weight | Default Field Groups | Templates | Mental Model |
|------|---------------|---------------------|-----------|-------------|
| `story` | heavy | media, meta | feature, gazette, digest | "Read and shape prose" |
| `notice` | light | meta, source | gazette, ledger, bulletin, ticker, digest, feature-reversed | "How prominent?" |
| `exchange` | medium | contact, items, status | gazette, ledger, bulletin, ticker | "Is this verified?" |
| `event` | medium | datetime, location, contact | gazette, ledger, bulletin, ticker, feature | "Still upcoming?" |
| `spotlight` | medium | person, media, location, contact | feature, gazette, bulletin | "Right week to feature?" |
| `reference` | medium | items, contact, location, schedule | gazette, ledger, bulletin | "Still accurate?" |

---

## Appendix B: Field Group Quick Reference

| Group | Fields | Default On |
|-------|--------|-----------|
| `media` | image, caption, credit | story, spotlight |
| `contact` | phone, email, website | exchange, event, reference, spotlight |
| `location` | address, coordinates | event, reference, spotlight |
| `schedule` | entries[{day, opens, closes}] | reference, spotlight |
| `items` | [{name, detail}] | exchange, reference |
| `status` | state, verified | exchange |
| `datetime` | start, end, cost, recurring, until | event |
| `person` | name, role, bio, photo, quote | spotlight |
| `link` | label, url, deadline | (none by default) |
| `source` | name, attribution | notice |
| `meta` | kicker, byline, timestamp, updated | story, notice, reference |

---

## Appendix C: Mapping from Design Playground

The `mntogether-temp` repo is a visual design playground. Here's how its current types and components map to this spec.

### Renderer Types → Post Types

| Playground Type | → Spec Type | Notes |
|----------------|-------------|-------|
| `story` | `story` | Direct |
| `urgent` | `notice` + `urgent` tag | Was its own type, now a Notice with urgency |
| `update` | `notice` | Both updates and alerts are now Notice type |
| `event` | `event` | Direct |
| `request` | `exchange` + `need` tag | |
| `offer` | `exchange` + `aid` tag | |
| `resource` | `reference` | Renamed for clarity |
| `cta` | Any type + `link` group + `action` tag | Collapsed into field group + tag |
| `local` | `spotlight` + `business` tag | |
| `hero` | `story` via `feature` post template | Visual treatment, not a type |
| `profile` | `spotlight` + `person` tag | |
| `editorial` | `story` via `feature` post template | Visual treatment, not a type |
| `photo` | Any type with `media` group | Field group, not a type |

### Rendering Families → Post Templates

| Playground Family | → Post Template |
|------------------|----------------|
| Gazette (`gaz-`) | `gazette` |
| Ledger (`led-`) | `ledger` |
| Bulletin (`bul-`) | `bulletin` |
| Ticker (`tick-`) | `ticker` |
| Feature (`feat-`) | `feature` |
| Digest (`dig-`) | `digest` |
| Feature dark/reversed | `feature-reversed` |

### Widgets (Not Post Types)

| Playground Widget | Spec Status |
|------------------|-------------|
| Pull Quote | Inline element within Story body |
| Stat Card / Number Block | Widget — design TBD |
| Weather Forecast | Automated widget, own system |
| Resource Bar (hotlines) | Persistent UI element, settings page |
| Section Separator | Row template concern |

### Detail Page Mockups

| Mockup | Spec Type | Field Groups Used |
|--------|-----------|-------------------|
| `mockup-story.html` | story | media, meta |
| `mockup-event.html` | event | datetime, location, contact |
| `mockup-request.html` | exchange + `need` | schedule, contact, location, items |
| `mockup-offer.html` | exchange + `aid` | status, schedule, contact, location, items |
| `mockup-resource.html` | reference | schedule, location, contact, items |
| `mockup-cta.html` | story + `link` + `action` tag | link, contact, media, items |
| `mockup-local.html` | spotlight + `business` | schedule, contact, location |

---

## Appendix D: Example Broadsheet Data

A concrete example of what a broadsheet data object looks like after the layout engine runs:

```json
{
  "id": "edition-2026-03-01",
  "published": null,
  "rows": [
    {
      "row_template": "hero-with-sidebar",
      "slots": [
        {
          "post_id": "post-warming-center-story",
          "post_template": "feature",
          "post_summary": {
            "type": "story",
            "weight": "heavy",
            "priority": 95,
            "title": "Phillips Warming Center Saved 200 Lives This Winter",
            "tags": ["housing", "community-voices", "phillips"]
          }
        },
        {
          "post_id": "post-water-advisory",
          "post_template": "ticker",
          "post_summary": {
            "type": "notice",
            "weight": "light",
            "priority": 90,
            "title": "Boil Water Advisory: North Minneapolis",
            "tags": ["urgent", "safety", "north-minneapolis"]
          }
        },
        {
          "post_id": "post-mnsure-deadline",
          "post_template": "ticker",
          "post_summary": {
            "type": "notice",
            "weight": "light",
            "priority": 80,
            "title": "MNsure Open Enrollment Ends Friday",
            "tags": ["health"]
          }
        }
      ]
    },
    {
      "row_template": "three-column",
      "slots": [
        {
          "post_id": "post-food-drive-event",
          "post_template": "gazette",
          "post_summary": {
            "type": "event",
            "weight": "medium",
            "priority": 75,
            "title": "Lake Street Food Drive This Saturday",
            "tags": ["food", "lake-street"]
          }
        },
        {
          "post_id": "post-coats-needed",
          "post_template": "gazette",
          "post_summary": {
            "type": "exchange",
            "weight": "medium",
            "priority": 70,
            "title": "Winter Coats Needed for Families",
            "tags": ["need", "urgent", "winter-gear", "donations"]
          }
        },
        {
          "post_id": "post-legal-clinic",
          "post_template": "bulletin",
          "post_summary": {
            "type": "exchange",
            "weight": "medium",
            "priority": 65,
            "title": "Free Immigration Legal Help Available",
            "tags": ["aid", "legal", "immigration"]
          }
        }
      ]
    },
    {
      "row_template": "classifieds",
      "slots": [
        {
          "post_id": "post-esl-tutors",
          "post_template": "ledger",
          "post_summary": {
            "type": "exchange",
            "weight": "light",
            "priority": 55,
            "title": "ESL Tutors Needed — Cedar Riverside",
            "tags": ["need", "education", "language-access", "cedar-riverside"]
          }
        },
        {
          "post_id": "post-furniture-available",
          "post_template": "ledger",
          "post_summary": {
            "type": "exchange",
            "weight": "light",
            "priority": 50,
            "title": "Free Furniture: Couch, Table, Chairs",
            "tags": ["aid", "housing"]
          }
        },
        {
          "post_id": "post-drivers-needed",
          "post_template": "ledger",
          "post_summary": {
            "type": "exchange",
            "weight": "light",
            "priority": 45,
            "title": "Volunteer Drivers for Medical Appointments",
            "tags": ["need", "transit", "health"]
          }
        },
        {
          "post_id": "post-childcare-available",
          "post_template": "ledger",
          "post_summary": {
            "type": "exchange",
            "weight": "light",
            "priority": 40,
            "title": "Drop-In Childcare: Spots Available",
            "tags": ["aid", "childcare"]
          }
        }
      ]
    },
    {
      "row_template": "single-medium",
      "slots": [
        {
          "post_id": "post-buen-dia-bakery",
          "post_template": "gazette",
          "post_summary": {
            "type": "spotlight",
            "weight": "medium",
            "priority": 35,
            "title": "Buen Dia Bakery: A Lake Street Gem",
            "tags": ["business", "restaurant", "lake-street"]
          }
        }
      ]
    },
    {
      "row_template": "three-column",
      "slots": [
        {
          "post_id": "post-hennepin-food-shelves",
          "post_template": "gazette",
          "post_summary": {
            "type": "reference",
            "weight": "medium",
            "priority": 30,
            "title": "Hennepin County Food Shelves",
            "tags": ["food", "hennepin-county"]
          }
        },
        {
          "post_id": "post-free-clinics",
          "post_template": "gazette",
          "post_summary": {
            "type": "reference",
            "weight": "medium",
            "priority": 28,
            "title": "5 Free Health Clinics Near You",
            "tags": ["health"]
          }
        },
        {
          "post_id": "post-tax-prep-sites",
          "post_template": "gazette",
          "post_summary": {
            "type": "reference",
            "weight": "medium",
            "priority": 25,
            "title": "Free Tax Preparation Sites 2026",
            "tags": ["employment"]
          }
        }
      ]
    }
  ]
}
```

This is what the CMS receives from the layout engine and presents to the editor as the broadsheet dashboard. Every aspect of it — row order, post placement, template choice — is editable.
