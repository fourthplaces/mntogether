-- Migration 000042: Rename posts.need_id to posts.listing_id
-- This completes the migration from organization_needs table to listings table

-- Rename the column
ALTER TABLE posts RENAME COLUMN need_id TO listing_id;

-- Drop old foreign key constraint
ALTER TABLE posts DROP CONSTRAINT IF EXISTS posts_need_id_fkey;

-- Add new foreign key constraint pointing to listings table
ALTER TABLE posts
  ADD CONSTRAINT posts_listing_id_fkey
    FOREIGN KEY (listing_id) REFERENCES listings(id) ON DELETE CASCADE;

-- Drop old index
DROP INDEX IF EXISTS idx_posts_need_id;

-- Create new index
CREATE INDEX idx_posts_listing_id ON posts(listing_id);

-- Add comment
COMMENT ON COLUMN posts.listing_id IS 'Foreign key reference to listings table (was need_id)';
