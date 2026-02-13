-- Add deduplication fields for listing extraction
-- Prevents duplicate listings when re-crawling websites

-- Add normalized title for deduplication matching
-- Strips to lowercase alphanumeric for fuzzy matching
ALTER TABLE listings ADD COLUMN IF NOT EXISTS title_normalized TEXT
    GENERATED ALWAYS AS (
        lower(regexp_replace(title, '[^a-z0-9]', '', 'gi'))
    ) STORED;

-- Unique constraint per website (prevents duplicates)
CREATE UNIQUE INDEX IF NOT EXISTS idx_listings_website_title_normalized
    ON listings(website_id, title_normalized)
    WHERE website_id IS NOT NULL AND title_normalized IS NOT NULL;

-- Track when listings were last extracted for this website
ALTER TABLE websites ADD COLUMN IF NOT EXISTS listings_extracted_at TIMESTAMPTZ;

-- Index for checking if re-extraction needed
CREATE INDEX IF NOT EXISTS idx_websites_listings_extracted_at
    ON websites(listings_extracted_at)
    WHERE listings_extracted_at IS NOT NULL;
