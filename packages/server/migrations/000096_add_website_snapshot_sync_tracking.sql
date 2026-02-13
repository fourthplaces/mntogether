-- Migration: Add sync tracking to website_snapshots
--
-- Adds last_synced_at column to track when the extraction library last
-- synced content for this URL. This supports the new Ingestor pattern where
-- the extraction library owns page storage (extraction_pages table).
--
-- Note: page_snapshot_id is kept for backward compatibility with existing
-- crawl code. It can be dropped once all crawling code uses the extraction library.

-- Add last_synced_at column for tracking extraction library sync
ALTER TABLE website_snapshots
    ADD COLUMN IF NOT EXISTS last_synced_at TIMESTAMPTZ;

-- Index for finding stale snapshots that need re-syncing
CREATE INDEX IF NOT EXISTS idx_website_snapshots_last_synced_at
    ON website_snapshots(last_synced_at)
    WHERE last_synced_at IS NOT NULL;

COMMENT ON COLUMN website_snapshots.last_synced_at IS
    'When the extraction library last synced content for this URL';
