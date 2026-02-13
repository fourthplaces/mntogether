# Pending-Active Match Schema Hint

**Source:** `packages/server/src/domains/posts/activities/deduplication.rs` — `MATCH_SCHEMA`

**Type:** Schema hint used by `MATCH_PENDING_ACTIVE_SYSTEM_PROMPT`

## Schema

```
Return JSON in this format:
{
  "matches": [
    {
      "pending_id": "uuid of the draft post",
      "active_id": "uuid of the published post it duplicates",
      "reasoning": "why these describe the same service"
    }
  ],
  "unmatched_pending_ids": ["uuid1", "uuid2"]
}
```
