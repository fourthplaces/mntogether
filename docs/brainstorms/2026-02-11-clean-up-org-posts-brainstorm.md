---
date: 2026-02-11
topic: clean-up-org-posts
---

# Clean Up Org Posts

## The Problem

Duplicate published posts appear in the admin org view, especially from social media sources. The same resource gets extracted multiple times across runs, or the same content appears on both a website and Instagram with different wording. The existing deduplication workflow only catches duplicates at the **pending** stage — once posts are published (active), nothing cleans them up.

This is fundamentally a **safety net for LLM imperfection**. The extraction and sync pipeline uses GPT-5 Mini, which doesn't always catch semantic duplicates. A separate cleanup pass with GPT-5 full gives better accuracy on the cases that slipped through.

## What We're Building

A new Restate workflow — "Clean Up Org Posts" — triggered manually from the admin org page. It runs three phases:

### Phase 1: Deduplicate Pending Posts
- **Pending-to-pending**: Find duplicates among draft posts across all org sources, stage MERGE proposals
- **Pending-to-active**: Find pending posts that match already-published posts, stage UPDATE/DELETE proposals
- Most impactful phase — cleans the review queue before an admin works through it

### Phase 2: Deduplicate Active Posts
- **Active-to-active**: Find duplicates among published posts across all org sources, stage MERGE proposals
- This is the current gap — no existing mechanism handles this

### Phase 3: Purge Rejected Posts
- Automatically soft-delete all rejected posts for the org
- No proposals needed — admin already said no to these

## Why This Approach

- **Org-scoped**: One pass covers all sources, catches both same-source and cross-source duplicates
- **Separate from extraction pipeline**: Cleanup is a distinct admin action, doesn't risk breaking existing dedup
- **Proposals for dedup, auto for rejected**: Active/pending dupes need human review; rejected posts are safe to auto-delete
- **Manual trigger only**: Hit the button when the queue looks messy, no scheduling needed
- **GPT-5 full for detection**: Uses the bigger model since accuracy matters more than cost here — this is catching what Mini missed
- **GPT-5 Mini for merge reasons**: The short user-facing explanation doesn't need the big model

## Key Decisions

- **Org-scoped, not source-scoped**: Catches cross-source dupes (website + Instagram) in one pass
- **All duplicate resolutions go through proposals**: No automatic merging of active or pending posts
- **Rejected posts auto soft-delete**: Already rejected = safe to clean up without review
- **GPT-5 full for duplicate detection**: Different model than extraction to avoid reproducing the same blind spots
- **Standalone workflow**: Not bolted onto existing dedup or regeneration flows

## Open Questions

- Chunking strategy for large orgs with 200+ active posts — how to batch for the LLM
- Progress indicator in admin UI (like regenerate has)?
- Should cross-source priority rules apply? (website > facebook > instagram > tiktok > x for canonical selection)

## Next Steps

→ `/workflows:plan` for implementation details
