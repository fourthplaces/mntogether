-- Link organization_sources and organization_needs to organizations table

-- Add organization_id to sources
ALTER TABLE organization_sources
    ADD COLUMN organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE;

CREATE INDEX idx_organization_sources_org_id ON organization_sources(organization_id);

COMMENT ON COLUMN organization_sources.organization_id IS 'Optional link to the organization entity';

-- Add organization_id to needs (but keep organization_name for display)
ALTER TABLE organization_needs
    ADD COLUMN organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE;

CREATE INDEX idx_organization_needs_org_id ON organization_needs(organization_id);

COMMENT ON COLUMN organization_needs.organization_id IS 'Optional link to the organization entity. Keep organization_name for display even if linked.';
