# Domain Approval Workflow Architecture

## Overview

Resources are approved at the **domain level**, not individual URLs. Once a domain is approved, the intelligent-crawler automatically discovers and extracts content from all pages within that domain (up to `max_crawl_depth`).

## User Workflows

### Workflow 1: Admin/User Submits a Domain

```
User submits "foodbank.org"
  ↓
CREATE domain (status='pending_review', max_crawl_depth=3)
  ↓
Admin reviews in admin interface
  ↓
Admin approves → UPDATE status='approved'
  ↓
Trigger intelligent-crawler Discovery with max_depth
  ↓
Crawler automatically:
  - Discovers pages (up to depth 3)
  - Flags relevant pages (AI determines relevance)
  - Extracts structured data
  - Stores in page_snapshots (with content_hash caching)
  ↓
Crawler Refresh effect monitors for changes (automatic recurring)
```

### Workflow 2: User Submits Specific URL

```
User submits "foodbank.org/services/housing"
  ↓
Extract domain: "foodbank.org"
  ↓
Check: Is foodbank.org approved?
  ↓
  YES → Send URL directly to crawler (auto-approved)
  NO  → Prompt: "Would you like to submit foodbank.org for review?"
```

## Database Schema

### domains table (updated)

```sql
CREATE TABLE domains (
  id UUID PRIMARY KEY,
  domain_url TEXT NOT NULL UNIQUE,
  scrape_frequency_hours INT DEFAULT 24,
  last_scraped_at TIMESTAMPTZ,
  active BOOL DEFAULT true,

  -- Approval workflow
  status TEXT CHECK (status IN ('pending_review', 'approved', 'rejected', 'suspended')),
  submitted_by UUID REFERENCES members(id),
  submitter_type TEXT CHECK (submitter_type IN ('admin', 'public_user', 'system')),
  submission_context TEXT,
  reviewed_by UUID REFERENCES members(id),
  reviewed_at TIMESTAMPTZ,
  rejection_reason TEXT,

  -- Crawling config
  max_crawl_depth INT DEFAULT 3,
  crawl_rate_limit_seconds INT DEFAULT 2,
  is_trusted_domain BOOL DEFAULT false,

  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

### domain_scrape_urls (DROPPED)

This table is **no longer needed**. The intelligent-crawler automatically discovers pages within approved domains.

## Intelligent-Crawler Integration

### 1. Trigger Crawler on Domain Approval

```rust
// When admin approves domain
Domain::approve(domain_id, admin_id, pool).await?;

// Send command to crawler
let command = CrawlerCommand::DiscoverDomain {
    domain_id,
    start_url: domain.domain_url,
    max_depth: domain.max_crawl_depth,
    rate_limit: domain.crawl_rate_limit_seconds,
};

