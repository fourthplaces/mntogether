-- Unified post source tracking table
-- Replaces posts.website_id, posts.social_profile_id, and post_website_sync

CREATE TABLE post_sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    source_type TEXT NOT NULL,       -- 'website' | 'instagram' | 'facebook' | 'x'
    source_id UUID NOT NULL,         -- websites.id or social_profiles.id (polymorphic)
    source_url TEXT,
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    disappeared_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (post_id, source_type, source_id)
);

CREATE INDEX idx_post_sources_source ON post_sources(source_type, source_id);
CREATE INDEX idx_post_sources_post_id ON post_sources(post_id);
CREATE INDEX idx_post_sources_active ON post_sources(source_type, source_id) WHERE disappeared_at IS NULL;

-- Migrate post_website_sync rows
INSERT INTO post_sources (post_id, source_type, source_id, source_url,
    first_seen_at, last_seen_at, disappeared_at, created_at, updated_at)
SELECT pws.post_id, 'website', pws.website_id, pws.source_url,
    pws.first_seen_at, pws.last_seen_at, pws.disappeared_at, pws.created_at, pws.updated_at
FROM post_website_sync pws;

-- Migrate posts.website_id where no sync row exists
INSERT INTO post_sources (post_id, source_type, source_id, source_url,
    first_seen_at, last_seen_at, created_at, updated_at)
SELECT p.id, 'website', p.website_id, p.source_url,
    p.created_at, p.updated_at, p.created_at, p.updated_at
FROM posts p
WHERE p.website_id IS NOT NULL
  AND NOT EXISTS (
    SELECT 1 FROM post_sources ps
    WHERE ps.post_id = p.id AND ps.source_type = 'website' AND ps.source_id = p.website_id
  );

-- Migrate posts.social_profile_id (using platform from social_profiles)
INSERT INTO post_sources (post_id, source_type, source_id, source_url,
    first_seen_at, last_seen_at, created_at, updated_at)
SELECT p.id, sp.platform, p.social_profile_id, p.source_url,
    p.created_at, p.updated_at, p.created_at, p.updated_at
FROM posts p
JOIN social_profiles sp ON sp.id = p.social_profile_id
WHERE p.social_profile_id IS NOT NULL;

-- Drop dependent view that references posts.website_id
DROP VIEW IF EXISTS domain_statistics;

-- Drop dependent indexes on posts.website_id and posts.social_profile_id
DROP INDEX IF EXISTS idx_listings_website_id;
DROP INDEX IF EXISTS idx_listings_domain_source_url;
DROP INDEX IF EXISTS idx_listings_website_title_normalized;
DROP INDEX IF EXISTS idx_posts_social_profile_id;

-- Drop dependent FK constraints
ALTER TABLE posts DROP CONSTRAINT IF EXISTS organization_needs_domain_id_fkey;
ALTER TABLE posts DROP CONSTRAINT IF EXISTS listings_website_id_fkey;
ALTER TABLE posts DROP CONSTRAINT IF EXISTS posts_social_profile_id_fkey;

-- Drop old columns
ALTER TABLE posts DROP COLUMN website_id;
ALTER TABLE posts DROP COLUMN social_profile_id;

-- Drop old table
DROP TABLE post_website_sync;

-- Recreate domain_statistics view using post_sources instead of posts.website_id
CREATE VIEW domain_statistics AS
SELECT d.id AS domain_id,
    d.domain AS domain_url,
    d.status AS domain_status,
    count(DISTINCT ds.id) AS total_page_urls,
    count(DISTINCT ds.id) FILTER (WHERE ds.scrape_status = 'scraped') AS scraped_pages,
    count(DISTINCT ds.id) FILTER (WHERE ds.scrape_status = 'pending') AS pending_pages,
    count(DISTINCT ds.id) FILTER (WHERE ds.scrape_status = 'failed') AS failed_pages,
    count(DISTINCT ps2.id) AS total_snapshots,
    count(DISTINCT l.id) AS total_listings,
    count(DISTINCT l.id) FILTER (WHERE l.status = 'active') AS active_listings,
    count(DISTINCT l.id) FILTER (WHERE l.status = 'pending_approval') AS pending_listings,
    max(ds.last_scraped_at) AS last_scraped_at,
    d.created_at AS domain_created_at
FROM websites d
    LEFT JOIN website_snapshots ds ON ds.website_id = d.id
    LEFT JOIN page_snapshots ps2 ON ps2.id = ds.page_snapshot_id
    LEFT JOIN post_sources src ON src.source_type = 'website' AND src.source_id = d.id
    LEFT JOIN posts l ON l.id = src.post_id
GROUP BY d.id, d.domain, d.status, d.created_at;
