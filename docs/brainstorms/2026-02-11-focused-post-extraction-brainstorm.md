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
- **No scoring system (yet)**: The LLM's binary extract/skip decision with a clear prompt is sufficient for now. Scoring is a Step 2 enhancement.
- **GPT-5 Mini is fine**: The false positives were prompt-driven, not model-capability-driven.

## Step 2: Relevance Scoring (future)

Add `relevance_score` (1-10) and `relevance_reason` to extraction output. Surface in admin proposals UI so humans can filter: high-confidence posts auto-reviewed, low-confidence flagged as "likely noise, check these." This is a triage UX improvement, not needed until the prompt is proven.

## Implementation

Updated `NARRATIVE_EXTRACTION_PROMPT` in:
`packages/server/src/domains/crawling/activities/post_extraction.rs`

## Next Steps

1. Deploy updated prompt, monitor extraction quality
2. If borderline false positives persist, add relevance scoring to extraction schema
3. Build scoring filter into admin proposals UI
