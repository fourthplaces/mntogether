-- Junction table: many-to-many between organizations and tags

CREATE TABLE tags_on_organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,

    -- Prevent duplicate associations
    UNIQUE(organization_id, tag_id),

    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for common queries
CREATE INDEX idx_tags_on_orgs_org_id ON tags_on_organizations(organization_id);
CREATE INDEX idx_tags_on_orgs_tag_id ON tags_on_organizations(tag_id);

-- Composite index for "find orgs with specific tag"
CREATE INDEX idx_tags_on_orgs_tag_org ON tags_on_organizations(tag_id, organization_id);

COMMENT ON TABLE tags_on_organizations IS 'Junction table linking organizations to tags';
