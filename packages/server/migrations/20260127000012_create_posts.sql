-- Posts - curated announcements created when needs are approved
-- Separates REALITY (needs) from ANNOUNCEMENTS (posts)
--
-- Key concept:
-- - Needs = persistent state of the world (what organizations need)
-- - Posts = temporal announcements about that state (what we tell volunteers)
--
-- Benefits:
-- - One need can have multiple posts over time (re-posting unfilled needs)
-- - Posts can be customized without changing the underlying need
-- - Posts have temporal bounds (published_at, expires_at)
-- - Clean event tracking (post is the unit of notification)

CREATE TYPE post_status AS ENUM ('draft', 'published', 'expired', 'archived');

CREATE TABLE posts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Link to the underlying need
    need_id UUID NOT NULL REFERENCES organization_needs(id) ON DELETE CASCADE,

    -- Status lifecycle
    status post_status NOT NULL DEFAULT 'draft',

    -- Temporal bounds (posts are time-limited announcements)
    published_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,

    -- Admin customizations (override need content if needed)
    custom_title TEXT,                -- Override need title
    custom_description TEXT,          -- Add context or urgency
    custom_tldr TEXT,                 -- Custom summary

    -- Targeting hints (for future relevance matching)
    targeting_hints JSONB,            -- { tags: [...], locations: [...], skills: [...] }

    -- Engagement metrics (simple counts, no detailed tracking)
    view_count INTEGER DEFAULT 0,
    click_count INTEGER DEFAULT 0,
    response_count INTEGER DEFAULT 0,

    -- Audit
    created_by UUID,                  -- Admin who created/approved
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for finding published posts (most common query)
CREATE INDEX idx_posts_status ON posts(status);

-- Index for finding posts by need
CREATE INDEX idx_posts_need_id ON posts(need_id);

-- Index for finding posts that need expiration
CREATE INDEX idx_posts_expires_at
    ON posts(expires_at)
    WHERE status = 'published';

-- Index for finding recent posts
CREATE INDEX idx_posts_published_at
    ON posts(published_at DESC)
    WHERE status = 'published';

-- Trigger to update updated_at
CREATE OR REPLACE FUNCTION update_posts_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_posts_updated_at
    BEFORE UPDATE ON posts
    FOR EACH ROW
    EXECUTE FUNCTION update_posts_updated_at();

COMMENT ON TABLE posts IS
'Temporal announcements created when needs are approved. One need can have multiple posts over time.';

COMMENT ON COLUMN posts.custom_title IS
'Admin can override need title to add urgency or context (e.g., "URGENT: Food Bank Needs 10 Volunteers This Weekend")';

COMMENT ON COLUMN posts.targeting_hints IS
'Hints for future relevance matching (e.g., {"tags": ["spanish_speaking"], "locations": ["minneapolis"]})';

COMMENT ON COLUMN posts.expires_at IS
'Posts expire after a configured period (e.g., 5 days). Expired posts can be re-posted if need still exists.';
