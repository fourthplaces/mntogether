# Root Signal API Specification

> ⚠️ **SUPERSEDED 2026-04-19.** This doc's framing (Root Signal as an enrichment-only
> service that updates existing posts) no longer matches the agreed model. Root Signal
> is the *producer* of posts; Root Editorial is the *consumer*. The authoritative spec
> lives at [`docs/architecture/ROOT_SIGNAL_DATA_CONTRACT.md`](./ROOT_SIGNAL_DATA_CONTRACT.md).
>
> This file is kept for historical reference only. Do not cite in new work.

> **Status:** Draft — defines what Root Editorial expects from Root Signal.
> **Date:** 2026-03-10

## Overview

Root Signal is an AI analysis service that processes posts for a county+week and produces:

1. **Topic classification** — groups posts into editorial topics
2. **Weight recommendation** — suggests heavy/medium/light for layout prominence
3. **Priority scoring** — ranks posts by editorial importance
4. **Weight-specific body text** — pre-written body copy at 3 length tiers
5. **Tags** — semantic tags for filtering and display

Root Signal does **not** choose post templates, row templates, or layout decisions. Those are the responsibility of Root Editorial (the CMS layout engine and human editors).

## Integration Flow

```
Root Signal runs → Updates posts in DB → Layout engine reads enriched posts
                                         → Groups by topic → Selects row recipes
                                         → Fills slots → 95% ready edition
                                         → Editor confirms/tweaks
```

### Timing

Root Signal runs **before** `batch_generate_editions`. The recommended sequence:

1. Cron triggers Root Signal for each county
2. Root Signal updates posts in the `posts` table
3. CMS calls `batch_generate_editions` for the week
4. Layout engine reads enriched posts and produces editions
5. Editors review and publish

## Request Format

Root Signal is invoked per county, per period:

```json
{
  "county_id": "uuid",
  "period_start": "2026-03-09",
  "period_end": "2026-03-15"
}
```

## Response Format

```json
{
  "topics": [
    {
      "slug": "housing",
      "title": "Housing & Shelter",
      "subtitle": "3 new resources this week",
      "priority": 90
    },
    {
      "slug": "food-access",
      "title": "Food Access",
      "subtitle": null,
      "priority": 75
    }
  ],

  "post_assignments": [
    {
      "post_id": "uuid",
      "topic_slug": "housing",
      "weight": "heavy",
      "priority": 85,
      "body_heavy": "Long body text suitable for feature templates. Should be roughly 400-500 characters, providing full context and detail about the resource, including who it serves, how to access it, and why it matters this week. This text is used when the post appears in a feature or feature-reversed template.",
      "body_medium": "Medium body text for gazette/bulletin templates. Roughly 150-200 characters covering the key facts and one actionable detail.",
      "body_light": "Short text for digest/ticker. Under 100 characters with the essential fact.",
      "tags": ["urgent", "housing", "shelter"]
    },
    {
      "post_id": "uuid",
      "topic_slug": "food-access",
      "weight": "medium",
      "priority": 60,
      "body_heavy": "Detailed description of the food pantry hours, eligibility, and what's available this week...",
      "body_medium": "Food pantry open Tue/Thu 9am-1pm. No ID required. Fresh produce available.",
      "body_light": "Food pantry Tue/Thu 9am-1pm, no ID needed",
      "tags": ["food", "pantry"]
    }
  ],

  "county_context": {
    "headline": "Spring flooding expected in southern counties",
    "alerts": [
      "NWS flood watch through Thursday",
      "County shelter activated at First Lutheran"
    ]
  }
}
```

## Field Specifications

### topics[]

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `slug` | string | yes | URL-safe identifier (e.g., `housing`, `food-access`). Used as `topic_slug` on edition sections. |
| `title` | string | yes | Human-readable section title (e.g., "Housing & Shelter"). |
| `subtitle` | string | no | Optional subtitle shown below the section title. |
| `priority` | integer | yes | 0-100. Higher priority topics appear earlier in the edition. |

Topics are **dynamic per week** — Root Signal defines them based on what posts exist for that county/period. There are no fixed topic buckets.

