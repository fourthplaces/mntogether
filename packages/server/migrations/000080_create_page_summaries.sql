-- Create page_summaries table for caching AI-extracted page content
-- Summaries are linked to page_snapshots via content_hash for cache invalidation

CREATE TABLE page_summaries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    page_snapshot_id UUID NOT NULL REFERENCES page_snapshots(id) ON DELETE CASCADE,
    content_hash TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- One summary per unique content (cache key)
    UNIQUE(content_hash)
);

-- Index for fast cache lookups
CREATE INDEX idx_page_summaries_content_hash ON page_summaries(content_hash);

-- Index for finding summaries by snapshot
CREATE INDEX idx_page_summaries_snapshot_id ON page_summaries(page_snapshot_id);
