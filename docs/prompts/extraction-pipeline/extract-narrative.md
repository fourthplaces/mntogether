# Extract Narrative (Narrative Strategy)

**Source:** `packages/extraction/src/pipeline/prompts.rs` — `EXTRACT_NARRATIVE_PROMPT`

**Model:** Varies by caller

**Type:** User prompt (contains `{query}` and `{pages}` placeholders)

## Prompt

```
Summarize information about: {query}

From these pages:
{pages}

Create a cohesive narrative that:
1. Synthesizes information from all relevant pages
2. Organizes information logically
3. Cites sources for key facts
4. Notes any contradictions between sources

Output JSON:
{
    "content": "Narrative summary as markdown with inline citations [1], [2], etc.",
    "sources": [
        {"number": 1, "url": "https://...", "title": "Page title"}
    ],
    "key_points": ["Main point 1", "Main point 2"],
    "conflicts": []
}
```
