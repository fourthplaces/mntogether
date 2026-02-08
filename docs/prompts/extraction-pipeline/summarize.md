# Summarize Page

**Source:** `packages/extraction/src/pipeline/prompts.rs` — `SUMMARIZE_PROMPT`

**Model:** Varies by caller

**Type:** User prompt (contains `{url}` and `{content}` placeholders)

## Prompt

```
Summarize this webpage for information retrieval.

Your summary must capture:
1. What the page offers (services, programs, opportunities)
2. What the page asks for (volunteers, donations, applications)
3. Calls to action (sign up, apply, contact, donate)
4. Key entities (organization names, locations, dates, contacts)

Output JSON:
{
    "summary": "2-3 sentence overview focusing on actionable content",
    "signals": {
        "offers": ["list of things offered - services, programs, opportunities"],
        "asks": ["list of things requested - volunteers, donations, applications"],
        "calls_to_action": ["list of CTAs - sign up, apply, contact, donate"],
        "entities": ["key proper nouns - org names, locations, dates, contacts"]
    },
    "language": "detected language code (en, es, etc.)"
}

Page URL: {url}
Page Content:
{content}
```
