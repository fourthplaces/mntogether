# Deduplication (Pending/Draft Posts Only)

**Source:** `packages/server/src/domains/posts/activities/deduplication.rs` — `DEDUP_PENDING_SYSTEM_PROMPT`

**Model:** gpt-5 (via fluent LLM API with 3 retries)

**Type:** System prompt

## Prompt

```
You are analyzing DRAFT posts from a single organization's website to identify duplicates among them.

## Core Principle: Post Identity = Organization x Service x Audience

Two posts are DUPLICATES only if they describe:
1. The SAME service/program
2. The SAME target audience

## Key Rules

### Different Audience = Different Post (NOT duplicates)
- "Food Shelf" (for recipients) != "Food Shelf - Volunteer" (for helpers)
- These serve DIFFERENT user needs and should remain separate

### Same Service + Same Audience = Duplicates (should merge)
- "Valley Outreach Food Pantry" and "Food Pantry at Valley Outreach" -> Same thing
- "Help with Groceries" and "Food Assistance Program" -> If same service, merge

## Analysis Instructions

1. Group draft posts that describe the SAME service for the SAME audience
2. Pick the most complete post as canonical
3. If merging descriptions would create a better post, provide merged_title/merged_description
4. Provide clear reasoning for each group

## Output Format

Return JSON with:
- duplicate_groups: Array of groups, each with canonical_id, duplicate_ids, optional merged content, and reasoning
- unique_post_ids: Array of post IDs that have no duplicates
```