### post_assignments[]

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `post_id` | UUID | yes | The post being enriched. Must exist in the `posts` table. |
| `topic_slug` | string | no | Which topic this post belongs to. Must match a slug in `topics[]`. Posts without a topic remain ungrouped (above the fold). |
| `weight` | string | yes | One of `heavy`, `medium`, `light`. Determines layout prominence. |
| `priority` | integer | yes | 0-100. Higher = more prominent placement. |
| `body_heavy` | string | no | Body text for heavy-weight templates (~400-500 chars). |
| `body_medium` | string | no | Body text for medium-weight templates (~150-200 chars). |
| `body_light` | string | no | Body text for light-weight templates (~80-100 chars). |
| `tags` | string[] | yes | Semantic tags. Stored as tags with `kind='topic'` for the topic_slug and `kind='tag'` for others. |

### county_context

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `headline` | string | no | County-wide context headline for editors. |
| `alerts` | string[] | no | Active alerts relevant to the county (weather, emergencies, etc.). |

## What Root Signal Does NOT Provide

Root Signal is advisory — it enriches posts with metadata. It does not:

- **Choose post templates** — Root Editorial's layout engine matches posts to templates via row recipes and weight-based heuristics
- **Choose row templates** — The layout engine selects row recipes based on weight distribution
- **Order rows** — The layout engine orders by priority; editors can reorder
- **Create editions** — The CMS creates and manages edition lifecycle
- **Know about our component library** — It has no awareness of Feature, Gazette, Bulletin, etc.

## How Root Editorial Uses Root Signal Data

### Database Updates

When Root Signal returns, the CMS updates posts:

```sql
-- Update weight, priority, and body text
UPDATE posts SET
  weight = $2,
  priority = $3,
  body_heavy = $4,
  body_medium = $5,
  body_light = $6
WHERE id = $1;

-- Add topic tag (kind='topic')
INSERT INTO tags (kind, value) VALUES ('topic', $topic_slug)
  ON CONFLICT (kind, value) DO NOTHING;

INSERT INTO taggables (tag_id, taggable_type, taggable_id)
VALUES ((SELECT id FROM tags WHERE kind='topic' AND value=$topic_slug), 'post', $post_id)
  ON CONFLICT DO NOTHING;
```

### Layout Engine Behavior

1. **Load posts** — includes `topic_slug` from topic tags
2. **Group by topic** — posts with topics are clustered; ungrouped posts go above the fold
3. **Per-group placement** — select row recipes and fill slots for each topic group
4. **Section creation** — each topic group becomes an `edition_section` with the topic's title/subtitle
5. **Template assignment** — row recipe's `post_template_slug` is the default; engine falls back to weight-based matching if the recipe template isn't compatible

### Frontend Behavior

1. **Body text selection** — `preparePost()` selects `body_heavy`, `body_medium`, or `body_light` based on the assigned template's weight tier, falling back to `description` if no weight-specific body exists
2. **Section rendering** — rows are grouped by `section_id`; each section renders a `SectionSep` divider with the section's title and subtitle

## Body Text Guidelines

Root Signal should write body text following these principles:

- **Heavy** (~400-500 chars): Full context. Who, what, when, where, why. Written in a warm, community-newspaper tone. Can include multiple sentences and supporting details.
- **Medium** (~150-200 chars): Key facts and one actionable detail. Clear and direct. Think "community bulletin board" tone.
- **Light** (~80-100 chars): Essential fact only. One sentence or phrase. Think "ticker tape" or "at-a-glance."

All body text should:
- Use plain language (8th grade reading level)
- Be factual and actionable
- Avoid jargon
- Include specific details (dates, times, addresses) when relevant
- Not duplicate the post title

## Weight Assignment Guidelines

Root Signal should assign weights based on editorial importance:

- **Heavy**: Major stories, urgent notices, time-sensitive events, breaking community news. Expect 1-3 heavy posts per county per week.
- **Medium**: Standard community resources, ongoing programs, regular events. The majority of posts (60-70%) should be medium.
- **Light**: Quick updates, directory listings, reference items, ticker-worthy notices. Quick-scan content.

## Priority Scoring

Priority determines placement order within weight classes:

- **90-100**: Breaking/urgent — flooding, shelter openings, emergency resources
- **70-89**: High importance — new programs, major events, time-sensitive deadlines
- **50-69**: Standard — ongoing resources, regular events
- **30-49**: Lower priority — reference listings, evergreen content
- **0-29**: Filler — nice-to-have items if space permits

## Error Handling

If Root Signal fails for a county:
- The layout engine still works — it uses existing post data without enrichment
- Posts keep their current weight/priority from previous runs or manual assignment
- No body text tiers available — frontend falls back to `description` field
- No topic grouping — edition renders flat (no sections)

The system is designed to degrade gracefully. Root Signal enrichment is an enhancement, not a requirement.
