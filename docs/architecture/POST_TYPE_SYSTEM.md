# Post Type System — Design Rationale

> **Status:** Design rationale (finalized decisions)
> **Date:** 2026-02-25
> **Finalized spec:** [`CMS_SYSTEM_SPEC.md`](CMS_SYSTEM_SPEC.md) — the authoritative specification built from these discussions
> **Context:** This document captures the design reasoning behind the post type system, layout engine, and tag model used in Root Editorial. It exists as a reference for *why* decisions were made. For the *what*, see the CMS System Spec.

---

## Table of Contents

1. [System Architecture Context](#system-architecture-context)
2. [What "Type" Means in This System](#what-type-means-in-this-system)
3. [The 6 Post Types](#the-6-post-types)
4. [Post Data Structure](#post-data-structure)
5. [Field Groups (Optional Blocks)](#field-groups-optional-blocks)
6. [Tags (Free-Form, Multi-Select)](#tags-free-form-multi-select)
7. [Layout System: Rows, Slots, and Templates](#layout-system-rows-slots-and-templates)
8. [Where Intelligence Lives](#where-intelligence-lives)
9. [How Root Signal Maps to Post Types](#how-root-signal-maps-to-post-types)
10. [Resolved Questions](#resolved-questions)
11. [Decision Log](#decision-log)

---

## System Architecture Context

There are three layers, plus a glue system between the first two:

```
Root Signal          (data engine — situations, signals, evidence)
     │
     ▼
  [Glue Layer]       (queries Root Signal, proposes a broadsheet of posts)
     │
     ▼
Root Editorial       (CMS — editor reviews, tweaks, approves, adds manual content)
     │
     ▼
NextJS MN Together   (reader-facing frontend — the broadsheet)
```

### Root Signal's Data Model

Root Signal produces a 3-layer hierarchy:

- **Situations** — narrative wrapper around what's happening. Lives as long as the underlying tension does.
- **Signals** — the smallest atomic observation. Bucketed into categories:
  - **Tensions** — the underlying "why." Everything points back to tensions. Includes investigation into why the tension exists.
  - **Needs** — someone/something needs help, resources, action
  - **Aid** — help, resources, or services being offered
  - **Notices** — informational items, advisories, announcements
  - **Gatherings** — events, meetings, community moments
- **Evidence** — source links backing up signals (articles, GoFundMes, social posts, official notices, etc.)

Key characteristic: Root Signal supports natural-language queries against a graph database.

### Root Editorial's Role

Root Editorial is a CMS designed for:

1. Non-tech-savvy community leader types (nice GUI, markdown support for manual stories)
2. Serving as a daily/weekly semi-automated digital broadsheet
3. First-class integration with Root Signal
4. Auto-generated preview of the day/week "print" — sourced from Root Signal, with editorial control over prominence and placement
5. A layout engine that understands multi-column layout and makes placement decisions
6. Human-in-the-loop editorial review — the AI produces a ~95% ready broadsheet, the editor spends 1-2 hours/week tweaking

### Key Principle

Any and all content could be LLM-written, sourced in whole or in part from Root Signal. But it can also be fully manual/editorial. The type system must support both origins equally.

---

## What "Type" Means in This System

Post types serve **three purposes simultaneously**:

### 1. CMS Form Template
What input fields does the editor see when creating/editing this post? A long-form Story editor is a different UI experience than a short-form Notice editor or a structured Exchange editor. The type determines which field groups are open by default in the CMS form.

### 2. Layout Engine Compatibility
What "shape" is this post for the purposes of auto-placement? The layout engine needs to know what row slots a post can fit into and what post templates can render it. The type provides this via static compatibility mappings.

### 3. Post Template Compatibility
What visual treatments can render this type? Not every post template works for every type — a ticker treatment works for a Notice but not for a Story with pull quotes and images.

### What Type is NOT

- **Not a topic/category.** "Housing" is a tag, not a type. A Story about housing and a Need for housing are different types with the same tag.
- **Not a visual style.** "Gazette" and "ledger" are post templates (visual treatments), not types. A single type can render in multiple visual styles.
- **Not a Root Signal category.** Root Signal has its own taxonomy (tensions, needs, aid, notices, gatherings). The glue layer maps these to post types, but the mapping isn't 1:1.

---

## The 6 Post Types

### 1. Story
**What it is:** Long-form narrative content. Community voices, feature articles, investigative pieces, profiles-in-depth, explainers.

**CMS editing experience:** Rich text editor as the primary input. Multi-paragraph body with formatting (bold, italic, links, block quotes). Optional image, caption, pull quotes. Kicker field for topic labeling. This is the "writing" experience — the editor will read and shape prose.

**Layout weight default:** `heavy` — typically takes a full column or feature zone.

**Compatible post templates:** feature, gazette, digest

**Editor's mental model:** "I need to read this and decide if the writing is ready."

**Could originate from:** A Root Signal Situation (auto-drafted narrative around a tension), an editor's original reporting, an LLM draft sourced from multiple signals, a community-submitted piece.

---

### 2. Notice
**What it is:** Short, timely informational content. Covers the full spectrum from neutral updates ("MNsure deadline Friday") to critical alerts ("Boil water advisory"). The urgency level is handled by tags and priority, not by splitting into separate types.

**CMS editing experience:** Short body field (a few sentences, not paragraphs). Timestamp field. Source attribution fields (who issued this — city, county, utility, community org). This is the lightest editing experience — "title, sentence or two, check the source, done."

**Layout weight default:** `light` — stacks in ticker rows, digest lists, sidebar briefs.

**Compatible post templates:** gazette, ledger, bulletin, ticker, digest, feature-reversed (for urgent treatment)

**Editor's mental model:** "Is this current? How prominent should it be?"

**How urgency works within Notice:**
- A routine update is a Notice with low priority and no special tags.
- An alert is a Notice with the `urgent` tag, high priority, and source attribution. The `urgent` tag triggers high-contrast visual treatment (dark background, red accent) and elevated placement.
- The editor can escalate any Notice to urgent, or de-escalate, by toggling the tag and adjusting priority. No type change needed.

**Why Alert and Update were merged into Notice:** The CMS form is nearly identical for both (short body + optional source). The difference is urgency level, which is a tag. Keeping them as one type avoids the glue layer needing to make a severity judgment call — all notices arrive as Notice type, the editor decides prominence. See [Decision Log](#decision-log).

**Could originate from:** Root Signal Notices (any severity), automated deadline tracking, official government advisories, community safety reports.

---

### 3. Exchange
**What it is:** A community need or offer — someone/something needs resources, or someone/something has resources to give. This type covers both directions because the CMS form is structurally identical; the direction (need vs. offer) is indicated by a tag.

**CMS editing experience:** Structured fields as the primary input (not prose). Contact information, list of items needed/available, status/availability, optional location and hours/schedule. The editor verifies details and checks status rather than editing prose.

**Layout weight default:** `medium` — fits in classifieds columns, community sections, card grids.

**Compatible post templates:** gazette, ledger, bulletin, ticker

**Editor's mental model:** "Is this verified? Is it still active/available?"

**Direction is a tag, not a type distinction:**
- Tag `need` → "Volunteers Needed", "Donations Needed", "Housing Needed"
- Tag `aid` → "Available", "Offering", "Free"
- The layout engine can query by tag to group needs separately from aid if desired

**Could originate from:** Root Signal Needs or Aid signals, community submissions, organizational postings, mutual aid networks.

**Design decision — why collapse Need + Aid into Exchange:**
The CMS form is identical for both. Same fields: contact, items, status, schedule. The editorial workflow is the same: verify details, check status, decide placement. Keeping them as one type with a direction tag means fewer types to maintain, and the layout engine can still separate them by querying tags.

---

### 4. Event
**What it is:** Something happening at a specific time and place. Community gatherings, workshops, meetings, drives, fairs. Calendar-shaped content.

**CMS editing experience:** Date/time picker is the prominent UI element. Location/address field. Description body. Recurring toggle (weekly ESL class vs. one-time food drive). Optional cost, contact, registration link.

**Layout weight default:** `medium` — fits in calendar sidebars, event rows, card grids.

**Compatible post templates:** gazette, ledger, bulletin, ticker, feature

**Editor's mental model:** "Is this still upcoming? Worth featuring?"

**Recurring events:** The `datetime` field group has a `recurring` toggle. When on, the renderer shows a schedule pattern ("Mon 6-7:30pm") instead of a single date. The `recurring` reserved tag is also applied. The editor can set an optional `until` date for auto-expiration, tag `closed` to end it, or simply spike it from a given week.

**Could originate from:** Root Signal Gatherings, community calendar submissions, organizational event postings.

---

### 5. Spotlight
**What it is:** A feature profile of a person, business, or organization. "Neighbor to Know," business listings, organizational profiles. Human interest content that's more structured than a Story but more personal than a Reference.

**CMS editing experience:** Person-specific fields (name, role, bio, photo, quote) OR business-specific fields (tagline, hours, location, contact). Media/image upload. The editing experience is "curating a profile" — assembling structured info about a subject.

**Layout weight default:** `medium` (can be overridden to `heavy` for feature profiles)

**Compatible post templates:** feature, gazette, bulletin

**Editor's mental model:** "Is this the right week to feature this?"

**People vs. places:** Distinguished by which fields are populated and by tags (`person` vs. `business`). If `person` fields are filled (name, role, bio, quote), render as a community profile. If business fields are filled (tagline, hours, location), render as a business listing. Both can coexist.

**Design decision — why merge Profile + Local into Spotlight:**
The CMS form is similar: both involve assembling structured info about a subject. The editorial intent is the same: "let's feature this person/place this week." A `person` or `business` tag distinguishes them for filtering. The renderer adapts based on which field groups are populated.

---

### 6. Reference
**What it is:** Evergreen, structured information. Directories, resource lists, guides, how-tos. Low churn — updated periodically, not daily. The "clip and save" content.

**CMS editing experience:** Structured item list as the primary input (name + detail per row, potentially multi-column). Contact, location, hours/schedule fields. "Last updated" / freshness date. The editor's job is verification: "are these phone numbers still correct? Are these hours current?"

**Layout weight default:** `medium` — fits in resource sections, sidebars, reference blocks.

**Compatible post templates:** gazette, ledger, bulletin

**Editor's mental model:** "Is this still accurate? When was it last verified?"

**Could originate from:** Curated directories, organizational resource pages, LLM-compiled reference guides, editor-maintained lists.

---

## Post Data Structure

### Universal Fields (every post has these)

```
Post {
  id:          string         // unique identifier
  type:        PostType       // one of 6 values (see above)
  tags:        string[]       // free-form, multi-select

  // Layout metadata
  weight:      "heavy" | "medium" | "light"   // default from type, editor-overridable
  priority:    number         // higher = more important, set by glue layer, editor-overridable

  // Universal content
  title:       string         // every post has a title
  body:        string         // rich text, can be one sentence or ten paragraphs
}
```

### The `weight` field

Weight tells the layout engine how much space this post needs. It determines which row slots the post can fill.

- **heavy** — needs a full column or feature-width slot. Stories with images, in-depth profiles, major features.
- **medium** — fits a standard card/column slot. Most exchanges, events, references, spotlight cards.
- **light** — fits a compact/ticker slot. Notices, brief exchanges, short items.

The type sets the default weight, but the editor (or glue layer) can override. A short story could be `medium`. A critical urgent notice could be `heavy`. A detailed exchange with many items could be `heavy`.

### The `priority` field

Priority determines ordering. Higher priority = closer to the top of the broadsheet. The glue layer sets initial priority based on Root Signal scoring (impact, recency, severity). The editor overrides by reordering rows or manually adjusting priority.

Priority is a single number, not a multi-factor score. The glue layer is responsible for collapsing Root Signal's multidimensional scoring into a single priority value. The layout engine doesn't need to understand why something is high-priority — just that it is.

---

## Field Groups (Optional Blocks)

Every field group is available on every post type. The type determines which groups are **open by default** in the CMS form, but the editor can always expand or collapse any group. Nothing is locked out — a Story could have Hours, a Notice could have Contact.

### media
```
{
  image:    string    // image URL or upload reference
  caption:  string    // image caption
  credit:   string    // photographer/source credit
}
```
**Default open on:** Story, Spotlight

### contact
```
{
  phone:    string
  email:    string
  website:  string
}
```
**Default open on:** Exchange, Event, Reference, Spotlight

### location
```
{
  address:      string    // human-readable address
  coordinates:  [lat, lng]  // for map rendering
}
```
**Default open on:** Event, Reference, Spotlight

### schedule
```
{
  entries: [
    { day: "Monday", opens: "9:00 AM", closes: "5:00 PM" },
    { day: "Tuesday", opens: "9:00 AM", closes: "5:00 PM" },
    ...
  ]
}
```
**Default open on:** Reference, Spotlight (business)

### items
```
[
  { name: "Winter coats (adult)", detail: "Sizes M-XXL, new or gently used" },
  { name: "Children's boots", detail: "All sizes needed" },
  ...
]
```
**Default open on:** Exchange, Reference

### status
```
{
  state:     string    // freeform: "Available now", "Needed", "Closed", "By referral"
  verified:  date      // when status was last confirmed
}
```
**Default open on:** Exchange

### datetime
```
{
  start:     datetime
  end:       datetime
  cost:      string    // "Free", "$5", "Sliding scale", etc.
  recurring: boolean   // if true, renderer shows schedule pattern instead of one-off date
  until:     date      // optional — auto-expire recurring events after this date
}
```
**Default open on:** Event

### person
```
{
  name:   string
  role:   string    // "Community Organizer", "Owner", etc.
  bio:    string    // short biographical text
  photo:  string    // image URL or upload reference
  quote:  string    // pull quote or testimonial
}
```
**Default open on:** Spotlight

### link
```
{
  label:     string    // button text: "Sign the petition", "Register to vote"
  url:       string
  deadline:  date      // "Action needed by March 12"
}
```
**Default open on:** (none by default — toggled open when needed, especially for Action-oriented content)

**Design note:** Call to Action (CTA) was considered as its own type but collapsed. A CTA is typically a Story or Notice with a `link` field group toggled open and an `action` tag. If the editorial team finds they're creating CTAs frequently enough that it warrants a dedicated form preset, this could be revisited.

### source
```
{
  name:         string    // "City of Minneapolis", "Hennepin County", "Community Report"
  attribution:  string    // additional source context
}
```
**Default open on:** Notice

### meta
```
{
  kicker:     string    // topic label displayed above title ("Community Voices", "Housing")
  byline:     string    // author attribution
  timestamp:  datetime  // when published or last updated
  updated:    string    // freshness label ("Updated weekly", "Updated Feb 2026")
}
```
**Default open on:** Story (kicker, byline), Notice (timestamp), Reference (updated)

---

## Tags (Free-Form, Multi-Select)

Tags are not a closed set. New tags emerge as content demands them. Tags serve three purposes:

### 1. Topic/Domain Tags
Describe what the post is about. Used for filtering, search, and discovery.

Common examples (not exhaustive — this list grows organically):
> housing, food, health, education, immigration, legal, environment, transit, safety, employment, youth, seniors, disability, language-access, resettlement, childcare, animals, winter-gear, voting, census

### 2. Reserved Tags (trigger visual/behavioral changes)
A small set of tags with special meaning in the rendering and layout systems:

| Tag | Effect |
|-----|--------|
| `urgent` | High-contrast visual treatment (dark background, red accent), elevated placement priority |
| `recurring` | Schedule display instead of one-off date, auto-include in future broadsheets |
| `closed` | Greyed-out treatment, "fulfilled" / "expired" badge |
| `need` | Direction indicator on Exchange type — "something is needed" |
| `aid` | Direction indicator on Exchange type — "something is available" |
| `action` | Triggers CTA rendering — prominent link button, deadline display |
| `person` | On Spotlight type: render as community member profile |
| `business` | On Spotlight type: render as business/org listing |

### 3. Geographic/Community Tags
Where this is relevant. Used for geographic filtering.

> north-minneapolis, phillips, lake-street, midtown, cedar-riverside, hennepin-county, ramsey-county, statewide

### Tag Management (Future)
Eventually the CMS would benefit from:
- Suggested tags when creating a post (drawn from existing popular tags)
- Tag merging/aliasing ("winter gear" = "winter-gear" = "cold weather supplies")
- A small set of "official" tags that map to filter UI icons or colors
- Tag analytics (trending tags, tag frequency)

This is a product decision for later, not an architecture decision now. The data model is the same regardless.

---

## Layout System: Rows, Slots, and Templates

### Three-Layer Rendering Model

```
BROADSHEET
  └── ROW (row template — defines column grid, ordered by importance)
        └── SLOT (within the row — defines size constraint)
              └── POST (post template — visual treatment of the content)
                    └── POST DATA (type + fields + tags)
```

### Row Templates

A row template defines a column layout with typed slots. Examples:

```
"hero-with-sidebar" {
  description: "Full-width feature with stacked sidebar items"
  slots: [
    { weight: "heavy", count: 1 },
    { weight: "light", count: 3 }
  ]
}

"three-column" {
  description: "Three equal columns"
  slots: [
    { weight: "medium", count: 3 }
  ]
}

"two-column-wide-narrow" {
  description: "Wide left column, narrow right column"
  slots: [
    { weight: "heavy", count: 1 },
    { weight: "medium", count: 1 }
  ]
}

"classifieds" {
  description: "Dense multi-column listings"
  slots: [
    { weight: "light", count: 4-6 }
  ]
}

"ticker" {
  description: "Horizontal strip of compact items"
  slots: [
    { weight: "light", count: 5-8 }
  ]
}
```

Row order = editorial importance. Top row = most important. The editor reorders rows by dragging.

Slot `accepts` filters are optional — most slots use weight-only matching. Add type restrictions only when auto-generated broadsheets produce incoherent groupings.

### Post Templates

A post template defines the visual treatment of a single post within a slot:

- **feature** — premium editorial treatment, large typography, dramatic layout
- **feature-reversed** — dark/high-contrast treatment, used for urgent notices
- **gazette** — top-border tabbed frame, colored accent, standard card
- **ledger** — left-border tabbed, classifieds feel, compact listing
- **bulletin** — boxed card, community board feel
- **ticker** — ultra-compact single-line treatment
- **digest** — headline-only, no body text

Each post template declares which types it can render:

```
feature:          [story, event, spotlight]
feature-reversed: [notice]
gazette:          [story, notice, exchange, event, reference, spotlight]  (all 6)
ledger:           [notice, exchange, event, reference]
bulletin:         [notice, exchange, event, reference, spotlight]
ticker:           [notice, exchange, event]
digest:           [story, notice, exchange]
```

Post templates enforce character limits on the homepage broadsheet. Detail pages show full content with no truncation. See [CMS_SYSTEM_SPEC.md §8](CMS_SYSTEM_SPEC.md#8-post-templates-and-character-limits) for the truncation model.

### How the Layout Engine Works

The layout engine is deliberately dumb. It's a slot-filler, not a decision-maker.

```
INPUT:
  - List of posts, each with: type, weight, priority, tags
  - Library of row templates
  - Library of post templates (with type compatibility)

ALGORITHM:
  1. Sort posts by priority (descending)
  2. Walk down the sorted list
  3. For each post:
     a. Find row templates with available slots matching post.weight
     b. Within those slots, check that at least one post template is
        compatible with post.type
     c. Place post in the best available slot
  4. Order rows by the highest-priority post they contain
  5. Output the broadsheet

OUTPUT:
  Broadsheet = [
    {
      row_template: "hero-with-sidebar",
      slots: [
        { post: post_123, post_template: "feature" },
        { post: post_456, post_template: "ticker" },
        { post: post_789, post_template: "ticker" },
      ]
    },
    ...
  ]
```

### What the Editor Can Do

After the layout engine proposes a broadsheet:

- **Reorder rows** — drag rows up/down to change importance
- **Swap posts between compatible slots** — move a post from one slot to another (if weight and type are compatible)
- **Change a post's weight** — make a short story `medium` instead of `heavy` to fit a different slot
- **Change a row template** — switch from 2-column to 3-column for a row
- **Add manual posts** — create a new post from scratch, placed in any compatible slot
- **Remove/spike posts** — take a post out of the broadsheet (doesn't delete it, just unpublishes)
- **Edit post content** — click into any post to open its CMS form (determined by type)
- **Override post template** — choose a different visual treatment for a post within its slot

---

## Where Intelligence Lives

This is a critical design principle: **intelligence is pushed to the edges, not the middle.**

| Layer | Intelligence Level | Responsibility |
|-------|-------------------|----------------|
| **Root Signal** | High | Graph database analysis, signal detection, situation narrative, impact scoring, natural-language queries |
| **Glue Layer** | Medium | Maps Root Signal output to post types, sets initial priority/weight, drafts body text via LLM, proposes tags, creates the initial broadsheet proposal |
| **Layout Engine** | Low (dumb by design) | Slot-filling algorithm. Matches posts to row slots by weight and type compatibility. Orders rows by priority. No content understanding. |
| **Editor (human)** | High | Reviews proposed broadsheet, reorders, edits, adds manual content, makes judgment calls about prominence and tone. Spends ~1-2 hrs/week. |
| **Renderer** | Low (dumb by design) | Takes broadsheet structure, applies post templates, outputs HTML. No content understanding. |

### Why the Layout Engine Should Be Dumb

The glue layer has access to Root Signal's full graph — it understands tensions, relationships between signals, community impact, temporal urgency. It collapses all of this into two simple numbers: `priority` and `weight`. The layout engine doesn't need to re-derive any of that. It just sorts and slots.

This means:
- The layout engine is simple to build and test
- Layout bugs are easy to diagnose (it's just sorting and matching)
- All the "smart" decisions are made upstream where the data is richest
- The editor has clear, simple levers to pull (priority, weight, row order)

---

## How Root Signal Maps to Post Types

This mapping is performed by the glue layer, not hard-coded. But here's the expected default mapping:

| Root Signal Category | Default Post Type | Default Tags | Notes |
|---------------------|-------------------|--------------|-------|
| **Tension** | Story | (topic-derived) | A tension is the "why" behind everything. Becomes the seed for a narrative, possibly auto-drafted by LLM. |
| **Need** | Exchange | `need` + topic tags | Direct mapping. Items/contact/status populated from signal data. |
| **Aid** | Exchange | `aid` + topic tags | Direct mapping. Items/contact/status populated from signal data. |
| **Notice** (low severity) | Notice | topic tags | Standard informational post. |
| **Notice** (high severity) | Notice | `urgent` + topic tags | Urgent treatment suggested. Editor confirms. |
| **Gathering** | Event | topic tags, possibly `recurring` | Direct mapping. Date/time/location populated from signal data. |
| **Situation** | Story | topic tags | A situation narrative wraps multiple signals into a story. The glue layer may draft the body text from the situation summary. |

### Signals That Don't Become Posts

Not every Root Signal signal needs to become a post. The glue layer filters based on:
- Impact score (below a threshold → don't surface)
- Recency (stale signals → don't surface unless the underlying tension is active)
- Redundancy (multiple signals about the same thing → consolidate into one post)
- Editorial rules (e.g., "never auto-publish ICE-related alerts without human review")

### Posts That Don't Come from Root Signal

Some posts are purely editorial:
- Hand-written Stories about community members or issues
- Spotlight profiles of neighbors or businesses
- Curated Reference guides (food shelf directories, clinic lists)
- Manual Events from community submissions
- Editor-authored Notices about editorial matters

These are created directly in the CMS with no Root Signal origin. The type system treats them identically.

### Origin Metadata

Each post can optionally track its Root Signal origin for traceability:

```
origin: {
  signal_id:     string?    // Root Signal signal ID
  situation_id:  string?    // Root Signal situation ID
  generated:     boolean    // true if body text was LLM-drafted
  draft_source:  string?    // description of what data the LLM used
}
```

This is metadata for the editor's reference — "where did this post come from?" It doesn't affect rendering or layout.

---

## Resolved Questions

Questions that arose during the design conversation and have been resolved.

### Weight vs. size
**Question:** Does the layout engine need to know physical pixel size, or is `weight` (column width) sufficient?

**Resolution:** Weight-only. Post templates handle variable content height through truncation (see [CMS_SYSTEM_SPEC.md §8](CMS_SYSTEM_SPEC.md#8-post-templates-and-character-limits)). The layout engine cares about column width, not pixel height. Three values (heavy/medium/light) are sufficient to start.

### CTA as a type
**Question:** Should Call to Action be its own post type?

**Resolution:** Collapsed. CTA is achieved via the `link` field group + an `action` reserved tag on any type. The editing experience isn't different enough from other types to warrant its own type. Revisit if editors create CTAs frequently enough to want a dedicated form preset.

### Alert vs. Update vs. Notice
**Question:** Should alerts and updates be separate types, or merged?

**Resolution:** Merged into **Notice**. The CMS form is nearly identical (short body + source). The difference is urgency level, which is better expressed as a tag (`urgent`) and priority level than as a type distinction. This avoids the glue layer needing to make a severity judgment call — all Root Signal notices arrive as Notice type, the editor decides prominence.

### Source field group placement
**Question:** Should `source` be its own field group or part of `meta`?

**Resolution:** Keep separate. Notices benefit from prominent source display ("City of Minneapolis", "Hennepin County Health"). Having it as its own field group means the CMS form can show it prominently.

### Row template slot granularity
**Question:** Should slot definitions include type/tag restrictions?

**Resolution:** Start weight-only. Slots optionally accept an `accepts` filter for type restrictions, but this is off by default. Add restrictions only when auto-generated broadsheets produce incoherent groupings.

### Recurring event lifecycle
**Question:** Do recurring events auto-expire?

**Resolution:** The `datetime` field group includes an optional `until` date for auto-expiration. The `closed` tag can end a recurring event immediately. The broadsheet builder auto-includes active recurring events each week. The editor can spike a recurring event from a given week without closing it.

---

## Decision Log

Decisions made during the design process, with rationale.

| Decision | Rationale |
|----------|-----------|
| **6 types, not 12** | Collapsed based on structural similarity of CMS forms and layout behavior. Fewer types = simpler system. |
| **Alert + Update → Notice** | Same CMS form (short body + source). Urgency is a tag + priority, not a type. Removes the severity judgment from the glue layer. |
| **Need + Aid → Exchange** | Same CMS form, same fields, same editorial workflow. Direction indicated by `need`/`aid` tag. Layout engine can filter by tag if separate zones are wanted. |
| **Profile + Local → Spotlight** | Similar CMS form (structured info about a subject). `person`/`business` tag distinguishes them. Renderer adapts based on which field groups are populated. |
| **Tags instead of subtypes** | Request/Offer subcategories (volunteer, donation, housing, childcare, transport) are free-form tags, not enumerated subtypes. Avoids type explosion as new needs emerge. |
| **Urgent is a reserved tag, not a type** | Can apply to any type (urgent Exchange, urgent Notice, urgent Event). Tag triggers visual treatment; priority controls placement. |
| **CTA collapsed into link field group + action tag** | The editing experience isn't different enough from other types to warrant its own type. Any type + link field group + action tag = CTA behavior. May revisit. |
| **Field groups are available on all types** | Type sets defaults-open, but nothing is locked out. A Story can have Hours. A Notice can have Contact. Flexibility over rigidity. |
| **Weight is a default from type, editor-overridable** | The layout engine needs weight for slot-matching, but the editor/glue layer can override for specific posts that don't fit the type's default. |
| **Layout engine is dumb by design** | Intelligence lives in Root Signal (analysis) and the glue layer (mapping/prioritization). The layout engine just sorts and slots. Simple to build, test, and debug. |
| **Tags are free-form, not a closed set** | New topics, geographies, and community needs emerge organically. A small set of reserved tags trigger special behavior; everything else is pure filtering/discovery. |
| **Types are config, not architecture** | Adding a 7th type = adding a row to the type config. No schema migration, no layout engine changes. Types are cheap to add or remove. |
