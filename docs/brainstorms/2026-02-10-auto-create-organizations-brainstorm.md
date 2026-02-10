---
date: 2026-02-10
topic: auto-create-organizations
---

# Auto-Create Organizations from Crawled Websites

## What We're Building

After a website is crawled (3-pass extraction pipeline), run an additional LLM pass on the already-fetched pages to extract organization-level info: name, description, and social media links. Automatically create an Organization, link it to the website, and create SocialProfiles for any discovered social accounts.

Also provide a backfill endpoint to trigger this process for all websites that don't have an organization yet.

## Flow

```
Crawl website (existing 3-pass pipeline)
    ↓
Check: website.organization_id IS NULL?
    ↓ yes                    ↓ no
Extract org info             Skip (already has org)
from crawled pages
    ↓
Create Organization
    ↓
Link website → organization
    ↓
Create SocialProfiles for found social links
```

## Key Decisions

- **Uses existing crawled pages**: No additional fetching. Homepage/about pages already in `extraction_pages` contain org name, description, and social links (footers/headers).
- **Single LLM pass**: One structured extraction call targeting org-level info (not post-level).
- **Skip on re-crawl**: If website already has an organization_id, skip org extraction entirely. Future: sync/update functionality if needed.
- **Activity, not separate workflow**: Implement as an activity callable from both the crawl workflow and a standalone backfill endpoint.
- **Backfill via Restate endpoint**: Query `websites WHERE organization_id IS NULL`, trigger org extraction for each.

## Extracted Fields

From crawled pages, extract:
- `organization_name` — from page title, about page, header
- `description` — mission statement, about blurb
- `social_profiles[]` — platform + handle pairs from social links (Instagram, Facebook, TikTok, Twitter/X, LinkedIn, YouTube)

## Open Questions

- Should the backfill be rate-limited or batched to avoid LLM cost spikes?
- Should we extract any additional org metadata (logo URL, address, founding year)?

## Next Steps

→ `/workflows:plan` for implementation details
