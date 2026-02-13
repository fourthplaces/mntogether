-- Resources table - simplified content model for extracted services/programs
-- Replaces the complex Listing model with a cleaner, normalized schema
--
-- Naming:
-- - "resources" = extracted content from websites (services, programs, opportunities)
-- - "listings" = the old complex model (to be deprecated)
-- - "posts" = temporal announcements (existing table, different concept)

-- Main resources table
CREATE TABLE resources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    website_id UUID NOT NULL REFERENCES websites(id) ON DELETE CASCADE,

    -- Core content
    title TEXT NOT NULL,
    content TEXT NOT NULL,

    -- Location/service area
    location TEXT,

    -- Workflow status
    status TEXT NOT NULL DEFAULT 'pending_approval' CHECK (status IN (
        'pending_approval', 'active', 'rejected', 'expired'
    )),

    -- Source tracking
    organization_name TEXT,  -- Extracted org name (denormalized for display)

    -- Vector search (for semantic matching and deduplication)
    embedding vector(1536),

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for resources
CREATE INDEX idx_resources_website_id ON resources(website_id);
CREATE INDEX idx_resources_status ON resources(status);
CREATE INDEX idx_resources_created_at ON resources(created_at DESC);

-- HNSW index for fast vector similarity search (deduplication pre-filter)
CREATE INDEX idx_resources_embedding ON resources USING hnsw (embedding vector_cosine_ops);

-- Track which pages a resource was extracted from (many-to-many)
CREATE TABLE resource_sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    resource_id UUID NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    page_url TEXT NOT NULL,
    snapshot_id UUID REFERENCES page_snapshots(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(resource_id, page_url)
);

CREATE INDEX idx_resource_sources_resource_id ON resource_sources(resource_id);
CREATE INDEX idx_resource_sources_snapshot_id ON resource_sources(snapshot_id);

-- Tags for resources (reuses existing tags table via polymorphic pattern)
CREATE TABLE resource_tags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    resource_id UUID NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(resource_id, tag_id)
);

CREATE INDEX idx_resource_tags_resource_id ON resource_tags(resource_id);
CREATE INDEX idx_resource_tags_tag_id ON resource_tags(tag_id);

-- Version history for audit trail
-- Every change creates a new version record
CREATE TABLE resource_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    resource_id UUID NOT NULL REFERENCES resources(id) ON DELETE CASCADE,

    -- Snapshot of content at this version
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    location TEXT,

    -- Why this version was created
    change_reason TEXT NOT NULL CHECK (change_reason IN (
        'created',       -- Initial creation
        'ai_update',     -- AI detected content change and updated
        'manual_edit',   -- Admin manually edited
        'ai_merge'       -- AI merged content from multiple sources
    )),

    -- Deduplication decision context (for ai_update/ai_merge)
    dedup_decision JSONB,  -- { matched_resource_id, similarity_score, ai_reasoning }

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_resource_versions_resource_id ON resource_versions(resource_id);
CREATE INDEX idx_resource_versions_created_at ON resource_versions(created_at DESC);

-- Add 'resource' to contacts table's contactable_type constraint
DO $$
BEGIN
    -- The contacts table uses a CHECK constraint, not an ENUM
    -- We need to drop and recreate it to add 'resource'
    ALTER TABLE contacts DROP CONSTRAINT IF EXISTS contacts_contactable_type_check;

    -- Add new constraint including 'resource'
    ALTER TABLE contacts ADD CONSTRAINT contacts_contactable_type_check
        CHECK (contactable_type IN ('organization', 'listing', 'provider', 'resource'));
END $$;

-- Add 'resource' to taggables table's taggable_type constraint
DO $$
BEGIN
    -- Drop existing constraint if any
    ALTER TABLE taggables DROP CONSTRAINT IF EXISTS taggables_taggable_type_check;

    -- Add new constraint including 'resource'
    ALTER TABLE taggables ADD CONSTRAINT taggables_taggable_type_check
        CHECK (taggable_type IN ('listing', 'organization', 'referral_document', 'domain', 'provider', 'container', 'website', 'resource'));
END $$;
