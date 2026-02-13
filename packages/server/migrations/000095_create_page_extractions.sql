-- Generic table for AI-extracted content from page snapshots
-- Supports multiple extraction types (summary, posts, contacts, hours, etc.)
-- with versioning and model tracking

CREATE TABLE page_extractions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    page_snapshot_id UUID NOT NULL REFERENCES page_snapshots(id) ON DELETE CASCADE,
    extraction_type TEXT NOT NULL,      -- 'summary', 'posts', 'contacts', 'hours', etc.
    content JSONB NOT NULL,             -- Structure varies by extraction_type
    model TEXT,                         -- 'gpt-4o', 'claude-3-5-sonnet', etc.
    prompt_version TEXT,                -- Track prompt iterations for A/B testing
    tokens_used INTEGER,                -- Track token usage for cost analysis
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    is_current BOOLEAN NOT NULL DEFAULT TRUE
);

-- Fast lookup for current extraction by type
CREATE INDEX idx_page_extractions_current
    ON page_extractions(page_snapshot_id, extraction_type)
    WHERE is_current = TRUE;

-- For finding all extractions of a type (e.g., all summaries)
CREATE INDEX idx_page_extractions_type
    ON page_extractions(extraction_type, created_at DESC);

-- Ensure only one current extraction per page/type combo
CREATE UNIQUE INDEX idx_page_extractions_unique_current
    ON page_extractions(page_snapshot_id, extraction_type)
    WHERE is_current = TRUE;

COMMENT ON TABLE page_extractions IS 'AI-extracted content from page snapshots, versioned by type';
COMMENT ON COLUMN page_extractions.extraction_type IS 'Type of extraction: summary, posts, contacts, hours, events, etc.';
COMMENT ON COLUMN page_extractions.content IS 'JSON content structure varies by extraction_type';
COMMENT ON COLUMN page_extractions.is_current IS 'Only one extraction per page/type can be current';
