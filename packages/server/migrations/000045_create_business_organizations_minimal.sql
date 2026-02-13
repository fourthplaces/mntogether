-- Create business_organizations table (minimal essential fields only)
--
-- Philosophy: Keep it simple. Use org.description for narrative content.
-- Only create fields for: queries, relationships, and CTAs.

CREATE TABLE business_organizations (
  organization_id UUID PRIMARY KEY REFERENCES organizations(id) ON DELETE CASCADE,

  -- Cause-driven commerce (for queries)
  proceeds_percentage DECIMAL(5,2) CHECK (proceeds_percentage >= 0 AND proceeds_percentage <= 100),
  proceeds_beneficiary_id UUID REFERENCES organizations(id),

  -- Support CTAs (links)
  donation_link TEXT,
  gift_card_link TEXT,
  online_store_url TEXT,

  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_business_orgs_proceeds ON business_organizations(proceeds_percentage)
  WHERE proceeds_percentage IS NOT NULL AND proceeds_percentage > 0;

CREATE INDEX idx_business_orgs_beneficiary ON business_organizations(proceeds_beneficiary_id)
  WHERE proceeds_beneficiary_id IS NOT NULL;

COMMENT ON TABLE business_organizations IS 'Business properties: proceeds sharing + support links. Everything else in org.description or tags.';
COMMENT ON COLUMN business_organizations.proceeds_percentage IS 'Percentage of proceeds donated (0-100). Null if not applicable.';
COMMENT ON COLUMN business_organizations.proceeds_beneficiary_id IS 'Organization receiving proceeds (can be any org in system).';

-- Seed tags for categorical metadata
-- (Ownership, certifications, business models are tags, not fields)

INSERT INTO tags (kind, value, display_name) VALUES
  -- Ownership
  ('ownership', 'minority_owned', 'Minority-Owned'),
  ('ownership', 'women_owned', 'Women-Owned'),
  ('ownership', 'lgbtq_owned', 'LGBTQ+-Owned'),
  ('ownership', 'veteran_owned', 'Veteran-Owned'),
  ('ownership', 'immigrant_owned', 'Immigrant-Owned'),
  ('ownership', 'bipoc_owned', 'BIPOC-Owned'),

  -- Certifications
  ('certification', 'b_corp', 'Certified B Corp'),
  ('certification', 'benefit_corp', 'Benefit Corporation'),

  -- Worker structure
  ('worker_structure', 'worker_owned', 'Worker-Owned'),
  ('worker_structure', 'cooperative', 'Worker Cooperative'),

  -- Business models
  ('business_model', 'cause_driven', 'Cause-Driven'),
  ('business_model', 'social_enterprise', 'Social Enterprise')
ON CONFLICT (kind, value) DO NOTHING;
