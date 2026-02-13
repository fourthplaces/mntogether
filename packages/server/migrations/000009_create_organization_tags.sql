-- Organization tags: flexible tagging for matching and filtering
-- Pattern: Tag { kind: "service" | "language" | "community", value: "food_assistance" }

CREATE TABLE organization_tags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,

    -- Tag classification
    kind TEXT NOT NULL CHECK (kind IN ('service', 'language', 'community')),
    value TEXT NOT NULL,

    -- Prevent duplicates
    UNIQUE(organization_id, kind, value),

    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for filtering queries
CREATE INDEX idx_organization_tags_org_id ON organization_tags(organization_id);
CREATE INDEX idx_organization_tags_kind_value ON organization_tags(kind, value);

-- Index for "show me all orgs that provide X service"
CREATE INDEX idx_organization_tags_value ON organization_tags(value) WHERE kind = 'service';

COMMENT ON TABLE organization_tags IS 'Flexible tagging for organizations. Supports service types, languages, and communities served.';
COMMENT ON COLUMN organization_tags.kind IS 'Tag category: service (food_assistance), language (spanish), or community (somali)';
COMMENT ON COLUMN organization_tags.value IS 'Tag value: e.g., food_assistance, spanish, somali, general';

-- Example tags for reference
COMMENT ON TABLE organization_tags IS $comment$
Flexible tagging for organizations.

Example service tags:
  - food_assistance, housing_assistance, legal_services
  - employment_support, emergency_financial_aid, shelter

Example language tags:
  - english, spanish, somali, hmong, karen, vietnamese

Example community tags:
  - latino, somali, hmong, karen, vietnamese, east_african, native_american, general
$comment$;
