-- Normalize website URLs to just domain (lowercase, no www prefix, no protocol)
-- This ensures consistent deduplication and matching

-- Update existing URLs to normalized format
-- Extract domain: remove protocol, remove www., lowercase
UPDATE websites
SET url = LOWER(
    REGEXP_REPLACE(
        REGEXP_REPLACE(
            REGEXP_REPLACE(url, '^https?://', ''),  -- Remove protocol
            '^www\.', ''                             -- Remove www. prefix
        ),
        '/.*$', ''                                   -- Remove path
    )
)
WHERE url IS NOT NULL;

-- Handle any duplicates that may have been created by normalization
-- Keep the oldest record (first created)
DELETE FROM websites a
USING websites b
WHERE a.url = b.url
  AND a.id != b.id
  AND a.created_at > b.created_at;
