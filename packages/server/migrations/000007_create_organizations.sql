-- Organizations table: lightweight anchor for grouping needs
-- Uses existing patterns: contact_info JSONB, location TEXT

CREATE TABLE organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Core identity
    name TEXT NOT NULL,
    description TEXT,

    -- Contact (reuses same JSONB pattern as organization_needs)
    contact_info JSONB,  -- { email, phone, website }

    -- Location (free-text, no parsing)
    location TEXT,  -- e.g., "Brooklyn Center, MN" or "Hennepin County"
    city TEXT,
    state TEXT DEFAULT 'MN',

    -- Simple status
    status TEXT DEFAULT 'active' CHECK (status IN ('pending', 'active', 'inactive')),

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for queries that matter
CREATE INDEX idx_organizations_status ON organizations(status) WHERE status = 'active';

-- Full text search on name and description
CREATE INDEX idx_organizations_search ON organizations USING GIN(
    to_tsvector('english', name || ' ' || COALESCE(description, ''))
);

COMMENT ON TABLE organizations IS 'Lightweight anchor for grouping needs and sources. Not a canonical directory.';
COMMENT ON COLUMN organizations.contact_info IS 'JSONB: { email, phone, website } - same pattern as organization_needs';
COMMENT ON COLUMN organizations.location IS 'Free-text location, no parsing. E.g., "Brooklyn Center, MN" or "Hennepin County"';
