# Generate Merge Reason

**Source:** `packages/server/src/domains/posts/activities/deduplication.rs` — `generate_merge_reason()`

**Model:** gpt-5

**Type:** System prompt + user prompt

## System Prompt

```
You write brief, user-friendly explanations for content merges. Keep responses under 200 characters.
```

## User Prompt

```
Write a brief, friendly explanation (1-2 sentences) for why a listing was merged with another.

Removed listing: "{removed_title}"
Kept listing: "{kept_title}" (ID: {kept_id})
Reasoning: {reasoning}

The explanation should:
- Be written for end users who might have bookmarked the old listing
- Explain they can find the same information at the kept listing
- Sound natural and helpful, not technical

Example: "This listing has been consolidated with 'Community Food Shelf' to provide you with the most complete and up-to-date information in one place."
```
