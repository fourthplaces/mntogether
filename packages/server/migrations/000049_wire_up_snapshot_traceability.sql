-- Migration 49: Wire up complete snapshot traceability
-- This enables: Domain -> Page URL -> Cached Content -> Extracted Listings

-- ============================================================================
-- PART 1: Add missing link from listings to page_snapshots
-- ============================================================================

-- Add page_snapshot_id to listings so we can trace back to the cached content
ALTER TABLE listings
  ADD COLUMN IF NOT EXISTS page_snapshot_id UUID
  REFERENCES page_snapshots(id) ON DELETE SET NULL;

-- Index for querying listings by page snapshot
CREATE INDEX IF NOT EXISTS idx_listings_page_snapshot_id
  ON listings(page_snapshot_id) WHERE page_snapshot_id IS NOT NULL;

-- Index for finding listings from a specific page URL within a domain
CREATE INDEX IF NOT EXISTS idx_listings_domain_source_url
  ON listings(domain_id, source_url) WHERE domain_id IS NOT NULL AND source_url IS NOT NULL;

-- ============================================================================
-- PART 2: Enhance page_snapshots for better admin UI
-- ============================================================================

-- Add extraction metadata to track how many listings came from this page
ALTER TABLE page_snapshots
  ADD COLUMN IF NOT EXISTS listings_extracted_count INTEGER DEFAULT 0,
  ADD COLUMN IF NOT EXISTS extraction_completed_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS extraction_status TEXT DEFAULT 'pending'
    CHECK (extraction_status IN ('pending', 'processing', 'completed', 'failed'));

CREATE INDEX IF NOT EXISTS idx_page_snapshots_extraction_status
  ON page_snapshots(extraction_status);

-- ============================================================================
-- PART 3: Add helper function to update extraction counts
-- ============================================================================

-- Function to automatically update listings count when listings are added
CREATE OR REPLACE FUNCTION update_page_snapshot_listings_count()
RETURNS TRIGGER AS $$
BEGIN
  IF NEW.page_snapshot_id IS NOT NULL THEN
    UPDATE page_snapshots
    SET listings_extracted_count = (
      SELECT COUNT(*)
      FROM listings
      WHERE page_snapshot_id = NEW.page_snapshot_id
    )
    WHERE id = NEW.page_snapshot_id;
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger to keep counts in sync
DROP TRIGGER IF EXISTS trigger_update_page_snapshot_count ON listings;
CREATE TRIGGER trigger_update_page_snapshot_count
  AFTER INSERT OR UPDATE OF page_snapshot_id OR DELETE ON listings
  FOR EACH ROW
  EXECUTE FUNCTION update_page_snapshot_listings_count();

-- ============================================================================
-- PART 4: Add domain-level statistics view
-- ============================================================================

-- View for quick domain statistics (used by admin UI)
CREATE OR REPLACE VIEW domain_statistics AS
SELECT
  d.id as domain_id,
  d.domain_url,
  d.status as domain_status,
  COUNT(DISTINCT ds.id) as total_page_urls,
  COUNT(DISTINCT ds.id) FILTER (WHERE ds.scrape_status = 'scraped') as scraped_pages,
  COUNT(DISTINCT ds.id) FILTER (WHERE ds.scrape_status = 'pending') as pending_pages,
  COUNT(DISTINCT ds.id) FILTER (WHERE ds.scrape_status = 'failed') as failed_pages,
  COUNT(DISTINCT ps.id) as total_snapshots,
  COUNT(DISTINCT l.id) as total_listings,
  COUNT(DISTINCT l.id) FILTER (WHERE l.status = 'active') as active_listings,
  COUNT(DISTINCT l.id) FILTER (WHERE l.status = 'pending_approval') as pending_listings,
  MAX(ds.last_scraped_at) as last_scraped_at,
  d.created_at as domain_created_at
FROM domains d
LEFT JOIN domain_snapshots ds ON ds.domain_id = d.id
LEFT JOIN page_snapshots ps ON ps.id = ds.page_snapshot_id
LEFT JOIN listings l ON l.domain_id = d.id
GROUP BY d.id, d.domain_url, d.status, d.created_at;

-- Index the view for performance
CREATE INDEX IF NOT EXISTS idx_domain_stats_status
  ON domains(status, created_at DESC);

-- ============================================================================
-- PART 5: Add page snapshot detail view
-- ============================================================================

-- View for page snapshot details with listing counts
CREATE OR REPLACE VIEW page_snapshot_details AS
SELECT
  ps.id as snapshot_id,
  ps.url,
  ps.content_hash,
  ps.crawled_at,
  ps.fetched_via,
  ps.listings_extracted_count,
  ps.extraction_status,
  ps.extraction_completed_at,
  ds.id as domain_snapshot_id,
  ds.domain_id,
  ds.page_url as submitted_page_url,
  ds.scrape_status,
  ds.last_scraped_at,
  ds.submitted_at,
  d.domain_url,
  d.status as domain_status,
  COUNT(l.id) as actual_listings_count
FROM page_snapshots ps
LEFT JOIN domain_snapshots ds ON ds.page_snapshot_id = ps.id
LEFT JOIN domains d ON d.id = ds.domain_id
LEFT JOIN listings l ON l.page_snapshot_id = ps.id
GROUP BY
  ps.id, ps.url, ps.content_hash, ps.crawled_at, ps.fetched_via,
  ps.listings_extracted_count, ps.extraction_status, ps.extraction_completed_at,
  ds.id, ds.domain_id, ds.page_url, ds.scrape_status,
  ds.last_scraped_at, ds.submitted_at,
  d.domain_url, d.status;

-- ============================================================================
-- PART 6: Add helper function for querying listings by page
-- ============================================================================

-- Function to get all listings from a specific page URL within a domain
CREATE OR REPLACE FUNCTION get_listings_by_domain_page(
  p_domain_id UUID,
  p_page_url TEXT
) RETURNS TABLE (
  listing_id UUID,
  title TEXT,
  description TEXT,
  status TEXT,
  source_url TEXT,
  page_snapshot_id UUID,
  created_at TIMESTAMPTZ
) AS $$
BEGIN
  RETURN QUERY
  SELECT
    l.id,
    l.title,
    l.description,
    l.status,
    l.source_url,
    l.page_snapshot_id,
    l.created_at
  FROM listings l
  WHERE l.domain_id = p_domain_id
    AND l.source_url = p_page_url
  ORDER BY l.created_at DESC;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- COMMENTS for documentation
-- ============================================================================

COMMENT ON COLUMN listings.page_snapshot_id IS
  'Links to the cached page content (HTML/markdown) that this listing was extracted from. Enables viewing original source.';

COMMENT ON COLUMN page_snapshots.listings_extracted_count IS
  'Cached count of listings extracted from this page snapshot. Updated automatically via trigger.';

COMMENT ON VIEW domain_statistics IS
  'Aggregated statistics for each domain showing page counts, listing counts, and scrape status. Used by admin dashboard.';

COMMENT ON VIEW page_snapshot_details IS
  'Detailed view of page snapshots with their domain context and listing counts. Used by admin UI for traceability.';

COMMENT ON FUNCTION get_listings_by_domain_page IS
  'Helper function to retrieve all listings extracted from a specific page URL within a domain. Used by admin UI for drill-down.';
