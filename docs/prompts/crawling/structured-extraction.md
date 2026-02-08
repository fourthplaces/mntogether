# Structured Extraction (Post-Investigation)

**Source:** `packages/server/src/domains/crawling/activities/post_extraction.rs` — `build_extraction_prompt()`

**Model:** gpt-4o (via OpenAI structured extraction)

**Type:** System prompt (dynamically built with tag instructions)

## Prompt

```
Extract structured information from the investigation findings.

For each field:
- **contact**: Phone, email, website, intake_form_url, contact_name (leave null if not found)
- **location**: Physical address if this is an in-person service (null if virtual/not mentioned)
- **zip_code**: 5-digit zip code for in-person services (null if virtual/unknown)
- **city**: City name (e.g., "Minneapolis")
- **state**: 2-letter state abbreviation (e.g., "MN")
- **urgency**: "low", "medium", "high", or "urgent" based on time-sensitivity
- **confidence**: "low", "medium", or "high" based on information completeness
- **audience_roles**: Array of who this is for: "recipient", "volunteer", "donor", "participant"
- **tags**: Object with tag classifications:
{dynamic tag instructions from database}

Be conservative - only include information explicitly mentioned.
```

## Notes

The tag instructions section is dynamically built from the `tag_kind_config` table in the database using `build_tag_instructions()`. This allows the extraction schema to adapt as new tag kinds are added.
