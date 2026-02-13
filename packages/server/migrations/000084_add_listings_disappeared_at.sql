-- Add disappeared_at column back to listings table
-- This column was removed in 000057 but is still used by sync logic

ALTER TABLE listings ADD COLUMN IF NOT EXISTS disappeared_at TIMESTAMPTZ;

-- Index for efficient filtering of active listings
CREATE INDEX IF NOT EXISTS idx_listings_disappeared_at
ON listings(disappeared_at)
WHERE disappeared_at IS NULL;
