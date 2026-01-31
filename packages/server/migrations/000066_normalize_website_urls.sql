-- Normalize website domains to lowercase, no www prefix
-- This ensures consistent deduplication and matching
-- Note: Column was renamed from 'url' to 'domain' in migration 68

-- Update existing domains to normalized format (idempotent)
UPDATE websites
SET domain = LOWER(
    REGEXP_REPLACE(
        REGEXP_REPLACE(
            REGEXP_REPLACE(domain, '^https?://', ''),  -- Remove protocol
            '^www\.', ''                                -- Remove www. prefix
        ),
        '/.*$', ''                                      -- Remove path
    )
)
WHERE domain IS NOT NULL
  AND domain ~ '(^https?://|^www\.|/)';  -- Only update if not already normalized

-- Handle any duplicates that may have been created by normalization
-- Keep the oldest record (first created)
DELETE FROM websites a
USING websites b
WHERE a.domain = b.domain
  AND a.id != b.id
  AND a.created_at > b.created_at;
