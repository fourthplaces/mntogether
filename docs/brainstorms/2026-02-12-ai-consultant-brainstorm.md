---
date: 2026-02-12
topic: ai-consultant
---

# AI Consultant: Replacing Pipeline with Agentic Synthesis

## What We're Building

Replace the current rigid pipeline (3-pass extraction → LLM sync → scoring → notes) with an **AI consultant** that runs per-org using a **map-reduce architecture**:

1. **Page Briefs** (map) — Each crawled page is compressed into a brief of critical information. Not post-shaped — just structured knowledge about what the page tells us about the org.
2. **Org Document** (compile) — All page briefs from all sources + existing posts/notes compiled into a single document.
3. **Consultant** (reduce) — An LLM synthesizes the full org document and proposes actions: create posts, update existing ones, add notes, flag contradictions, merge duplicates, archive stale content.

All proposed actions go through a **unified review surface** where admins can comment, triggering AI-driven revisions that admins can approve or edit directly.

## Why This Approach

- **Cross-source reasoning** — Consultant sees all sources at once, catches contradictions (e.g., website says "accepting donations" but social media says "not accepting donations at this time")
- **Handles scale** — Page briefs compress 150k+ pages to ~1-2k of critical info
- **Single review surface** — All actions in one place with conversational refinement, replacing scattered sync proposals + notes + enrichment
- **Clean separation** — Page briefs are pure information extraction (dumb), consultant is where the intelligence lives
- **Flexible** — Consultant decides what shape information takes (post, note, update) rather than the pipeline pre-deciding

## Architecture

```
Crawled Pages (150k+ tokens each)
    ↓ map (parallel, per-page)
Page Briefs (~1-2k each)
    ↓ compile (deterministic, prioritized)
Org Document (all briefs + existing posts + notes)
    ↓ reduce (single LLM call, the "consultant")
Proposed Actions (create, update, note, merge, archive)
    ↓
Review Surface (approve / comment → AI revises / edit directly)
```

## Key Decisions

### Page Brief Schema
Each page brief extracts:
- Critical information (what this org does, programs, services)
- Locations (addresses, service areas)
- Urgent calls to action (accepting donations, need volunteers, emergency services)
- Summaries (concise overview of page content)

### Org Document Compilation Priority
1. **Website information first** — Stable foundation (services, hours, programs)
2. **Most recent social media posts** — Timely signal layer (closures, urgent needs, event updates)
3. **Existing posts + notes** — Current state in the system

### Consultant Behavior
- Acts like a social media consultant for the organization
- Sees the full picture, reasons holistically
- Proposes a batch of actions (not just one)
- Actions: create post, update post, add note, merge duplicates, archive stale content, flag contradictions

### Review + Refinement Loop
1. Consultant proposes actions in a batch
2. Admin reviews proposals in unified surface
3. Admin can **comment** on any proposal (e.g., "hours are wrong")
4. AI interprets comment → proposes revised version
5. Admin can **approve as-is** or **edit directly** before approving

### Triggers
- **After crawl** — New content arrives for an org → consultant runs automatically
- **Scheduled** — Periodic sweep to catch staleness and drift
- **On-demand** — Admin clicks "Run consultant for Org X"

### Scope
- **One org at a time** — No cross-org reasoning (existing dedup workflow handles that)
- Optional `ReadFullPage(url)` tool as fallback if consultant needs raw detail

### What This Replaces vs Keeps
**Replaces:** 3-pass extraction pipeline, LLM sync, relevance scoring, notes generation
**Keeps:** Crawling/page extraction (Firecrawl), embeddings, org/source management, dedup workflow

## Open Questions for Planning

- How is the org document structured? (Sections per source? Single narrative?)
- Proposal model schema evolution — how do comments + revisions extend current sync_proposals?
- Consultant prompt design — persona, quality standards, decision framework
- Migration path — run both systems in parallel during transition?
- How does the scheduled trigger determine which orgs need attention?

## Next Steps

→ `/workflows:plan` for implementation details
