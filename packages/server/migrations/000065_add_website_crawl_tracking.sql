-- Migration: Add website crawl tracking for multi-page crawling with retry mechanism
-- This migration:
-- 1. Adds crawl tracking fields to websites table
-- 2. Creates website_snapshot_listings junction table (decoupled listing tracking)
-- 3. Drops unused intelligent-crawler tables (keep page_snapshots)

-- ============================================================================
-- STEP 1: Add crawl tracking fields to websites table
-- ============================================================================

ALTER TABLE websites
    ADD COLUMN IF NOT EXISTS crawl_status TEXT DEFAULT 'pending'
        CHECK (crawl_status IN ('pending', 'crawling', 'completed', 'no_listings_found', 'failed')),
    ADD COLUMN IF NOT EXISTS crawl_attempt_count INT DEFAULT 0,
    ADD COLUMN IF NOT EXISTS max_crawl_retries INT DEFAULT 5,
    ADD COLUMN IF NOT EXISTS last_crawl_started_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS last_crawl_completed_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS pages_crawled_count INT DEFAULT 0,
    ADD COLUMN IF NOT EXISTS max_pages_per_crawl INT DEFAULT 20;

CREATE INDEX IF NOT EXISTS idx_websites_crawl_status ON websites(crawl_status);

-- ============================================================================
-- STEP 2: Create junction table linking website_snapshots to listings
-- ============================================================================

-- Junction table: links website_snapshots to listings (decoupled)
-- This allows tracking which listings were extracted from which page snapshot
CREATE TABLE IF NOT EXISTS website_snapshot_listings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    website_snapshot_id UUID NOT NULL REFERENCES website_snapshots(id) ON DELETE CASCADE,
    listing_id UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    extracted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(website_snapshot_id, listing_id)
);

CREATE INDEX IF NOT EXISTS idx_wsl_snapshot ON website_snapshot_listings(website_snapshot_id);
CREATE INDEX IF NOT EXISTS idx_wsl_listing ON website_snapshot_listings(listing_id);

-- ============================================================================
-- STEP 3: Create view for website_snapshots with listings info
-- ============================================================================

-- View to easily check which snapshots have listings
CREATE OR REPLACE VIEW website_snapshots_with_listings AS
SELECT
    ws.*,
    COUNT(wsl.listing_id) > 0 AS has_listings,
    COUNT(wsl.listing_id) AS listings_count
FROM website_snapshots ws
LEFT JOIN website_snapshot_listings wsl ON ws.id = wsl.website_snapshot_id
GROUP BY ws.id;

-- ============================================================================
-- STEP 4: Drop unused intelligent-crawler tables (keep page_snapshots)
-- ============================================================================

-- Drop unused tables in reverse dependency order
DROP TABLE IF EXISTS field_provenance;
DROP TABLE IF EXISTS relationships;
DROP TABLE IF EXISTS extractions;
DROP TABLE IF EXISTS detections;
DROP TABLE IF EXISTS schemas;

-- ============================================================================
-- Summary of Changes
-- ============================================================================
-- New columns on websites:
--   crawl_status ('pending', 'crawling', 'completed', 'no_listings_found', 'failed')
--   crawl_attempt_count (for retry tracking)
--   max_crawl_retries (default 5)
--   last_crawl_started_at, last_crawl_completed_at
--   pages_crawled_count, max_pages_per_crawl (default 20)
--
-- New table created:
--   website_snapshot_listings (junction table linking snapshots to listings)
--
-- New view created:
--   website_snapshots_with_listings (shows has_listings and listings_count)
--
-- Tables dropped:
--   field_provenance, relationships, extractions, detections, schemas
--   (page_snapshots kept for content caching)
