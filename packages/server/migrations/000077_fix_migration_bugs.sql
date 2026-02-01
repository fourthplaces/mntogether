-- Fix bugs in previous migrations that reference non-existent columns
--
-- Migration 57 tried to create an index on website_snapshots.snapshot_hash
-- but this column never existed. We drop the index creation attempt (it will
-- fail silently with IF EXISTS).
--
-- Migration 66 referenced 'domain' column but at that point the column was
-- still called 'url' (renamed in migration 68). We handle this by checking
-- if the normalization needs to be applied to whichever column exists.

-- Fix 1: Drop the invalid index from migration 57 (if it somehow exists)
DROP INDEX IF EXISTS idx_website_snapshots_snapshot_hash;

-- Fix 2: Ensure website URLs/domains are normalized
-- At this point the column is called 'domain' (after migration 68)
-- Re-run the normalization that migration 66 was supposed to do
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
