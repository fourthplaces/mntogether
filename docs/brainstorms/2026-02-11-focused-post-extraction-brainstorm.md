---
date: 2026-02-11
topic: focused-post-extraction
---

# Focused Post Extraction (v2)

## Context

MN Together is a platform connecting communities around the immigration crisis in Minnesota. The extraction pipeline scrapes org websites and uses LLM to extract actionable posts.

## The Problem

The previous extraction prompt was too broad. It extracted general community services (food shelf hours, worship services, generic programs) alongside immigration-crisis-specific events. False positives like "Attend Sunday Worship" and "Attend Bilingual Sunday Worship" were cluttering the feed.

## Key Insight

This isn't a model problem (GPT-5 vs GPT-5 Mini) — it's a prompt problem. The prompt told the LLM to extract broadly across "three pillars" including standing services. Tightening the prompt to immigration-crisis events is the fix.

## The Litmus Test (v2)

Two-part test, both must pass:
1. "Is this connected to immigrant communities and the current crisis?"
2. "Is this something someone can show up to, participate in, or contribute to?"

## What Qualifies

### Community Support Events
- Know-your-rights workshops, community meetings about immigration response
- Vigils, gatherings, solidarity events connected to immigration
- Rallies and marches connected to immigration
- ICE rapid response trainings, sanctuary events
- Legal clinics or legal aid events (not standing office hours)

### Volunteer Opportunities
- Grocery/supply delivery to families afraid to leave home
- Accompaniment, supply packing, rapid response teams
- Sanctuary hosting, translation at events

### Donation Drives
- Legal defense / bail fund fundraisers
- Supply drives for immigrant families
- Rent/housing emergency funds

## What to Exclude

- Regular worship services
- Standing services with regular hours (unless explicitly serving immigrant families in crisis)
- Staff job postings, board governance, "about us" pages
- Past event recaps, press releases, generic navigation
- General community programs not connected to immigrant communities
- Political events unrelated to immigration (environmental protests, general labor actions)

## Key Decisions

- **Immigration-focused, not broad crisis response**: The immigration focus IS the filter that keeps general community events out.
- **Community support framing, not "resistance"**: Many orgs avoid terms like "resistance" and "political action." This is about community support first.
- **Dropped "recipient" audience type**: With focus on events/drives, audiences are: participant, volunteer, donor.
- **Standing services carve-out**: Standing services excluded UNLESS explicitly tied to immigrant families in crisis.
- **GPT-5 Mini is fine**: The false positives were prompt-driven, not model-capability-driven.

## Step 2: Relevance Scoring

### Design

**Standalone scoring pass** — separate from extraction, reusable on both new and existing posts.

**Input:** post title + summary + description + org name

**Output per post:**
- `score` — composite 1-10 integer
- `breakdown` — per-factor scores and reasoning

**Composite score weighting:**

| Factor | Weight | What it measures |
|--------|--------|-----------------|
| Immigration relevance | 50% | Connected to immigrant communities / ICE / the crisis |
| Actionability | 30% | Specific event, drive, or action someone can take |
| Completeness | 20% | Has date, location, contact info |

**Breakdown format example:**
> Relevance: 9/10 — ICE rapid response training. Actionability: 4/10 — no specific date. Completeness: 3/10 — missing contact info.

### Thresholds

| Score | Label | Behavior |
|-------|-------|----------|
| 8-10 | High confidence | Quick scan / near-auto-approve |
| 5-7 | Review needed | Human reviews |
| 1-4 | Likely noise | Flagged as "probably noise, check if you want" |

### Two use cases

**New posts:** Extract (Pass 1) → Dedupe (Pass 2) → Investigate (Pass 3) → **Score (Pass 4)** → Surface in proposals UI with score

**Existing posts:** Batch score all posts in database → Flag low scorers (1-4) for human review. This cleans up posts that slipped through under the old broad prompt.

### Why separate from extraction

- Scorer is a different "judge" than the extractor — less likely to be generous with its own work
- Reusable on existing posts without re-extracting
- Scoring criteria can evolve independently of extraction prompt
- One extra LLM call per post (GPT-5 Mini, small input — cheap)

## Implementation

### Step 1 (done)
Updated `NARRATIVE_EXTRACTION_PROMPT` in:
`packages/server/src/domains/crawling/activities/post_extraction.rs`

### Step 2 (next)
- Build standalone scoring activity (`score_post` function)
- Integrate into extraction pipeline as Pass 4
- Add `relevance_score`, `relevance_breakdown` fields to post model
- Build batch scoring for existing posts
- Surface scores in admin proposals UI with filtering

## Next Steps

1. Deploy updated extraction prompt, monitor quality
2. Build scoring activity and integrate as Pass 4
3. Add scoring fields to post schema (migration)
4. Build batch scoring command for existing posts
5. Update admin proposals UI with score filtering
