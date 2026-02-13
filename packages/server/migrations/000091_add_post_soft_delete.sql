-- Add soft delete fields to posts table
-- deleted_at: when the post was soft-deleted
-- deleted_reason: why it was deleted (e.g., "Duplicate of post <uuid>")

ALTER TABLE posts ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMP WITH TIME ZONE;
ALTER TABLE posts ADD COLUMN IF NOT EXISTS deleted_reason TEXT;

-- Index for efficient filtering of non-deleted posts
CREATE INDEX IF NOT EXISTS idx_posts_deleted_at ON posts (deleted_at) WHERE deleted_at IS NULL;

-- Update active posts views/queries to filter out soft-deleted posts
COMMENT ON COLUMN posts.deleted_at IS 'Soft delete timestamp - post is hidden but preserved for link continuity';
COMMENT ON COLUMN posts.deleted_reason IS 'Reason for deletion, e.g. "Duplicate of post <uuid>" or "Merged into <uuid>"';
