-- Migration: Rename domains to websites and decouple sync logic
-- This migration:
-- 1. Renames all domain-related tables to website-related tables
-- 2. Renames columns (domain_url → url, domain_id → website_id, etc.)
-- 3. Creates listing_website_sync table for decoupled sync tracking
-- 4. Migrates existing sync data from listings table
-- 5. Removes old sync columns from listings table

-- ============================================================================
-- STEP 1: Rename Tables
-- ============================================================================

ALTER TABLE domains RENAME TO websites;
ALTER TABLE domain_snapshots RENAME TO website_snapshots;
ALTER TABLE domain_assessments RENAME TO website_assessments;
ALTER TABLE domain_research RENAME TO website_research;

-- ============================================================================
-- STEP 2: Rename Columns in websites table
-- ============================================================================

ALTER TABLE websites RENAME COLUMN domain_url TO url;
ALTER TABLE websites RENAME COLUMN is_trusted_domain TO is_trusted;

-- ============================================================================
-- STEP 3: Rename Foreign Key Columns (domain_id → website_id)
-- ============================================================================

-- website_snapshots
ALTER TABLE website_snapshots RENAME COLUMN domain_id TO website_id;

-- website_assessments
ALTER TABLE website_assessments RENAME COLUMN domain_id TO website_id;

-- website_research
ALTER TABLE website_research RENAME COLUMN domain_id TO website_id;

-- listings
ALTER TABLE listings RENAME COLUMN domain_id TO website_id;

-- ============================================================================
-- STEP 4: Rename Indexes and Constraints
-- ============================================================================

-- Drop old constraints/indexes
ALTER TABLE website_snapshots DROP CONSTRAINT IF EXISTS domain_snapshots_domain_id_fkey;
ALTER TABLE website_assessments DROP CONSTRAINT IF EXISTS domain_assessments_domain_id_fkey;
ALTER TABLE website_research DROP CONSTRAINT IF EXISTS domain_research_domain_id_fkey;
ALTER TABLE listings DROP CONSTRAINT IF EXISTS listings_domain_id_fkey;

DROP INDEX IF EXISTS idx_domain_snapshots_domain_id;
DROP INDEX IF EXISTS idx_domain_snapshots_snapshot_hash;
DROP INDEX IF EXISTS idx_domain_assessments_domain_id;
DROP INDEX IF EXISTS idx_domain_research_domain_id;
DROP INDEX IF EXISTS idx_listings_domain_id;

-- Recreate with new names
ALTER TABLE website_snapshots
    ADD CONSTRAINT website_snapshots_website_id_fkey
    FOREIGN KEY (website_id) REFERENCES websites(id) ON DELETE CASCADE;

ALTER TABLE website_assessments
    ADD CONSTRAINT website_assessments_website_id_fkey
    FOREIGN KEY (website_id) REFERENCES websites(id) ON DELETE CASCADE;

ALTER TABLE website_research
    ADD CONSTRAINT website_research_website_id_fkey
    FOREIGN KEY (website_id) REFERENCES websites(id) ON DELETE CASCADE;

ALTER TABLE listings
    ADD CONSTRAINT listings_website_id_fkey
    FOREIGN KEY (website_id) REFERENCES websites(id) ON DELETE SET NULL;

-- Recreate indexes
CREATE INDEX idx_website_snapshots_website_id ON website_snapshots(website_id);
-- Note: snapshot_hash column doesn't exist on website_snapshots, removed invalid index
CREATE INDEX idx_website_assessments_website_id ON website_assessments(website_id);
CREATE INDEX idx_website_research_website_id ON website_research(website_id);
CREATE INDEX idx_listings_website_id ON listings(website_id);

-- ============================================================================
-- STEP 5: Create listing_website_sync Table
-- ============================================================================

CREATE TABLE listing_website_sync (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    listing_id UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    website_id UUID NOT NULL REFERENCES websites(id) ON DELETE CASCADE,
    content_hash TEXT NOT NULL,
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    disappeared_at TIMESTAMPTZ,
    source_url TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(listing_id, website_id)
);

-- Indexes for listing_website_sync
CREATE INDEX idx_listing_website_sync_listing_id ON listing_website_sync(listing_id);
CREATE INDEX idx_listing_website_sync_website_id ON listing_website_sync(website_id);
CREATE INDEX idx_listing_website_sync_content_hash ON listing_website_sync(content_hash);
CREATE INDEX idx_listing_website_sync_disappeared_at ON listing_website_sync(disappeared_at) WHERE disappeared_at IS NULL;

-- ============================================================================
-- STEP 6: Migrate Existing Sync Data from listings to listing_website_sync
-- ============================================================================

INSERT INTO listing_website_sync (
    listing_id,
    website_id,
    content_hash,
    first_seen_at,
    last_seen_at,
    disappeared_at,
    source_url,
    created_at,
    updated_at
)
SELECT
    l.id,
    l.website_id,
    COALESCE(l.content_hash, MD5(COALESCE(l.title, '') || COALESCE(l.description, ''))),
    l.created_at,
    COALESCE(l.last_seen_at, l.created_at),
    l.disappeared_at,
    COALESCE(l.source_url, ''),
    l.created_at,
    l.updated_at
FROM listings l
WHERE l.website_id IS NOT NULL;

-- ============================================================================
-- STEP 7: Remove Old Sync Columns from listings
-- ============================================================================

ALTER TABLE listings DROP COLUMN IF EXISTS last_seen_at;
ALTER TABLE listings DROP COLUMN IF EXISTS disappeared_at;
ALTER TABLE listings DROP COLUMN IF EXISTS content_hash;

-- ============================================================================
-- STEP 8: Update Unique Constraints
-- ============================================================================

-- Drop old unique constraint on listings if it exists
-- (listings may have had unique constraint on domain_id + content_hash)
DROP INDEX IF EXISTS idx_listings_domain_id_content_hash;

-- ============================================================================
-- Summary of Changes
-- ============================================================================
-- Tables renamed:
--   domains → websites
--   domain_snapshots → website_snapshots
--   domain_assessments → website_assessments
--   domain_research → website_research
--
-- Columns renamed:
--   websites.domain_url → url
--   websites.is_trusted_domain → is_trusted
--   All domain_id FKs → website_id
--
-- New table created:
--   listing_website_sync (for decoupled sync tracking)
--
-- Columns removed from listings:
--   last_seen_at, disappeared_at, content_hash
--   (migrated to listing_website_sync)
--
-- Data preserved:
--   All existing sync data migrated to listing_website_sync
--   website_id kept in listings for traceability
