-- Fix migration 88 checksum
-- Delete the old migration record so it can be re-applied
DELETE FROM _sqlx_migrations WHERE version = 88;

-- Now manually apply migration 88 content
ALTER TABLE posts RENAME COLUMN listing_type TO post_type;

-- Update the check constraint
ALTER TABLE posts DROP CONSTRAINT IF EXISTS posts_listing_type_check;
ALTER TABLE posts DROP CONSTRAINT IF EXISTS listings_listing_type_check;
ALTER TABLE posts ADD CONSTRAINT posts_post_type_check
  CHECK (post_type IN ('service', 'opportunity', 'business', 'professional'));

-- Rename the index
ALTER INDEX IF EXISTS idx_listings_type RENAME TO idx_posts_type;
