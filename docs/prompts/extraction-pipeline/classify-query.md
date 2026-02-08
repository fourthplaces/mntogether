# Classify Query Intent

**Source:** `packages/extraction/src/pipeline/prompts.rs` — `CLASSIFY_QUERY_PROMPT`

**Model:** Varies by caller

**Type:** User prompt (contains `{query}` placeholder)

## Prompt

```
Classify the intent of this search query.

Query: {query}

Categories:
- COLLECTION: "Find all X" - looking for a list of items (volunteer opportunities, services, events)
- SINGULAR: "Find specific info" - looking for one piece of information (phone number, email, address)
- NARRATIVE: "Summarize/describe" - looking for an overview or description

Output JSON:
{
    "strategy": "COLLECTION" | "SINGULAR" | "NARRATIVE",
    "confidence": 0.0 to 1.0,
    "reasoning": "brief explanation"
}
```
