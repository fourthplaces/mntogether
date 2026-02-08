# Expand Query

**Source:** `packages/extraction/src/pipeline/prompts.rs` — `EXPAND_QUERY_PROMPT`

**Model:** Varies by caller

**Type:** User prompt (contains `{query}` placeholder)

## Prompt

```
Expand this search query with related terms to improve recall.

Query: {query}

Generate 5-10 related search terms that would help find relevant content.
Include:
- Synonyms
- Related concepts
- Common phrasings
- Industry jargon

Output JSON array of strings:
["term1", "term2", "term3", ...]
```
