# Sync Schema Hint

**Source:** `packages/server/src/domains/posts/activities/llm_sync.rs` — `SYNC_SCHEMA`

**Type:** Schema hint passed alongside the sync system prompt

## Schema

```
EXACT structure required (use lowercase operation names):

{
  "operations": [
    {"operation": "insert", "fresh_id": "fresh_1"},
    {"operation": "update", "fresh_id": "fresh_2", "existing_id": "550e8400-e29b-41d4-a716-446655440000"},
    {"operation": "delete", "existing_id": "550e8400-e29b-41d4-a716-446655440000", "reason": "No longer on website"},
    {"operation": "merge", "canonical_id": "550e8400-e29b-41d4-a716-446655440000", "duplicate_ids": ["6ba7b810-9dad-11d1-80b4-00c04fd430c8"], "merged_title": "Best combined title", "merged_description": "Combined description with details from all duplicates", "reason": "Duplicate entries for same service"}
  ],
  "summary": "1 insert, 1 merge"
}

CRITICAL RULES:
1. Use LOWERCASE operation names: "insert", "update", "delete", "merge"
2. For fresh_id: Use EXACT values from Fresh Posts (e.g., "fresh_1", "fresh_2") - do NOT invent IDs
3. For existing_id/canonical_id/duplicate_ids: Use EXACT UUIDs from Existing Posts (the "id" field) - NEVER use placeholders like "uuid-123" or made-up IDs
4. For MERGE: provide merged_title and merged_description with COMBINED content from all duplicates
```
