# Seed Data

Development seed data lives in three JSON files. A Node script converts them to SQL at load time.

## Quick Start

```bash
# Seed an existing (migrated) database
make seed

# Full reset: drop DB, run migrations, seed
make reset-db

# Or from the dev dashboard
./dev.sh     # then press [d]
```

## Files

| File | Records | Purpose |
|------|---------|---------|
| `organizations.json` | 50 orgs | Service providers across 5 metro counties |
| `posts.json` | 107 posts | Stories, notices, exchanges, events, spotlights, references |
| `tags.json` | 21 topics, 11 areas, 3 safety | Tag display config (colors, display names) |
| `seed.mjs` | -- | Reads the 3 JSON files, outputs SQL to stdout |

## How It Works

`seed.mjs` reads the JSON files and generates a single SQL transaction with:

1. Tag INSERTs (topic colors, service areas, safety, reserved guards)
2. Organization INSERTs
3. Post CTEs — each post gets its own CTE chain that inserts into `posts` + field group tables (`post_meta`, `post_source_attribution`, `post_person`, `schedules`, `post_items`, `post_link`, `post_media`) and wires up `taggables`

Everything uses `ON CONFLICT DO NOTHING` so the script is idempotent.

## Editing Seed Data

Edit the JSON files directly. The schema for each post in `posts.json`:

```json
{
  "title": "...",
  "description": "...",
  "summary": "...",
  "postType": "story|notice|exchange|event|spotlight|reference",
  "category": "community",
  "weight": "heavy|medium|light",
  "priority": 50,
  "location": "City, MN",
  "meta": { "kicker": "...", "timestamp": "2026-02-20T00:00:00Z" },
  "source": "Organization Name",
  "tags": {
    "topic": ["food", "housing"],
    "serviceArea": ["hennepin-county"],
    "reserved": ["need"],
    "safety": ["no_id_required"]
  },
  "person": { "name": "...", "role": "...", "bio": "...", "photoUrl": "...", "quote": "..." },
  "schedule": [{ "dtstart": "...", "dtend": "...", "rrule": "...", "isAllDay": false }],
  "items": [{ "name": "...", "detail": "...", "sortOrder": 0 }],
  "links": [{ "url": "...", "label": "...", "deadline": "2026-03-15" }],
  "media": [{ "imageUrl": "...", "caption": "...", "credit": "...", "sortOrder": 0 }]
}
```

Optional fields (`person`, `schedule`, `items`, `links`, `media`, `meta`) can be omitted.
