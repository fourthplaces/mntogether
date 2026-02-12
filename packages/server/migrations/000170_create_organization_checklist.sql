-- Pre-launch checklist items for organizations.
-- All items must be checked before an organization can be approved.
CREATE TABLE organization_checklist_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    checklist_key TEXT NOT NULL,
    checked_by UUID NOT NULL REFERENCES members(id),
    checked_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(organization_id, checklist_key)
);

CREATE INDEX idx_org_checklist_org_id ON organization_checklist_items(organization_id);
