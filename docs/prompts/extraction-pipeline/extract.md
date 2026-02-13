# Extract (Collection Strategy)

**Source:** `packages/extraction/src/pipeline/prompts.rs` — `EXTRACT_PROMPT`

**Model:** Varies by caller

**Type:** User prompt (contains `{query}`, `{pages}`, and `{hints_section}` placeholders)

## Prompt

```
Extract information about: {query}

From these pages:
{pages}

Rules:
1. For EVERY claim, quote the source text that supports it
2. Note which page (URL) each quote comes from
3. Mark claims as:
   - DIRECT: Exact quote supports the claim
   - INFERRED: Reasonable inference from the source
   - ASSUMED: No direct evidence (WARNING: may be hallucination)
4. Explicitly note what information is MISSING (gaps)
5. If sources contradict each other, note the conflict

{hints_section}

Output JSON:
{
    "content": "Extracted information as markdown",
    "claims": [
        {
            "statement": "The claim being made",
            "evidence": [
                {
                    "quote": "Exact quote from source",
                    "source_url": "https://..."
                }
            ],
            "grounding": "DIRECT" | "INFERRED" | "ASSUMED"
        }
    ],
    "sources": [
        {
            "url": "https://...",
            "role": "PRIMARY" | "SUPPORTING" | "CORROBORATING"
        }
    ],
    "gaps": [
        {
            "field": "What's missing (e.g., 'contact email')",
            "query": "Search query to find it (e.g., 'the contact email for the volunteer coordinator')"
        }
    ],
    "conflicts": [
        {
            "topic": "What the conflict is about",
            "claims": [
                {"statement": "Claim A", "source_url": "url1"},
                {"statement": "Claim B", "source_url": "url2"}
            ]
        }
    ]
}
```
