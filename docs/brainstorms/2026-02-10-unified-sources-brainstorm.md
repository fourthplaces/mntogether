---
date: 2026-02-10
topic: unified-sources
---

# Unified Sources: Replacing Websites + Social Profiles

## What We're Building

A unified `sources` table with class table inheritance (`website_sources`, `social_sources`) to replace the separate `websites` and `social_profiles` tables. The admin UI at `/admin/websites` becomes `/admin/sources` — a flat list showing all source types with a type column.

## Why This Approach

- Websites and social profiles share most of their DNA (scheduling, active state, org linkage, post generation, approval workflow)
- A unified table gives one model, one list query, one approval workflow, one set of CRUD operations
- Class table inheritance keeps type-specific fields normalized without nullable column sprawl
- The extraction library is domain-agnostic (works with URLs/pages via traits) — no impact from this change

## Schema Design

```
sources
├── id UUID PK
├── source_type TEXT NOT NULL ('website','instagram','facebook','tiktok')
├── url TEXT
├── organization_id UUID FK → organizations(id)
├── status TEXT DEFAULT 'pending_review'
├── active BOOLEAN DEFAULT true
├── scrape_frequency_hours INT DEFAULT 24
├── last_scraped_at TIMESTAMPTZ
├── submitted_by UUID FK → members(id)
├── submitter_type TEXT
├── reviewed_by UUID
├── reviewed_at TIMESTAMPTZ
├── rejection_reason TEXT
├── created_at TIMESTAMPTZ
├── updated_at TIMESTAMPTZ

website_sources
├── id UUID PK
├── source_id UUID FK → sources(id) UNIQUE, ON DELETE CASCADE
├── domain TEXT NOT NULL UNIQUE
├── max_crawl_depth INT
├── crawl_rate_limit_seconds INT
├── is_trusted BOOLEAN DEFAULT false

social_sources
├── id UUID PK
├── source_id UUID FK → sources(id) UNIQUE, ON DELETE CASCADE
├── source_type TEXT NOT NULL (denormalized for constraint)
├── handle TEXT NOT NULL
├── UNIQUE(source_type, handle)
```

## Key Decisions

- **Class table inheritance over single table**: Keeps type-specific fields normalized, avoids nullable sprawl
- **No `identifier` on parent table**: Use JOINs to get domain/handle for display — no data duplication
- **`source_type` denormalized on `social_sources`**: Enables DB-level UNIQUE(source_type, handle) constraint
- **Approval workflow on parent**: All sources go through the same pending_review → approved flow
- **No extraction library changes needed**: It's domain-agnostic, works with URLs via traits

## Migration Path

1. Create new tables (sources, website_sources, social_sources)
2. Migrate existing `websites` rows → sources + website_sources
3. Migrate existing `social_profiles` rows → sources + social_sources
4. Update FKs on posts and other referencing tables
5. Drop old tables

## UI Changes

- `/admin/websites` → `/admin/sources` (flat list with type column)
- Organization detail page: unified sources list instead of separate websites/social profiles sections

## Open Questions

- How to handle posts FK migration (currently references website_id and social_profile_id separately)
- Whether to keep backward-compatible views during transition
- Ordering/priority of server-side model + GraphQL changes vs UI changes

## Next Steps

→ `/workflows:plan` for implementation details
