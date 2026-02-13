---
date: 2026-02-10
topic: ai-note-generation-pipeline
---

# AI Note Generation Pipeline

## What We're Building

A 3-step pipeline that generates contextual notes from an organization's sources (websites, Instagram, etc.) and attaches them to the org's posts. This lets us surface information like "org says they're not accepting volunteers right now" on every relevant post listing.

## Pipeline

### Step 1: Generate (per source)

When a source is crawled/scraped, extract notes via LLM. One note per distinct piece of noteworthy information. This already exists for websites (`generate_notes_for_organization` in the notes extraction activity) and needs to expand to social sources (Instagram bios, posts, etc.).

### Step 2: Merge (per org)

Deduplicate by content similarity against existing notes for the org. If a semantically similar note already exists (e.g., "not accepting volunteers" found on both the homepage and Instagram), skip creating a duplicate. Keep the first note, don't bother tracking multi-source provenance — YAGNI.

### Step 3: Attach (org-wide)

Link the note to ALL active posts belonging to that organization. Notes are org-level context — if the org says "we're closed for the season," that's relevant to every one of their listings.

## When It Runs

- **On source crawl/scrape**: Generate + Merge + Attach. Notes stay fresh as content changes.
- **On post creation**: Attach only. New posts pick up existing org-level notes without re-running generation.

## Key Decisions

- **All sources considered**: The AI looks at websites, Instagram, Facebook — everything for the org. The value is catching cross-source contradictions ("Instagram says X but the listing says Y").
- **Simple deduplication over provenance tracking**: If two sources say the same thing, keep the first note. No `note_sources` join table or multi-source tracking.
- **Org-wide attachment over per-post matching**: Every note for an org applies to all its active posts. No embedding similarity or LLM-based matching needed for v1.
- **Severity levels preserved**: WARN (closures, capacity limits), NOTICE (schedule changes), INFO (announcements) — already in the model.

## What Already Exists

- `notes` table with severity, source tracking, is_public, expired_at
- `noteables` polymorphic join table (supports post, organization, website, social_profile)
- `generate_notes_for_organization` extraction activity (website content only)
- LLM prompt with severity classification
- Deduplication by exact source match (needs upgrade to semantic similarity)
- Restate service with full CRUD + `generate_notes` handler

## What's New

1. Expand generation to social sources (Instagram, etc.)
2. Semantic dedup in the merge step (not just exact source match)
3. Auto-attach to all org posts after generation
4. Attach existing org notes on post creation
5. Note expiration when source content changes (re-crawl finds the info is gone)

## Open Questions

- How to determine "semantic similarity" for dedup — embedding distance? LLM comparison? Simple text similarity?
- Should expired notes auto-detach from posts or just stop displaying?
- Batch size for attach step — attach to all org posts in one go, or paginate?

## Next Steps

-> `/workflows:plan` for implementation details
