# Generate Summary (TLDR)

**Source:** `packages/server/src/domains/posts/activities/post_extraction.rs` — `generate_summary()`

**Model:** Default (via `complete()`)

**Type:** Single prompt with injection boundaries

## Prompt

```
Summarize this listing in 1-2 clear sentences. Focus on what help is needed and the impact.

[SYSTEM BOUNDARY - USER INPUT BEGINS BELOW]

Description:
{description}

[END USER INPUT]

Return ONLY the summary (no markdown, no explanation).
```

## Notes

- Input description is sanitized via `sanitize_prompt_input()`
- Output is trimmed of whitespace