crawler.execute(command).await?;
```

### 2. Crawler Automatically Handles Everything

The intelligent-crawler has built-in phases:

- **Discovery**: Fetches pages, follows links (up to max_depth)
- **Flagging**: AI determines if page is relevant
- **Extraction**: Extracts structured data (services, opportunities, businesses)
- **Refresh**: Monitors approved domains for content changes

### 3. Caching is Automatic

The `page_snapshots` table automatically caches Firecrawl results:

```sql
INSERT INTO page_snapshots (url, content_hash, markdown, ...)
ON CONFLICT (url, content_hash) DO NOTHING
```

**Result:** Unchanged pages don't trigger Firecrawl API calls. ✅

### 4. Skip Unproductive URLs Automatically

Query crawler data to identify dead URLs:

```sql
-- URLs that have been flagged but never produced extractions
SELECT ps.url
FROM page_snapshots ps
LEFT JOIN extractions e ON e.page_snapshot_id = ps.id
WHERE ps.crawled_at > NOW() - INTERVAL '90 days'
GROUP BY ps.url
HAVING COUNT(e.id) = 0;
```

Crawler's Refresh effect can use this to skip rechecking dead pages.

## Admin Interface Requirements

### Domain Submission Form

**Fields:**
- Domain URL (e.g., "foodbank.org")
- Context/Description (optional): "State food bank directory"
- Max Crawl Depth (default: 3)
  - 0 = Homepage only
  - 1 = Homepage + direct links
  - 2-3 = Recommended
  - 4+ = Caution (can be slow)

**Validation:**
- Extract domain from URL (strip protocol, path)
- Check if domain already exists
- If exists and approved: "Already approved, would you like to re-scrape?"
- If exists and pending: "Already submitted for review"

### Domain Review Queue

**Display:**
- List of domains with status='pending_review'
- Show: domain_url, submitted_by, submission_context, created_at
- Actions: Approve, Reject (with reason)

**On Approve:**
1. Update status='approved'
2. Trigger crawler Discovery command
3. Show: "Crawling started. Check back in a few minutes."

**On Reject:**
1. Update status='rejected'
2. Require rejection_reason
3. Optionally notify submitter

### Approved Domains List

**Display:**
- List of domains with status='approved'
- Show: domain_url, last_scraped_at, listings count
- Actions:
  - Re-scrape now (trigger crawler)
  - Mark as trusted (auto-approve URLs from this domain)
  - Suspend (pause crawling)
  - View extractions

## Key Architectural Decisions

### ✅ Domain-level approval (not URL-level)

**Why:** Reduces admin burden. Once a domain is trusted (e.g., state government site), all pages from that domain can be crawled automatically.

### ✅ Dropped domain_scrape_urls table

**Why:** Intelligent-crawler automatically discovers pages via link following. No need to manually specify URLs.

### ✅ max_crawl_depth prevents runaway crawling

**Why:** Some sites have millions of pages. Depth limit keeps crawling manageable.

**Recommendation:**
- Depth 0: Homepage only (e.g., single-page sites)
- Depth 1-2: Small sites (e.g., local nonprofit with 20 pages)
- Depth 3: Medium sites (default, recommended)
- Depth 4+: Large sites (use with caution, monitor performance)

### ✅ Automatic Firecrawl caching via page_snapshots

**Why:** page_snapshots uses `(url, content_hash)` as unique key. If content hasn't changed, no Firecrawl API call needed.

### ✅ Crawler's Refresh effect handles recurring checks

**Why:** Built into intelligent-crawler. Automatically monitors approved domains for changes based on `scrape_frequency_hours`.

### ✅ Trust flag for auto-approval

**Why:** Highly trusted domains (e.g., .gov sites) can have URLs auto-approved without review, speeding up processing.

## Migration Path

### Step 1: Run Migration
```bash
sqlx migrate run
# Runs 000047_add_domain_approval_workflow.sql
```

### Step 2: Backfill Existing Domains
```sql
-- Mark existing domains as approved
UPDATE domains SET status = 'approved' WHERE status IS NULL;
```

### Step 3: Update Scraping Code
- Remove direct Firecrawl calls in `scraper.rs`
- Replace with crawler Discovery commands
- Listen to crawler events (PageFlagged, ExtractionCompleted)

### Step 4: Build Admin UI
- Domain submission form
- Review queue interface
- Approved domains list

## Questions Answered

### Q: "Does this cache Firecrawl results?"
**A:** Yes! `page_snapshots` table automatically caches via `(url, content_hash)` unique constraint.

### Q: "How do we skip unproductive URLs?"
**A:** Query `page_snapshots` LEFT JOIN `extractions` to find URLs with 0 extractions. Crawler can skip these during Refresh.

### Q: "What if we want to scrape a specific URL from an unapproved domain?"
**A:** Prompt user: "Would you like to submit [domain] for review?" Once domain is approved, that URL will be crawled.

### Q: "How do we prevent runaway crawling?"
**A:** `max_crawl_depth` limits how deep the crawler goes. Default is 3 levels.

### Q: "Can users submit domains, or only admins?"
**A:** Both! Set `submitter_type='public_user'` for user submissions. They go to the same review queue.

### Q: "What about trusted domains like .gov sites?"
**A:** Mark as `is_trusted_domain=true`. URLs from these domains bypass review and go straight to crawler.
