# Complete Snapshot Traceability System

## Overview
This document describes the complete data flow for tracking where listings come from, with full traceability from domain → page URL → cached content → extracted listings.

## Database Schema After Migration 49

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          DOMAIN APPROVAL FLOW                            │
└─────────────────────────────────────────────────────────────────────────┘

                    ┌──────────────────┐
                    │    DOMAINS       │
                    │                  │
                    │ • id             │◄────┐
                    │ • domain_url     │     │
                    │ • status         │     │
                    │   - pending      │     │  Foreign Keys
                    │   - approved ✓   │     │
                    │   - rejected     │     │
                    │   - suspended    │     │
                    └──────────────────┘     │
                            │                │
                            │ 1:N            │
                            ▼                │
┌─────────────────────────────────────────────────────────────────────────┐
│                        PAGE SUBMISSION FLOW                              │
└─────────────────────────────────────────────────────────────────────────┘
                                               │
                    ┌──────────────────────┐   │
                    │ DOMAIN_SNAPSHOTS     │   │
                    │                      │   │
                    │ • id                 │   │
                    │ • domain_id          │───┘
                    │ • page_url           │
                    │ • page_snapshot_id   │───┐
                    │ • scrape_status      │   │
                    │   - pending          │   │
                    │   - scraped ✓        │   │
                    │   - failed           │   │
                    │ • last_scraped_at    │   │
                    │ • submitted_by       │   │
                    └──────────────────────┘   │
                            │                  │
                            │ N:1              │
                            ▼                  │
┌─────────────────────────────────────────────────────────────────────────┐
│                       CONTENT CACHING LAYER                              │
└─────────────────────────────────────────────────────────────────────────┘
                                               │
                    ┌──────────────────────┐   │
                    │  PAGE_SNAPSHOTS      │◄──┘
                    │                      │
                    │ • id                 │
                    │ • url                │
                    │ • content_hash       │
                    │ • html               │ ← Cached HTML content
                    │ • markdown           │ ← Cached markdown
                    │ • crawled_at         │
                    │ • fetched_via        │
                    │                      │
                    │ NEW COLUMNS:         │
                    │ • listings_count     │ ← Auto-updated count
                    │ • extraction_status  │
                    │ • extraction_done_at │
                    └──────────────────────┘
                            │
                            │ 1:N
                            ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      EXTRACTED LISTINGS                                  │
└─────────────────────────────────────────────────────────────────────────┘

                    ┌──────────────────────┐
                    │     LISTINGS         │
                    │                      │
                    │ • id                 │
                    │ • title              │
                    │ • description        │
                    │ • status             │
                    │ • domain_id          │ ← Which domain
                    │ • source_url         │ ← Which page URL
                    │                      │
                    │ NEW COLUMN:          │
                    │ • page_snapshot_id   │ ← Link to cached content!
                    │                      │
                    └──────────────────────┘
```

## Complete Traceability Chain

### User Submits Resource
```
User: "https://example.org/volunteer-opportunities"
  ↓
System extracts: domain = "example.org"
  ↓
Check: Does domain exist?
  ├─ NO  → Create domain (status=pending_review) ⏸️  STOP
  └─ YES → Is domain approved?
           ├─ NO  → ⏸️  STOP (wait for admin approval)
           └─ YES → ✓ Continue to scraping
```

### Admin Approves Domain
```
Admin: Approve "example.org"
  ↓
Update: domain.status = 'approved'
  ↓
Find: All pending domain_snapshots for this domain
  ↓
For each page URL:
  1. Scrape page → Get HTML + markdown
  2. Create page_snapshot record ✓
  3. Link: domain_snapshot.page_snapshot_id = page_snapshot.id
  4. Extract listings with AI
  5. Create listings with page_snapshot_id ✓
```

### Data Flow (Scraping → Extraction)

```
┌─────────────────────────────────────────────────────────────────┐
│ STEP 1: SCRAPE                                                  │
└─────────────────────────────────────────────────────────────────┘

Scraper fetches: https://example.org/volunteer-opportunities
  ↓
Receives:
  • HTML: "<div>Food Bank needs volunteers...</div>"
  • Markdown: "# Food Bank\n\nWe need volunteers..."
  ↓
INSERT INTO page_snapshots (
  id = uuid1,
  url = "https://example.org/volunteer-opportunities",
  html = "...",
  markdown = "...",
  content_hash = sha256(html),
  crawled_at = NOW()
)
  ↓
UPDATE domain_snapshots
  SET page_snapshot_id = uuid1,
      scrape_status = 'scraped',
      last_scraped_at = NOW()
  WHERE domain_id = X AND page_url = "https://example.org/volunteer-opportunities"

┌─────────────────────────────────────────────────────────────────┐
│ STEP 2: EXTRACT LISTINGS (AI)                                  │
└─────────────────────────────────────────────────────────────────┘

AI analyzes page_snapshot.markdown
  ↓
Extracts 3 listings:
  1. "Food Bank Volunteer"
  2. "Tutoring Help Needed"
  3. "Community Garden Support"
  ↓
For each listing:
  INSERT INTO listings (
    id = uuid2,
    title = "Food Bank Volunteer",
    description = "...",
    domain_id = X,
    source_url = "https://example.org/volunteer-opportunities",
    page_snapshot_id = uuid1,  ← KEY LINK!
    status = 'pending_approval'
  )
  ↓
