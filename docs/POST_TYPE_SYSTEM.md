# Post Type System — Design Discussion

> **Status:** Active design discussion, not finalized
> **Date:** 2026-02-25
> **Context:** This document captures the evolving design conversation about how post types, tags, layout, and CMS UI relate to each other across Root Signal, Root Editorial, and the MN Together frontend.

---

## Table of Contents

1. [System Architecture Context](#system-architecture-context)
2. [What "Type" Means in This System](#what-type-means-in-this-system)
3. [The 7 Post Types](#the-7-post-types)
4. [Post Data Structure](#post-data-structure)
5. [Field Groups (Optional Blocks)](#field-groups-optional-blocks)
6. [Tags (Free-Form, Multi-Select)](#tags-free-form-multi-select)
7. [Layout System: Rows, Slots, and Templates](#layout-system-rows-slots-and-templates)
8. [Where Intelligence Lives](#where-intelligence-lives)
9. [How Root Signal Maps to Post Types](#how-root-signal-maps-to-post-types)
10. [Open Questions](#open-questions)
11. [Decision Log](#decision-log)
12. [Appendix: Current Codebase Audit](#appendix-current-codebase-audit)

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

Key characteristic: Root Signal supports natural-language queries against a graph database. Example queries like "give me all high impact volunteer opportunities" or "tell me 5 things I can do today that can have an impact on the situation" are intended to work.

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

### What This Design Playground Is

This repo (`mntogether-temp`) is a design iteration playground for visual/UI exploration. It is NOT representative of the structure or architecture of the main repos. The component families, rendering variants, and dummy data here are explorations of visual treatment, not production code.

---

## What "Type" Means in This System

Post types serve **three purposes simultaneously**:

### 1. CMS Form Template
What input fields does the editor see when creating/editing this post? A long-form Story editor is a different UI experience than a short-form Alert editor or a structured Exchange editor. The type determines which field groups are open by default in the CMS form.

### 2. Layout Engine Compatibility
What "shape" is this post for the purposes of auto-placement? The layout engine needs to know what row slots a post can fit into and what post templates can render it. The type provides this via static compatibility mappings.

### 3. Post Template Compatibility
What visual treatments can render this type? Not every post template works for every type — a ticker treatment works for an Update but not for a Story with pull quotes and images.

### What Type is NOT

- **Not a topic/category.** "Housing" is a tag, not a type. A Story about housing and a Need for housing are different types with the same tag.
- **Not a visual style.** "Gazette" and "ledger" are post templates (visual treatments), not types. A single type can render in multiple visual styles.
- **Not a Root Signal category.** Root Signal has its own taxonomy (tensions, needs, aid, notices, gatherings). The glue layer maps these to post types, but the mapping isn't 1:1.

---

## The 7 Post Types

### 1. Story
**What it is:** Long-form narrative content. Community voices, feature articles, investigative pieces, profiles-in-depth, explainers.

**CMS editing experience:** Rich text editor as the primary input. Multi-paragraph body with formatting (bold, italic, links, block quotes). Optional image, caption, pull quotes, drop cap, multi-column layout. Kicker field for topic labeling. This is the "writing" experience — the editor will read and shape prose.

**Layout weight default:** `heavy` — typically takes a full column or feature zone.

**Compatible post templates:** feature, gazette, digest

**Editor's mental model:** "I need to read this and decide if the writing is ready."

**Could originate from:** A Root Signal Situation (auto-drafted narrative around a tension), an editor's original reporting, an LLM draft sourced from multiple signals, a community-submitted piece.

**Example content from current codebase:**
- `storyFire` — house fire recovery narrative (kicker: "Community Voices")
- `digestHomeless` — housing situation explainer (kicker: "Housing")
- `digestRefugee` — resettlement story (kicker: "Resettlement")
- `digestClimate` — environment piece (kicker: "Environment")

---

### 2. Alert
**What it is:** Elevated-severity notice demanding reader attention. Public safety advisories, community alerts, emergency information, critical service disruptions. The key distinction from Update: an Alert implies the reader may need to act or be aware for their safety/wellbeing.

**CMS editing experience:** Short body (a few sentences, not paragraphs). Source/attribution field (who issued this — city, county, utility, community org). Severity indicator. Optional expiration date. This is a "verify and place" experience — the editor checks the source and decides prominence, not prose quality.

**Layout weight default:** `light` in terms of physical space, but `high priority` in terms of placement — compact content that goes near the top of the broadsheet.

**Compatible post templates:** feature-reversed (dark/high-contrast), gazette, bulletin, ticker

**Editor's mental model:** "Is this real? How high do I place it?"

**Could originate from:** A Root Signal Notice with high severity, an official government advisory, a community safety report, editorial judgment.

**Example content from current codebase:**
- `urgentShelter` — shelter capacity crisis (tag: "Urgent")
- `urgentHousing` — family displacement notice (tag: "Housing Needed")
- `alertWater` — boil water advisory (tag: "Water Advisory")
- `alertICE` — know-your-rights community alert (tag: "Community Alert")

**Design note:** The current codebase treats "urgent" as a standalone type with its own renderer (dark ink background, red accent, reversed-out block). In this new system, Alert is the type; urgency level is handled by severity or the `urgent` tag.

---

### 3. Update
**What it is:** Brief, timely, neutral informational item. News briefs, status changes, deadline reminders. Low editorial overhead — these often run as-is or with a quick trim.

**CMS editing experience:** The lightest form. Title + short body + timestamp. No special field groups needed by default. This is "title, sentence or two, done."

**Layout weight default:** `light` — stacks in ticker rows, digest lists, sidebar briefs.

**Compatible post templates:** gazette, ledger, ticker, digest

**Editor's mental model:** "Is this still current? Does it need a trim?"

**Could originate from:** Root Signal Notices (low-to-medium severity), automated deadline tracking, official announcements, LLM-summarized news.

**Example content from current codebase:**
- `updateWarming` — warming center hours extended ("5 hours ago")
- `updateMNsure` — enrollment deadline reminder ("Today")
- `updateTax` — free tax prep availability ("Yesterday")
- `updateSNAP` — benefits update ("3 days ago")
- Dateline variants: `dateSNAP`, `dateSchool`, `dateTransit` with absolute dates

---

### 4. Exchange
**What it is:** A community need or offer — someone/something needs resources, or someone/something has resources to give. This type covers both directions because the CMS form is structurally identical; the direction (need vs. offer) is indicated by a tag.

**CMS editing experience:** Structured fields as the primary input (not prose). Contact information, list of items needed/available, status/availability, optional location and hours/schedule. The editor verifies details and checks status rather than editing prose.

**Layout weight default:** `medium` — fits in classifieds columns, community sections, card grids.

**Compatible post templates:** gazette, ledger, bulletin, ticker

**Editor's mental model:** "Is this verified? Is it still active/available?"

**Direction is a tag, not a type distinction:**
- Tag `need` → "Volunteers Needed", "Donations Needed", "Housing Needed"
- Tag `aid` → "Available", "Offering", "Free"
- The layout engine can query by tag to group needs separately from aid if desired

**Could originate from:** Root Signal Needs or Aid signals, community submissions, organizational postings, GoFundMe/mutual aid scraping.

**Example content from current codebase (needs):**
- `reqFoodShelf` — food shelf volunteers needed (tag: "Volunteers Needed")
- `reqCoats` — winter coat donations needed (tag: "Donations Needed")
- `reqESL` — ESL tutors needed (tag: "Volunteers Needed")
- `reqInterp` — interpreters needed (tag: "Volunteers Needed")
- `reqDrivers` — drivers for appointments needed (tag: "Volunteers Needed")

**Example content from current codebase (aid):**
- `offerRoom` — room available (tag: "Offer . Housing", status: "Available now")
- `offerFurniture` — furniture available (tag: "Offer . Donation", status: "Available now")
- `offerLegal` — free legal help (tag: "Offer . Volunteer", status: "By referral")
- `offerChildcare` — childcare available (tag: "Offer . Childcare", status: "Spots available")
- `offerTransport` — rides to appointments (tag: "Offer . Transport", status: "Call 651-602-1111")

**Design decision — why collapse Need + Aid into Exchange:**
The CMS form is identical for both. Same fields: contact, items, status, schedule. The editorial workflow is the same: verify details, check status, decide placement. Keeping them as one type with a direction tag means fewer types to maintain, and the layout engine can still separate them by querying tags. If a page of "just needs" or "just offers" is wanted, filter by tag.

---

### 5. Event
**What it is:** Something happening at a specific time and place. Community gatherings, workshops, meetings, drives, fairs. Calendar-shaped content.

**CMS editing experience:** Date/time picker is the prominent UI element. Location/address field. Description body. Recurring toggle (weekly ESL class vs. one-time food drive). Optional cost, contact, registration link.

**Layout weight default:** `medium` — fits in calendar sidebars, event rows, card grids.

**Compatible post templates:** gazette, ledger, bulletin, ticker, feature

**Editor's mental model:** "Is this still upcoming? Worth featuring?"

**Could originate from:** Root Signal Gatherings, community calendar submissions, organizational event postings, scraped event listings.

**Example content from current codebase:**
- `eventFoodDrive` — Saturday food drive (month/day/dow + "Sat 10am-2pm . Free")
- `eventRights` — Know Your Rights workshop (month/day/dow + "Tue 6-8pm . Free")
- `eventKaren` — Karen New Year celebration
- `eventTenant` — Tenant rights meeting
- `eventHealth` — Community health screening
- Recurring/agenda variants: `agendaLiteracy` ("Mon 6-7:30pm"), `agendaCitizenship` ("Wed 10am-12pm"), `agendaSobriety` ("Daily 7-8am")

**Design note:** The `circleLabel` field ("Today!", "Tomorrow") in the current codebase is a rendering concern, not a data concern. The renderer derives this from the event's date relative to the current date.

---

### 6. Reference
**What it is:** Evergreen, structured information. Directories, resource lists, guides, how-tos. Low churn — updated periodically, not daily. The "clip and save" content.

**CMS editing experience:** Structured item list as the primary input (name + detail per row, potentially multi-column). Contact, location, hours/schedule fields. "Last updated" / freshness date. The editor's job is verification: "are these phone numbers still correct? Are these hours current?"

**Layout weight default:** `medium` — fits in resource sections, sidebars, reference blocks.

**Compatible post templates:** gazette, ledger, bulletin

**Editor's mental model:** "Is this still accurate? When was it last verified?"

**Could originate from:** Curated directories, organizational resource pages, LLM-compiled reference guides, editor-maintained lists.

**Example content from current codebase:**
- `resHennepin` — Hennepin County food shelves (structured items list)
- `resRamsey` — Ramsey County food shelves
- `resTaxSites` — 12 free tax prep sites (count badge: "12")
- `resClinics` — 5 free health clinics (count badge: "5")

---

### 7. Spotlight
**What it is:** A feature profile of a person, business, or organization. "Neighbor to Know," business listings, organizational profiles. Human interest content that's more structured than a Story but more personal than a Reference.

**CMS editing experience:** Person-specific fields (name, role, bio, photo, quote) OR business-specific fields (tagline, hours, location, contact). Media/image upload. The editing experience is "curating a profile" — assembling structured info about a subject rather than writing prose or verifying a list.

**Layout weight default:** `medium` to `heavy` depending on whether it's a sidebar card or a feature profile.

**Compatible post templates:** feature, gazette, bulletin

**Editor's mental model:** "Is this the right week to feature this?"

**Could originate from:** Editor-created profiles, community nominations, business directory entries, LLM-drafted profiles from Root Signal data.

**Covers both people and places:**
- Person spotlight: `profileLiNguyen` ("Neighbor to Know"), `profileMarcus` ("Community Voice")
- Business spotlight: `localPimento` (restaurant), `localHmong` (shopping center), `localMidtown` (market), `localSomali` (mall)

**Design decision — why merge Profile + Local into Spotlight:**
The CMS form is similar: both involve assembling structured info about a subject (name/tagline, description, photo, location, contact, hours). The editorial intent is the same: "let's feature this person/place this week." A `person` tag or `business` tag distinguishes them for filtering. The renderer adapts based on which field groups are populated — if `person` fields are filled, render as a profile; if `hours` and `tagline` are filled, render as a business listing.

---

## Post Data Structure

### Universal Fields (every post has these)

```
Post {
  id:          string         // unique identifier
  type:        PostType       // one of 7 values (see above)
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
- **light** — fits a compact/ticker slot. Updates, alerts, brief exchanges, short items.

The type sets the default weight, but the editor (or glue layer) can override. A short story could be `medium`. A critical alert could be `heavy`. A detailed exchange with many items could be `heavy`.

### The `priority` field

Priority determines ordering. Higher priority = closer to the top of the broadsheet. The glue layer sets initial priority based on Root Signal scoring (impact, recency, severity). The editor overrides by reordering rows or manually adjusting priority.

Priority is a single number, not a multi-factor score. The glue layer is responsible for collapsing Root Signal's multidimensional scoring into a single priority value. The layout engine doesn't need to understand why something is high-priority — just that it is.

---

## Field Groups (Optional Blocks)

Every field group is available on every post type. The type determines which groups are **open by default** in the CMS form, but the editor can always expand or collapse any group. Nothing is locked out — a Story could have Hours, an Update could have Contact.

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
**Default open on:** Exchange (aid direction), Reference, Spotlight (business)

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
  state:     "available" | "needed" | "closed"
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

**Design note:** Call to Action (CTA) was considered as its own type but collapsed. A CTA is typically a Story, Update, or Alert with a `link` field group toggled open. If the editorial team finds they're creating CTAs frequently enough that it warrants a dedicated form preset, this could be revisited. See [Open Questions](#open-questions).

### source
```
{
  name:         string    // "City of Minneapolis", "Hennepin County", "Community Report"
  attribution:  string    // additional source context
}
```
**Default open on:** Alert

### meta
```
{
  kicker:     string    // topic label displayed above title ("Community Voices", "Housing")
  byline:     string    // author attribution
  timestamp:  datetime  // when published or last updated
  updated:    string    // freshness label ("Updated weekly", "Updated Feb 2026")
}
```
**Default open on:** Story (kicker, byline), Update (timestamp), Reference (updated)

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
| `urgent` | Red accent treatment, elevated placement priority |
| `recurring` | Schedule display instead of one-off date, "ongoing" indicator |
| `closed` | Greyed-out treatment, "fulfilled" / "expired" badge |
| `need` | Direction indicator on Exchange type — "something is needed" |
| `aid` | Direction indicator on Exchange type — "something is available" |

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

"three-column-equal" {
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

### Post Templates

A post template defines the visual treatment of a single post within a slot. The current codebase calls these "families" and "variants." Examples from the design playground:

- **feature** — premium editorial treatment, large typography, dramatic layout
- **gazette** — top-border tabbed frame, colored accent, standard card
- **ledger** — left-border tabbed, classifieds feel, compact listing
- **bulletin** — boxed card, community board feel
- **ticker** — ultra-compact single-line treatment
- **digest** — headline-only, no body text

Each post template declares which types it can render:

```
feature:    [story, alert, event, spotlight]
gazette:    [story, alert, update, exchange, event, reference, spotlight]
ledger:     [update, exchange, event, reference]
bulletin:   [alert, exchange, event, reference, spotlight]
ticker:     [alert, update, exchange, event]
digest:     [story, update, exchange]
```

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
        { post: post_012, post_template: "ticker" },
      ]
    },
    {
      row_template: "three-column-equal",
      slots: [
        { post: post_345, post_template: "gazette" },
        { post: post_678, post_template: "gazette" },
        { post: post_901, post_template: "bulletin" },
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
| **Tension** | Story | (topic-derived) | A tension is the "why" behind everything. Becomes the seed for a narrative, possibly auto-drafted by LLM. May also surface as an Alert if severe enough. |
| **Need** | Exchange | `need` + topic tags | Direct mapping. Items/contact/status populated from signal data. |
| **Aid** | Exchange | `aid` + topic tags | Direct mapping. Items/contact/status populated from signal data. |
| **Notice** | Update or Alert | topic tags, possibly `urgent` | Low-severity notices → Update. High-severity notices → Alert. The glue layer decides the threshold. |
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
- Editor-authored Alerts about editorial matters

These are created directly in the CMS with no Root Signal origin. The type system treats them identically.

---

## Open Questions

### Q1: Is `weight` sufficient, or do we need `size` as a separate dimension?

**The concern:** Weight currently combines two concepts:
- **Editorial importance** for placement (handled by `priority`)
- **Physical space needed** for rendering

A dense Reference with 12 items in two columns is `medium` weight, but it's visually much taller than a 3-line Exchange card that's also `medium`. Can the post template handle variable content lengths gracefully, or does the layout engine need to know the physical size?

**Possible resolution:** The post template is responsible for handling variable content (truncation, "show more", scrolling). The layout engine doesn't care about pixel height — it cares about column width, which `weight` handles. If a medium-weight Reference has 12 items, the gazette template truncates to 4-5 with a "see all" link. The layout engine never needs to know.

**Status:** Leaning toward weight-only, with templates handling overflow.

### Q2: Should Call to Action (CTA / Action) be its own type?

**The case for collapsing it:** A CTA is often just a Story, Update, or Alert with a link/button. The `link` field group (label, URL, deadline) can be toggled open on any type. The editing experience isn't fundamentally different.

**The case for keeping it:** The current codebase has a dedicated CTA type with distinct visual treatment (teal accent, bold bottom button). The editorial intent is different — "I want the reader to DO something specific." The detail page mockup (`mockup-cta.html`) has unique sidebar components: legislative hotline, talking points, audio player.

**The case for it being a tag:** If `action` is a reserved tag that triggers CTA-style rendering (prominent link button, deadline display), any type could become action-oriented. A Story with an `action` tag gets a CTA button. An Alert with an `action` tag gets a "take action" treatment.

**Status:** Currently collapsed — CTA is achieved via the `link` field group + an `action` tag on any type. Revisit if editors find themselves wanting a dedicated CTA form preset.

### Q3: Does `weight` need more than 3 values?

Current values: `heavy`, `medium`, `light`

**The concern:** With only 3 values, the matching might be too coarse. A large Story with multiple images (definitely `heavy`) and a medium Story with one image (maybe `heavy`? maybe `medium`?) might need different slot sizes.

**Possible resolution:** 3 values is probably fine for the layout engine's purposes. The row template defines the column widths; the post template handles content within that width. If we find edge cases, we could add `x-light` (for ticker-only items) or `x-heavy` (for full-width features), but start with 3.

**Status:** Start with 3, add more only if real layout problems emerge.

### Q4: How do recurring events work?

A one-off food drive (Saturday, March 8) and a weekly ESL class (every Monday, 6-7:30pm) are both Events, but they render differently and have different editorial lifecycles.

**Current approach:** The `recurring` tag triggers schedule display instead of a one-off date. The `datetime` field group has a `recurring: boolean` flag. When recurring is true, the renderer shows the schedule pattern ("Mon 6-7:30pm") instead of a specific date.

**Open sub-question:** Does a recurring event expire? If the ESL class runs indefinitely, does it stay in the broadsheet every week? Or does it age out unless re-surfaced? This is probably an editorial workflow question, not a data model question.

**Status:** Handled by `recurring` tag + `datetime.recurring` flag. Editorial lifecycle TBD.

### Q5: How does the glue layer decide Alert vs. Update?

Both come from Root Signal Notices. The glue layer needs a threshold for "this is severe enough to be an Alert rather than an Update."

**Possible approaches:**
- Root Signal impact score above X → Alert, below X → Update
- Certain Root Signal tension categories (public safety, emergency services) → always Alert
- LLM classification: "does this require reader action or awareness for safety?" → Alert
- Default to Update, escalate to Alert only with editorial review

**Status:** Glue layer design question. The post type system supports both; the glue layer decides which to use.

### Q6: Should `source` be a first-class field or part of `meta`?

Currently `source` is its own field group (name, attribution). But it's really metadata about the post — who originated this information. It could live inside `meta` alongside `kicker`, `byline`, `timestamp`.

**The case for separate:** Alerts especially benefit from prominent source display ("City of Minneapolis", "Hennepin County Health"). Having it as its own field group means the CMS form can show it prominently for Alert types.

**The case for merging into meta:** It's metadata. Keeping the field group count lower reduces complexity.

**Status:** Leaning toward keeping it separate for now, since Alert editing benefits from prominent source display.

### Q7: How granular should row template slot definitions be?

Current slot definition: `{ weight: "medium", count: 3 }`

**Could also include:**
- Type restrictions: `{ weight: "medium", accepts: ["exchange", "event"], count: 3 }` — a "community needs" row that only accepts exchanges
- Tag restrictions: `{ weight: "medium", requires_tag: "urgent", count: 2 }` — an "urgent items" row
- Template restrictions: `{ weight: "medium", template: "ledger", count: 4 }` — a classifieds row that always uses ledger treatment

**The tradeoff:** More granular slots = more predictable layout, but less flexibility and more configuration to maintain. Less granular slots = more flexible, but the layout engine might produce unexpected combinations.

**Status:** Start with weight-only slots. If the auto-generated broadsheets are too chaotic, add type/tag restrictions to slots.

---

## Decision Log

Decisions made during the design conversation, with rationale.

| Decision | Rationale |
|----------|-----------|
| **7 types, not 12** | Collapsed based on structural similarity of CMS forms and layout behavior. Fewer types = simpler system. |
| **Tags instead of subtypes** | Request/Offer subcategories (volunteer, donation, housing, childcare, transport) are free-form tags, not enumerated subtypes. Avoids type explosion as new needs emerge. |
| **Need + Aid → Exchange** | Same CMS form, same fields, same editorial workflow. Direction indicated by `need`/`aid` tag. Layout engine can filter by tag if separate zones are wanted. |
| **Profile + Local → Spotlight** | Similar CMS form (structured info about a subject). `person`/`business` tag distinguishes them. Renderer adapts based on which field groups are populated. |
| **Urgent is a tag, not a type** | Can apply to any type (urgent Exchange, urgent Alert, urgent Event). Alert type handles the "elevated notice" case; `urgent` tag adds visual treatment to anything. |
| **Alert is a type, not just urgent Update** | Different CMS form (source/severity/expiration fields). Different editorial intent ("is this real? how high?") vs. Update ("is this current? does it need a trim?"). Different layout behavior (compact but high-placement). |
| **CTA collapsed into link field group + action tag** | The editing experience isn't different enough from other types to warrant its own type. Any type + link field group + action tag = CTA behavior. May revisit. |
| **Field groups are available on all types** | Type sets defaults-open, but nothing is locked out. A Story can have Hours. An Update can have Contact. Flexibility over rigidity. |
| **Weight is a default from type, editor-overridable** | The layout engine needs weight for slot-matching, but the editor/glue layer can override for specific posts that don't fit the type's default. |
| **Layout engine is dumb by design** | Intelligence lives in Root Signal (analysis) and the glue layer (mapping/prioritization). The layout engine just sorts and slots. Simple to build, test, and debug. |
| **Tags are free-form, not a closed set** | New topics, geographies, and community needs emerge organically. A small set of reserved tags (urgent, recurring, closed, need, aid) trigger special behavior; everything else is pure filtering/discovery. |
| **This repo is a design playground only** | Decisions here inform but don't constrain the main repos. The type system is architectural guidance, not production schema. |

---

## Appendix: Current Codebase Audit

What exists in the design playground (`mntogether-temp`) and how it maps to this proposed system.

### Current 9 Renderer Types (via `makeFamily()` in `components.js`)

| Current Type | Proposed Mapping |
|-------------|-----------------|
| `story` | → **Story** |
| `urgent` | → **Alert** (with `urgent` tag for severity) |
| `update` | → **Update** |
| `event` | → **Event** |
| `request` | → **Exchange** + `need` tag |
| `offer` | → **Exchange** + `aid` tag |
| `resource` | → **Reference** |
| `cta` | → Any type + `link` field group + `action` tag |
| `local` | → **Spotlight** + `business` tag |

### Current Feature-Only Types (not in `makeFamily()`)

| Current Type | Proposed Mapping |
|-------------|-----------------|
| `hero` | → **Story** rendered with `feature` post template |
| `profile` | → **Spotlight** + `person` tag |
| `editorial` | → **Story** rendered with `feature` post template (no kicker) |
| `photo` | → **Story** or **Update** with `media` field group (photo + caption) |

### Current Widget Types (non-post content)

| Widget | Status in New System |
|--------|---------------------|
| Pull Quote | Inline element within a Story body, not a post type |
| Stat Card | Widget, not a post type. Could be a layout element within a row template. |
| Number Block | Widget, not a post type. Same as stat card. |
| Weather Forecast | Its own system — runs once daily per county/city. Not a post type. |
| Resource Bar (hotlines) | Persistent UI element, not a post type. Part of the broadsheet frame. |
| Section Separator | Row template concern, not a post type. |

### Rendering Families in Current Codebase

| Family | Description | Maps To |
|--------|-------------|---------|
| Gazette (`gaz-`) | Top-border tabbed frame, colored accent | Post template: `gazette` |
| Ledger (`led-`) | Left-border tabbed, classifieds feel | Post template: `ledger` |
| Bulletin (`bul-`) | Boxed card, community board | Post template: `bulletin` |
| Ticker (`tick-`) | Ultra-compact single-line | Post template: `ticker` |
| Feature (`feat-`) | Premium editorial, dramatic typography | Post template: `feature` |
| Digest (`dig-`) | Compact headline-only | Post template: `digest` |
| Broadsheet | Freeform mid-weight editorial | Various post templates |

### Detail Page Mockups

| Mockup Page | Proposed Type | Key Sidebar Components |
|------------|---------------|----------------------|
| `mockup-story.html` | Story | Related articles |
| `mockup-event.html` | Event | Location, When, Related |
| `mockup-request.html` | Exchange + `need` | Hours, Contact, Location, Items Needed, Related |
| `mockup-offer.html` | Exchange + `aid` | Status, Hours, Contact, Location, Available Items, Related |
| `mockup-resource.html` | Reference | Hours (4 widget variants), Location, Contact, Resource Links, Related |
| `mockup-cta.html` | Story/Alert + `link` + `action` tag | Legislative Hotline, Resources, Audio, More Actions |
| `mockup-local.html` | Spotlight + `business` | Hours, Phone, Location, Links, Related |
