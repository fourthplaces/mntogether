---
title: "feat: Auto-create organizations from crawled websites"
type: feat
date: 2026-02-10
---

# Auto-Create Organizations from Crawled Websites

## Overview

After a website is crawled (existing 3-pass extraction pipeline), run one additional LLM pass on the already-fetched pages to extract organization-level info (name, description, social media links). Create an Organization, link the website to it, and create SocialProfiles for discovered social accounts. Skip on re-crawl if website already has an org. Provide a backfill endpoint for existing websites without organizations.

## Problem Statement

Websites are crawled and posts extracted, but there's no automatic way to create the Organization that owns each website. Admins must manually create organizations and link websites — tedious for 100+ websites. The crawled pages already contain the org info we need (name in headers, description on about pages, social links in footers).

## Proposed Solution

Add a 4th step to the crawl pipeline: **organization extraction**. This is an activity function callable from both the crawl workflow and a standalone backfill endpoint.

### Flow

```
Crawl website (existing 3-pass pipeline)
    |
    v
website.organization_id IS NULL?
    |                         |
    v (yes)                   v (no)
Extract org info           Skip
from crawled pages
    |
    v
Find-or-create Organization by name
    |
    v
Link website -> organization
    |
    v
Find-or-create SocialProfiles for discovered links
```

## Technical Approach

### 1. LLM Response Type

```rust
// packages/server/src/domains/crawling/activities/org_extraction.rs

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ExtractedOrganization {
    /// The organization's official name
    pub name: String,
    /// Brief description or mission statement
    pub description: Option<String>,
    /// Social media profiles found on the website
    pub social_links: Vec<ExtractedSocialLink>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ExtractedSocialLink {
    /// Normalized platform name: "instagram", "facebook", "tiktok", "twitter", "linkedin", "youtube"
    pub platform: String,
    /// Handle/username (without @ prefix)
    pub handle: String,
    /// Full URL if available
    pub url: Option<String>,
}
```

### 2. Page Selection for LLM Context

Don't send all pages. Select the most relevant ones:
- Homepage (shortest URL path or `/`)
- About page (URL contains `about`)
- Contact page (URL contains `contact`)
- Fallback: first 3 pages by URL depth

Cap total content at **50K characters** for a single LLM call.

```rust
fn select_org_pages(pages: &[(Uuid, String, String)]) -> Vec<&(Uuid, String, String)> {
    let mut selected = Vec::new();

    // Priority: homepage, about, contact
    for page in pages {
        let url_lower = page.1.to_lowercase();
        let path = url_lower.split('/').skip(3).collect::<Vec<_>>().join("/");
        if path.is_empty() || path == "/"
            || url_lower.contains("about")
            || url_lower.contains("contact") {
            selected.push(page);
        }
    }

    // Fallback: first 3 pages if none matched
    if selected.is_empty() {
        selected = pages.iter().take(3).collect();
    }

    selected
}
```

### 3. Activity Function

```
packages/server/src/domains/crawling/activities/org_extraction.rs (NEW)
```

- Load extraction pages for the website's domain
- Select relevant pages (homepage, about, contact)
- Call `OpenAIClient.extract::<ExtractedOrganization>()` with org extraction prompt
- Validate org name (min 2 chars, not generic like "Home", "About Us", "N/A")
- `Organization::find_or_create_by_name()` — exact match dedup
- `Website::set_organization_id()`
- For each social link: normalize handle, `SocialProfile::find_or_create()`
- Wrap DB operations in a transaction

### 4. Handle Normalization

Before storing social profile handles:
- Strip `@` prefix
- Lowercase
- Extract handle from full URLs (e.g., `https://instagram.com/handle/` -> `handle`)
- Trim whitespace

### 5. Integration into Crawl Pipeline

```
packages/server/src/domains/crawling/activities/crawl_full.rs (MODIFY)
```

Add org extraction as the final step, after post extraction/sync:

```rust
pub async fn crawl_website_full(...) -> Result<CrawlWebsiteResult> {
    // Step 1: Ingest website pages (existing)
    ingest_website(website_id, visitor_id, deps).await?;

    // Step 2: Extract posts (existing TODO, being wired up separately)
    // ...

    // Step 3: Auto-create organization (NEW)
    let website = Website::find_by_id(website_id, &deps.db_pool).await?;
    if website.organization_id.is_none() {
        if let Err(e) = extract_and_create_organization(website_id, deps).await {
            tracing::warn!("Org extraction failed for website {}: {}", website_id, e);
            // Don't fail the crawl — org extraction is best-effort
        }
    }

    Ok(CrawlWebsiteResult { ... })
}
```

