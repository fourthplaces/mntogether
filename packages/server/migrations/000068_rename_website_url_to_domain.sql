-- Rename websites.url to websites.domain for semantic correctness
-- The column stores domain names (e.g., "dhhmn.com"), not full URLs
-- This migration is idempotent - only renames if 'url' column exists

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'websites' AND column_name = 'url'
    ) THEN
        ALTER TABLE websites RENAME COLUMN url TO domain;
    END IF;
END $$;
