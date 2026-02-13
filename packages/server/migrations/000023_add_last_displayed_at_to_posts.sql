-- Add last_displayed_at column to posts table for engagement-based rotation
--
-- Purpose: Track when each post was last displayed to users, enabling fair
-- rotation of posts so that older, less-viewed posts get visibility.
--
-- Rotation Algorithm:
-- Posts are sorted by:
-- 1. view_count ASC (posts with fewer views shown first)
-- 2. last_displayed_at ASC NULLS FIRST (posts never shown or shown longest ago)
--
-- This ensures:
-- - New posts get initial visibility
-- - Under-engaged posts resurface periodically
-- - All posts get fair exposure over time

ALTER TABLE posts
ADD COLUMN last_displayed_at TIMESTAMP WITH TIME ZONE;

-- Create index for efficient sorting by rotation algorithm
CREATE INDEX idx_posts_rotation ON posts(view_count ASC, last_displayed_at ASC NULLS FIRST)
WHERE status = 'published';

-- Add comment explaining the column's purpose
COMMENT ON COLUMN posts.last_displayed_at IS 'Timestamp when this post was last fetched in a published posts query. Used for round-robin engagement tracking to ensure all posts get visibility.';