### 6. Backfill Endpoint

```
packages/server/src/domains/organization/restate/services/organizations.rs (MODIFY)
```

Add a new method to OrganizationsService:

```rust
async fn backfill_organizations(req: EmptyRequest) -> Result<BackfillResult, HandlerError>;
```

- Requires admin auth
- Queries `SELECT id FROM websites WHERE organization_id IS NULL AND deleted_at IS NULL`
- Iterates sequentially (one at a time to control LLM costs)
- Calls `extract_and_create_organization()` for each
- Continues on individual failures (logs warning, moves to next)
- Returns `{ processed: usize, succeeded: usize, failed: usize }`

### 7. Model Changes

**Organization model** (`organization.rs`) — add:
- `find_or_create_by_name(name, description, pool)` — `INSERT ... ON CONFLICT (name) DO UPDATE SET description = COALESCE(EXCLUDED.description, organizations.description) RETURNING *`

**Website model** (`website.rs`) — add:
- `set_organization_id(website_id, org_id, pool)` — `UPDATE websites SET organization_id = $2 WHERE id = $1`

**SocialProfile model** (`social_profile.rs`) — add:
- `find_or_create(org_id, platform, handle, url, pool)` — `INSERT ... ON CONFLICT (platform, handle) DO NOTHING RETURNING *` with fallback query

### 8. New Migration

```
packages/server/migrations/000146_add_org_name_unique_and_cascade_deletes.sql
```

```sql
-- Add unique constraint on organization name for find_or_create dedup
ALTER TABLE organizations ADD CONSTRAINT organizations_name_unique UNIQUE (name);

-- Fix cascade behavior for organization deletion
ALTER TABLE websites DROP CONSTRAINT IF EXISTS websites_organization_id_fkey;
ALTER TABLE websites ADD CONSTRAINT websites_organization_id_fkey
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE SET NULL;

ALTER TABLE social_profiles DROP CONSTRAINT IF EXISTS social_profiles_organization_id_fkey;
ALTER TABLE social_profiles ADD CONSTRAINT social_profiles_organization_id_fkey
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE;
```

## Acceptance Criteria

- [x] After a new website is crawled, an Organization is automatically created with name + description extracted from page content
- [x] Website is linked to the created organization (`organization_id` set)
- [x] Social media profiles found on the website are created as SocialProfiles
- [x] Re-crawling a website with an existing org skips extraction
- [x] Duplicate org names resolve to the existing org (find-or-create)
- [x] Duplicate social handles (same platform+handle) don't error
- [x] Social handles are normalized (no `@`, lowercase, URLs resolved to handles)
- [x] LLM extraction failure doesn't fail the crawl (best-effort, logs warning)
- [x] Backfill endpoint processes all websites without orgs
- [x] Backfill continues on individual failures
- [x] Backfill requires admin auth
- [x] Org name validation rejects empty/generic names ("Home", "N/A", etc.)

## File Changes Summary

| File | Action | What |
|------|--------|------|
| `crawling/activities/org_extraction.rs` | **NEW** | LLM extraction activity + types + prompt |
| `crawling/activities/mod.rs` | MODIFY | Export new activity |
| `crawling/activities/crawl_full.rs` | MODIFY | Add org extraction step |
| `organization/models/organization.rs` | MODIFY | Add `find_or_create_by_name` |
| `website/models/website.rs` | MODIFY | Add `set_organization_id` |
| `social_profile/models/social_profile.rs` | MODIFY | Add `find_or_create` |
| `organization/restate/services/organizations.rs` | MODIFY | Add `backfill_organizations` endpoint |
| `migrations/000146_...sql` | **NEW** | Unique name constraint + cascade fixes |

## Edge Cases Handled

- **No pages crawled**: Skip extraction, log warning
- **LLM returns empty/generic name**: Skip org creation
- **Two websites same org name**: find_or_create deduplicates on exact name match
- **Same social handle from multiple websites**: ON CONFLICT DO NOTHING
- **LLM call fails (rate limit, timeout)**: Log warning, don't fail crawl
- **Backfill partial failure**: Continue processing remaining websites
- **Website already has org (re-crawl)**: Skip entirely

## Future Improvements (Not in Scope)

- Fuzzy org name matching (Levenshtein / embedding similarity)
- Re-extract and update org description on re-crawl
- Auto-trigger social profile scraping after creation
- Org extraction using post-extraction context (contact info, etc.)
