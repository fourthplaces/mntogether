# Deduplication Schema Hint

**Source:** `packages/server/src/domains/posts/activities/deduplication.rs` — `DEDUP_SCHEMA`

**Type:** Schema hint used by both `DEDUP_SYSTEM_PROMPT` and `DEDUP_PENDING_SYSTEM_PROMPT`

## Schema

```
Return JSON in this format:
{
  "duplicate_groups": [
    {
      "canonical_id": "uuid - the post to keep",
      "duplicate_ids": ["uuid1", "uuid2", "... - posts to merge into canonical"],
      "merged_title": "string or null - improved title if merge improves it",
      "merged_description": "string or null - improved description if merge adds value",
      "reasoning": "string - why these are duplicates (same service + same audience)"
    }
  ],
  "unique_post_ids": ["uuid1", "uuid2", "... - posts with no duplicates"]
}
```
