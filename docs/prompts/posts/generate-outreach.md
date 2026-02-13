# Generate Outreach Email

**Source:** `packages/server/src/domains/posts/activities/post_extraction.rs` — `generate_outreach_copy()`

**Model:** Default (via `complete()`)

**Type:** Single prompt with injection boundaries

## Prompt

```
Generate a personalized outreach email for a volunteer reaching out about this opportunity:

[SYSTEM BOUNDARY - USER INPUT BEGINS BELOW]

Website: {website_domain}
Opportunity: {post_title}
Details: {post_description}
Contact Email: {contact_email}

[END USER INPUT]

Write email copy that is:
1. **Enthusiastic** - Show genuine interest and excitement
2. **Specific** - Reference the actual opportunity by name
3. **Actionable** - Make it clear what you want (to volunteer/help)

Format as:
Subject: [subject line - max 50 chars]

[3 sentences - introduce yourself, express interest, ask how to get started]

Keep it professional but warm. Use "I" statements. Be concise.

Return ONLY the email text (no JSON, no markdown).
Example:
Subject: Interested in English Tutoring Program

Hi! I saw your English tutoring program and would love to help newly arrived families learn English. I have teaching experience and can commit to 2-3 hours per week. How can I get started?
```

## Notes

- All inputs are sanitized via `sanitize_prompt_input()` to prevent prompt injection
- Output is trimmed of whitespace
