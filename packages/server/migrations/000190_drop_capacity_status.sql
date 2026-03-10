-- Drop unused capacity_status column from posts.
ALTER TABLE posts DROP COLUMN IF EXISTS capacity_status;
