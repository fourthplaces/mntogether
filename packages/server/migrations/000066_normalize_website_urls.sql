-- Normalize website URLs to lowercase, no www prefix
-- This ensures consistent deduplication and matching
-- Note: Column is still 'url' at this point (renamed to 'domain' in migration 68)

-- Update existing URLs to normalized format (idempotent)
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
WHERE url IS NOT NULL
  AND url ~ '(^https?://|^www\.|/)';  -- Only update if not already normalized

-- Handle any duplicates that may have been created by normalization
-- Keep the oldest record (first created)
DELETE FROM websites a
USING websites b
WHERE a.url = b.url
  AND a.id != b.id
  AND a.created_at > b.created_at;
