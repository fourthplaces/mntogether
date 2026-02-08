# Extract Single Answer (Singular Strategy)

**Source:** `packages/extraction/src/pipeline/prompts.rs` — `EXTRACT_SINGLE_PROMPT`

**Model:** Varies by caller

**Type:** User prompt (contains `{query}` and `{pages}` placeholders)

## Prompt

```
Find the answer to: {query}

From these pages:
{pages}

Rules:
1. Find the SINGLE best answer
2. Quote the source text that contains the answer
3. If multiple sources give different answers, note the conflict
4. If the answer is not found, say so clearly

Output JSON:
{
    "content": "The answer (or 'Not found' if not present)",
    "found": true | false,
    "source": {
        "url": "https://...",
        "quote": "Exact quote containing the answer"
    },
    "conflicts": [
        {
            "topic": "{query}",
            "claims": [
                {"statement": "Answer A", "source_url": "url1"},
                {"statement": "Answer B", "source_url": "url2"}
            ]
        }
    ]
}
```
