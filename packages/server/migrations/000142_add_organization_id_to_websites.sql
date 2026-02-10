ALTER TABLE websites
    ADD COLUMN organization_id UUID REFERENCES organizations(id);

CREATE INDEX idx_websites_organization_id ON websites(organization_id);