Trigger fires: update_page_snapshot_listings_count()
  ↓
UPDATE page_snapshots
  SET listings_extracted_count = 3,
      extraction_status = 'completed',
      extraction_completed_at = NOW()
  WHERE id = uuid1
```

## Admin UI Queries

### View 1: Pending Domains Queue
```sql
SELECT
  id,
  domain_url,
  status,
  submitter_type,
  submission_context,
  created_at,
  (SELECT COUNT(*) FROM domain_snapshots WHERE domain_id = domains.id) as pending_pages
FROM domains
WHERE status = 'pending_review'
ORDER BY created_at DESC;
```

### View 2: Domain Detail with Page Snapshots
```sql
SELECT
  d.id,
  d.domain_url,
  d.status,
  ds.id as snapshot_id,
  ds.page_url,
  ds.scrape_status,
  ds.last_scraped_at,
  ps.id as page_snapshot_id,
  ps.listings_extracted_count,
  ps.crawled_at,
  COALESCE(ps.listings_extracted_count, 0) as listings_count
FROM domains d
LEFT JOIN domain_snapshots ds ON ds.domain_id = d.id
LEFT JOIN page_snapshots ps ON ps.id = ds.page_snapshot_id
WHERE d.id = $1
ORDER BY ds.submitted_at DESC;
```

### View 3: Listings from Specific Page
```sql
SELECT
  l.id,
  l.title,
  l.description,
  l.status,
  l.urgency,
  l.created_at,
  ps.markdown as source_content,  -- Can view original content!
  ps.crawled_at as scraped_at
FROM listings l
JOIN page_snapshots ps ON ps.id = l.page_snapshot_id
WHERE l.domain_id = $1
  AND l.source_url = $2
ORDER BY l.created_at DESC;
```

### View 4: Domain Statistics (Dashboard)
```sql
SELECT * FROM domain_statistics
WHERE domain_status = 'approved'
ORDER BY last_scraped_at DESC NULLS LAST;

-- Returns:
-- domain_id, domain_url, total_page_urls, scraped_pages, pending_pages,
-- failed_pages, total_snapshots, total_listings, active_listings, etc.
```

## Admin UI Mockup

```
╔════════════════════════════════════════════════════════════════════╗
║ Domain Detail: example.org                          Status: APPROVED║
╠════════════════════════════════════════════════════════════════════╣
║ Pages: 3 total | 2 scraped | 1 pending                             ║
║ Listings: 15 total | 12 active | 3 pending approval                ║
║                                                                     ║
║ ┌─────────────────────────────────────────────────────────────┐   ║
║ │ Page: /volunteer-opportunities                              │   ║
║ │ Status: ✓ Scraped 2 hours ago                              │   ║
║ │ Listings: 5 (view details ▼)                               │   ║
║ │   • Food Bank Volunteer [approved] ← Click to see listing  │   ║
║ │   • Tutoring Help [pending]                                │   ║
║ │   • Garden Support [approved]                              │   ║
║ │   • Meal Delivery [approved]                               │   ║
║ │   • Tech Tutoring [pending]                                │   ║
║ │                                                            │   ║
║ │ [View Cached Content] [Re-scrape] [Delete]                │   ║
║ └─────────────────────────────────────────────────────────────┘   ║
║                                                                     ║
║ ┌─────────────────────────────────────────────────────────────┐   ║
║ │ Page: /events                                               │   ║
║ │ Status: ✓ Scraped 5 hours ago                              │   ║
║ │ Listings: 8 (view details ▼)                               │   ║
║ │ [View Cached Content] [Re-scrape]                          │   ║
║ └─────────────────────────────────────────────────────────────┘   ║
║                                                                     ║
║ ┌─────────────────────────────────────────────────────────────┐   ║
║ │ Page: /contact                                              │   ║
║ │ Status: ⏳ Pending (not scraped yet)                        │   ║
║ │ Submitted: 1 day ago                                        │   ║
║ │ [Scrape Now]                                                │   ║
║ └─────────────────────────────────────────────────────────────┘   ║
╚════════════════════════════════════════════════════════════════════╝
```

## Benefits of Complete Traceability

✅ **See where listings come from**: Domain → Page URL → Cached Content
✅ **View original content**: Click to see HTML/markdown that was scraped
✅ **Debug extraction**: If AI missed listings, view source to understand why
✅ **Track quality**: See which pages produce the most/best listings
✅ **Re-scrape failed pages**: Retry specific pages without re-scraping entire domain
✅ **Audit trail**: Complete history of what was scraped when
✅ **Content deduplication**: content_hash prevents duplicate scraping
✅ **Performance**: Cached content means no re-fetching for analysis

## Key Changes from Migration 49

1. **listings.page_snapshot_id** - NEW column linking to cached content
2. **page_snapshots.listings_extracted_count** - Auto-updated count via trigger
3. **page_snapshots.extraction_status** - Track extraction pipeline
4. **domain_statistics** - Pre-aggregated view for fast dashboard
5. **page_snapshot_details** - Comprehensive view for admin UI
6. **get_listings_by_domain_page()** - Helper function for drill-down

## Next Steps

1. ✅ Run migration 49
2. ⏳ Update scraper.rs to create page_snapshot records
3. ⏳ Update AI extraction to set page_snapshot_id on listings
4. ⏳ Add GraphQL queries for admin UI
5. ⏳ Build admin UI pages (PendingDomains, DomainDetail)
