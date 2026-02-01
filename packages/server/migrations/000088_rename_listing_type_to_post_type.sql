-- Rename listing_type column to post_type
-- This completes the listings -> posts rename

-- Rename the column in posts table (idempotent - only if column exists)
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'posts' AND column_name = 'listing_type'
    ) THEN
        ALTER TABLE posts RENAME COLUMN listing_type TO post_type;
    END IF;
END $$;

-- Update the check constraint
ALTER TABLE posts DROP CONSTRAINT IF EXISTS posts_listing_type_check;
ALTER TABLE posts DROP CONSTRAINT IF EXISTS listings_listing_type_check;
ALTER TABLE posts DROP CONSTRAINT IF EXISTS posts_post_type_check;
ALTER TABLE posts ADD CONSTRAINT posts_post_type_check
  CHECK (post_type IN ('service', 'opportunity', 'business', 'professional'));

-- Rename the index (idempotent)
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_indexes
        WHERE indexname = 'idx_listings_type'
    ) THEN
        ALTER INDEX idx_listings_type RENAME TO idx_posts_type;
    END IF;
END $$;
