-- Unified Sources: Class Table Inheritance
-- Replaces separate `websites` and `social_profiles` tables with:
--   sources (parent) + website_sources + social_sources
-- Reuses existing IDs from websites and social_profiles as sources.id
-- so that post_sources.source_id and other polymorphic references remain valid.

-- =============================================================================
-- STEP 1: Create new tables
-- =============================================================================

CREATE TABLE sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_type TEXT NOT NULL,  -- 'website', 'instagram', 'facebook', 'tiktok'
    url TEXT,
    organization_id UUID REFERENCES organizations(id) ON DELETE SET NULL,
    status TEXT NOT NULL DEFAULT 'pending_review',
    active BOOLEAN NOT NULL DEFAULT true,
    scrape_frequency_hours INT NOT NULL DEFAULT 24,
    last_scraped_at TIMESTAMPTZ,
    submitted_by UUID,
    submitter_type TEXT,
    submission_context TEXT,
    reviewed_by UUID,
    reviewed_at TIMESTAMPTZ,
    rejection_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE website_sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL UNIQUE REFERENCES sources(id) ON DELETE CASCADE,
    domain TEXT NOT NULL UNIQUE,
    max_crawl_depth INT NOT NULL DEFAULT 2,
    crawl_rate_limit_seconds INT NOT NULL DEFAULT 5,
    is_trusted BOOLEAN NOT NULL DEFAULT false
);

CREATE TABLE social_sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL UNIQUE REFERENCES sources(id) ON DELETE CASCADE,
    source_type TEXT NOT NULL,  -- denormalized for UNIQUE constraint
    handle TEXT NOT NULL,
    UNIQUE (source_type, handle)
);

-- =============================================================================
-- STEP 2: Migrate data from websites → sources + website_sources
-- =============================================================================

INSERT INTO sources (id, source_type, url, organization_id, status, active,
    scrape_frequency_hours, last_scraped_at, submitted_by, submitter_type,
    submission_context, reviewed_by, reviewed_at, rejection_reason,
    created_at, updated_at)
SELECT
    id,
    'website',
    'https://' || domain,
    organization_id,
    status,
    active,
    scrape_frequency_hours,
    last_scraped_at,
    submitted_by,
    submitter_type,
    submission_context,
    reviewed_by,
    reviewed_at,
    rejection_reason,
    created_at,
    updated_at
FROM websites;

INSERT INTO website_sources (source_id, domain, max_crawl_depth, crawl_rate_limit_seconds, is_trusted)
SELECT id, domain, max_crawl_depth, crawl_rate_limit_seconds, is_trusted
FROM websites;

-- =============================================================================
-- STEP 3: Migrate data from social_profiles → sources + social_sources
-- =============================================================================

INSERT INTO sources (id, source_type, url, organization_id, status, active,
    scrape_frequency_hours, last_scraped_at, submitted_by, submitter_type,
    submission_context, reviewed_by, reviewed_at, rejection_reason,
    created_at, updated_at)
SELECT
    id,
    platform,           -- 'instagram', 'facebook', etc. becomes source_type
    url,                -- profile URL (nullable)
    organization_id,
    'approved',         -- social profiles have no approval workflow currently; default to approved
    active,
    scrape_frequency_hours,
    last_scraped_at,
    NULL,               -- no submitted_by for social profiles
    NULL,               -- no submitter_type
    NULL,               -- no submission_context
    NULL,               -- no reviewed_by
    NULL,               -- no reviewed_at
    NULL,               -- no rejection_reason
    created_at,
    updated_at
FROM social_profiles;

INSERT INTO social_sources (source_id, source_type, handle)
SELECT id, platform, handle
FROM social_profiles;

-- =============================================================================
-- STEP 4: Drop domain_statistics view (depends on websites table)
-- =============================================================================

DROP VIEW IF EXISTS domain_statistics;
DROP VIEW IF EXISTS page_snapshot_details;

-- =============================================================================
-- STEP 5: Re-point foreign keys on dependent tables
-- =============================================================================

-- website_snapshots: FK from websites → website_sources
ALTER TABLE website_snapshots
    DROP CONSTRAINT IF EXISTS website_snapshots_website_id_fkey;
ALTER TABLE website_snapshots
    ADD CONSTRAINT website_snapshots_source_id_fkey
    FOREIGN KEY (website_id) REFERENCES website_sources(source_id) ON DELETE CASCADE;

-- website_assessments: FK from websites → website_sources
ALTER TABLE website_assessments
    DROP CONSTRAINT IF EXISTS website_assessments_website_id_fkey;
ALTER TABLE website_assessments
    ADD CONSTRAINT website_assessments_source_id_fkey
    FOREIGN KEY (website_id) REFERENCES website_sources(source_id) ON DELETE CASCADE;

-- website_research: FK from websites → website_sources
ALTER TABLE website_research
    DROP CONSTRAINT IF EXISTS website_research_website_id_fkey;
ALTER TABLE website_research
    ADD CONSTRAINT website_research_source_id_fkey
    FOREIGN KEY (website_id) REFERENCES website_sources(source_id) ON DELETE CASCADE;

