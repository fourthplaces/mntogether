-- Organization needs (volunteer opportunities extracted from websites)

CREATE TABLE organization_needs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Organization info
    organization_name TEXT NOT NULL,

    -- Content (AI-extracted)
    title TEXT NOT NULL,
    description TEXT NOT NULL,           -- Plain text for search
    description_markdown TEXT,           -- Markdown for display
    tldr TEXT,                           -- Short summary (1-2 sentences)

    -- Contact
    contact_info JSONB,                  -- { phone, email, website }

    -- Metadata
    urgency TEXT,                        -- 'urgent' | 'normal' | 'low'
    status TEXT DEFAULT 'pending_approval', -- 'pending_approval' | 'active' | 'rejected' | 'expired'
    content_hash TEXT,                   -- SHA256 for deduplication

    -- Sync tracking
    source_id UUID REFERENCES organization_sources(id) ON DELETE SET NULL,
    last_seen_at TIMESTAMPTZ DEFAULT NOW(),
    disappeared_at TIMESTAMPTZ,          -- When need was no longer found on website

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index for querying by status (most common query: status='active')
CREATE INDEX idx_organization_needs_status
    ON organization_needs(status);

-- Index for content hash lookup (deduplication)
CREATE INDEX idx_organization_needs_content_hash
    ON organization_needs(content_hash);

-- Index for finding active needs from a source
CREATE INDEX idx_organization_needs_source_id
    ON organization_needs(source_id)
    WHERE status = 'active';

-- Index for sync operations (find needs last seen before a certain time)
CREATE INDEX idx_organization_needs_last_seen
    ON organization_needs(last_seen_at, source_id);
