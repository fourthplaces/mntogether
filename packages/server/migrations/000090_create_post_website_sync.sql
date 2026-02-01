-- Create post_website_sync table for tracking when posts are seen on websites
-- This decouples sync tracking from the Post model, enabling:
-- - Posts to exist independently of any website
-- - Tracking multiple appearances of the same post across different websites
-- - Content hash-based duplicate detection per website
-- - Temporal tracking (first_seen, last_seen, disappeared_at)

CREATE TABLE IF NOT EXISTS post_website_sync (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    website_id UUID NOT NULL REFERENCES websites(id) ON DELETE CASCADE,
    content_hash TEXT NOT NULL,
    source_url TEXT NOT NULL,
    first_seen_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    disappeared_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    -- Unique constraint for the upsert operation
    UNIQUE (post_id, website_id)
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_post_website_sync_website_id ON post_website_sync(website_id);
CREATE INDEX IF NOT EXISTS idx_post_website_sync_post_id ON post_website_sync(post_id);
CREATE INDEX IF NOT EXISTS idx_post_website_sync_content_hash ON post_website_sync(website_id, content_hash);
CREATE INDEX IF NOT EXISTS idx_post_website_sync_active ON post_website_sync(website_id) WHERE disappeared_at IS NULL;