-- providers: rename website_id → source_id, FK to sources
ALTER TABLE providers RENAME COLUMN website_id TO source_id;
ALTER TABLE providers
    DROP CONSTRAINT IF EXISTS providers_website_id_fkey;
ALTER TABLE providers
    ADD CONSTRAINT providers_source_id_fkey
    FOREIGN KEY (source_id) REFERENCES sources(id) ON DELETE SET NULL;

-- organizations: drop website_id FK if it exists (orgs now link via sources.organization_id)
ALTER TABLE organizations
    DROP CONSTRAINT IF EXISTS organizations_website_id_fkey;
DROP INDEX IF EXISTS idx_organizations_website;
ALTER TABLE organizations DROP COLUMN IF EXISTS website_id;

-- posts: drop social_profile_id FK if it exists (posts now link via post_sources)
ALTER TABLE posts
    DROP CONSTRAINT IF EXISTS posts_social_profile_id_fkey;
DROP INDEX IF EXISTS idx_posts_social_profile_id;
ALTER TABLE posts DROP COLUMN IF EXISTS social_profile_id;

-- listing_website_sync: legacy sync table superseded by post_sources
DROP TABLE IF EXISTS listing_website_sync;

-- =============================================================================
-- STEP 6: Recreate domain_statistics view with new tables
-- =============================================================================

CREATE VIEW domain_statistics AS
SELECT s.id AS domain_id,
    ws.domain AS domain_url,
    s.status AS domain_status,
    count(DISTINCT ds.id) AS total_page_urls,
    count(DISTINCT ds.id) FILTER (WHERE ds.scrape_status = 'scraped') AS scraped_pages,
    count(DISTINCT ds.id) FILTER (WHERE ds.scrape_status = 'pending') AS pending_pages,
    count(DISTINCT ds.id) FILTER (WHERE ds.scrape_status = 'failed') AS failed_pages,
    count(DISTINCT ps2.id) AS total_snapshots,
    count(DISTINCT l.id) AS total_listings,
    count(DISTINCT l.id) FILTER (WHERE l.status = 'active') AS active_listings,
    count(DISTINCT l.id) FILTER (WHERE l.status = 'pending_approval') AS pending_listings,
    max(ds.last_scraped_at) AS last_scraped_at,
    s.created_at AS domain_created_at
FROM sources s
    JOIN website_sources ws ON ws.source_id = s.id
    LEFT JOIN website_snapshots ds ON ds.website_id = s.id
    LEFT JOIN page_snapshots ps2 ON ps2.id = ds.page_snapshot_id
    LEFT JOIN post_sources src ON src.source_type = 'website' AND src.source_id = s.id
    LEFT JOIN posts l ON l.id = src.post_id
WHERE s.source_type = 'website'
GROUP BY s.id, ws.domain, s.status, s.created_at;

CREATE VIEW page_snapshot_details AS
SELECT ps.id AS snapshot_id,
    ps.url,
    ps.content_hash,
    ps.crawled_at,
    ps.fetched_via,
    ps.listings_extracted_count,
    ps.extraction_status,
    ps.extraction_completed_at,
    ds.id AS domain_snapshot_id,
    ds.website_id AS domain_id,
    ds.page_url AS submitted_page_url,
    ds.scrape_status,
    ds.last_scraped_at,
    ds.submitted_at,
    ws.domain AS domain_url,
    s.status AS domain_status,
    count(l.id) AS actual_listings_count
FROM page_snapshots ps
    LEFT JOIN website_snapshots ds ON ds.page_snapshot_id = ps.id
    LEFT JOIN sources s ON s.id = ds.website_id
    LEFT JOIN website_sources ws ON ws.source_id = s.id
    LEFT JOIN posts l ON l.page_snapshot_id = ps.id
GROUP BY ps.id, ps.url, ps.content_hash, ps.crawled_at, ps.fetched_via,
    ps.listings_extracted_count, ps.extraction_status, ps.extraction_completed_at,
    ds.id, ds.website_id, ds.page_url, ds.scrape_status, ds.last_scraped_at,
    ds.submitted_at, ws.domain, s.status;

-- =============================================================================
-- STEP 7: Drop old tables
-- =============================================================================

DROP TABLE social_profiles;
DROP TABLE websites;

-- =============================================================================
-- STEP 8: Add indexes on new tables
-- =============================================================================

CREATE INDEX idx_sources_source_type ON sources(source_type);
CREATE INDEX idx_sources_organization_id ON sources(organization_id);
CREATE INDEX idx_sources_status ON sources(status);
CREATE INDEX idx_sources_active_due ON sources(active, status, last_scraped_at)
    WHERE active = true AND status = 'approved';
CREATE INDEX idx_sources_created_at ON sources(created_at);

CREATE INDEX idx_website_sources_source_id ON website_sources(source_id);
CREATE INDEX idx_website_sources_domain ON website_sources(domain);

CREATE INDEX idx_social_sources_source_id ON social_sources(source_id);
CREATE INDEX idx_social_sources_handle ON social_sources(source_type, handle);
