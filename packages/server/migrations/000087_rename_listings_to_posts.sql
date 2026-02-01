-- Rename listings table to posts
-- This is a comprehensive rename of the listings concept to posts

-- Step 0: Drop old posts table from migration 13 (posts were replaced by listings)
-- The old posts table had a different structure and is no longer used
DROP TABLE IF EXISTS posts CASCADE;
DROP TYPE IF EXISTS post_status CASCADE;

-- Step 1: Rename the main table
ALTER TABLE listings RENAME TO posts;

-- Step 2: Rename sequences (if any auto-generated)
-- PostgreSQL auto-renames sequences when table is renamed

-- Step 3: Rename foreign key constraints to reflect new name
-- Drop and recreate constraints with new names

-- Rename listing_id column references in other tables to post_id
ALTER TABLE listing_page_sources RENAME COLUMN listing_id TO post_id;
ALTER TABLE listing_page_sources RENAME TO post_page_sources;

-- Rename the listing_tags view/table references
-- (tags table uses entity_id which is generic, no change needed)

-- Step 4: Rename indexes
ALTER INDEX IF EXISTS listings_pkey RENAME TO posts_pkey;
ALTER INDEX IF EXISTS listings_content_hash_idx RENAME TO posts_content_hash_idx;
ALTER INDEX IF EXISTS listings_website_id_idx RENAME TO posts_website_id_idx;
ALTER INDEX IF EXISTS listings_status_idx RENAME TO posts_status_idx;
ALTER INDEX IF EXISTS listings_embedding_idx RENAME TO posts_embedding_idx;
ALTER INDEX IF EXISTS idx_listings_content_hash_unique RENAME TO idx_posts_content_hash_unique;

-- Step 5: Update any CHECK constraints or triggers that reference listings
-- (none identified that need changing)

-- Note: The Rust code, GraphQL schema, and frontend will be updated separately
-- to use Post/PostId instead of Listing/ListingId
