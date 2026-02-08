# Partition Pages

**Source:** `packages/extraction/src/pipeline/prompts.rs` — `PARTITION_PROMPT`

**Model:** Varies by caller

**Type:** User prompt (contains `{query}` and `{summaries}` placeholders)

## Prompt

```
Given a query and page summaries, identify distinct items to extract.

Query: {query}

For this query, determine:
1. What constitutes ONE distinct item?
2. Which pages contribute to each item?
3. Why are these pages grouped together?

Page Summaries:
{summaries}

Output JSON array:
[
    {
        "title": "Brief item title",
        "urls": ["url1", "url2"],
        "rationale": "Why these pages are grouped"
    }
]

Rules:
- Each item should be distinct (no duplicates)
- Pages can appear in multiple items if they contain multiple distinct things
- If a page contains only one item, it gets its own partition
- Group pages that discuss the SAME specific thing
```
