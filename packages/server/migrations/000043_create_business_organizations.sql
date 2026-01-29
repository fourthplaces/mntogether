-- Create business_organizations table for business-specific properties
-- Follows same pattern as listings → service_listings, opportunity_listings, business_listings
--
-- Design principle: Fields for quantitative/structured data, Tags for categorical metadata
-- See TAGS_VS_FIELDS.md for rationale

-- ============================================================================
-- Step 1: Create business_organizations extension table
-- ============================================================================

CREATE TABLE business_organizations (
  organization_id UUID PRIMARY KEY REFERENCES organizations(id) ON DELETE CASCADE,

  -- Business info (structured data)
  business_type TEXT,      -- 'retail', 'restaurant', 'service', 'manufacturer', etc.
  founded_year INT,
  employee_count TEXT,     -- '1-10', '11-50', '51-200', etc.

  -- Cause-driven commerce (quantitative + relationships)
  proceeds_percentage DECIMAL(5,2) CHECK (proceeds_percentage >= 0 AND proceeds_percentage <= 100),
  proceeds_beneficiary_id UUID REFERENCES organizations(id),  -- Org receiving proceeds
  proceeds_description TEXT,  -- "15% of all sales support immigrant families"
  impact_statement TEXT,      -- "Each purchase helps fund legal consultations"

  -- Direct support (URLs + methods)
  accepts_donations BOOL DEFAULT false,
  donation_link TEXT,
  donation_methods TEXT[],  -- ['paypal', 'venmo', 'cash_app', 'credit_card']

  -- Gift cards (URLs)
  gift_cards_available BOOL DEFAULT false,
  gift_card_link TEXT,

  -- Commerce capabilities (operational)
  online_store_url TEXT,
  delivery_available BOOL DEFAULT false,
  pickup_available BOOL DEFAULT false,
  ships_nationally BOOL DEFAULT false,

  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for common queries
CREATE INDEX idx_business_orgs_proceeds ON business_organizations(proceeds_percentage)
  WHERE proceeds_percentage IS NOT NULL AND proceeds_percentage > 0;

CREATE INDEX idx_business_orgs_beneficiary ON business_organizations(proceeds_beneficiary_id)
  WHERE proceeds_beneficiary_id IS NOT NULL;

CREATE INDEX idx_business_orgs_donations ON business_organizations(accepts_donations)
  WHERE accepts_donations = true;

-- Comments
COMMENT ON TABLE business_organizations IS 'Business-specific properties. Ownership/certifications are tags (not fields).';
COMMENT ON COLUMN business_organizations.proceeds_percentage IS 'Percentage of sales donated (0-100). Use for cause-driven commerce.';
COMMENT ON COLUMN business_organizations.proceeds_beneficiary_id IS 'Organization receiving proceeds (can reference any org in system).';
COMMENT ON COLUMN business_organizations.business_type IS 'Industry/category: retail, restaurant, service, manufacturer, etc.';
COMMENT ON COLUMN business_organizations.donation_methods IS 'Payment methods accepted: paypal, venmo, cash_app, credit_card, etc.';

-- ============================================================================
-- Step 2: Seed ownership tags (categorical metadata)
-- ============================================================================

INSERT INTO tags (kind, value, display_name) VALUES
  ('ownership', 'minority_owned', 'Minority-Owned'),
  ('ownership', 'women_owned', 'Women-Owned'),
  ('ownership', 'lgbtq_owned', 'LGBTQ+-Owned'),
  ('ownership', 'veteran_owned', 'Veteran-Owned'),
  ('ownership', 'immigrant_owned', 'Immigrant-Owned'),
  ('ownership', 'bipoc_owned', 'BIPOC-Owned'),
  ('ownership', 'family_owned', 'Family-Owned'),
  ('ownership', 'disabled_owned', 'Disabled-Owned')
ON CONFLICT (kind, value) DO NOTHING;

-- ============================================================================
-- Step 3: Seed certification tags
-- ============================================================================

INSERT INTO tags (kind, value, display_name) VALUES
  ('certification', 'b_corp', 'Certified B Corporation'),
  ('certification', 'benefit_corp', 'Benefit Corporation'),
  ('certification', 'one_percent_planet', '1% for the Planet'),
  ('certification', 'fair_trade', 'Fair Trade Certified'),
  ('certification', 'organic', 'USDA Organic'),
  ('certification', 'living_wage', 'Living Wage Employer'),
  ('certification', 'green_business', 'Green Business Certified')
ON CONFLICT (kind, value) DO NOTHING;

-- ============================================================================
-- Step 4: Seed worker structure tags
-- ============================================================================

INSERT INTO tags (kind, value, display_name) VALUES
  ('worker_structure', 'worker_owned', 'Worker-Owned'),
  ('worker_structure', 'cooperative', 'Worker Cooperative'),
  ('worker_structure', 'employee_owned', 'Employee-Owned (ESOP)'),
  ('worker_structure', 'union_shop', 'Union Shop'),
  ('worker_structure', 'profit_sharing', 'Profit Sharing')
ON CONFLICT (kind, value) DO NOTHING;

-- ============================================================================
-- Step 5: Seed business model tags (cause-driven commerce)
-- ============================================================================

INSERT INTO tags (kind, value, display_name) VALUES
  ('business_model', 'cause_driven', 'Cause-Driven Commerce'),
  ('business_model', 'social_enterprise', 'Social Enterprise'),
  ('business_model', 'nonprofit_venture', 'Nonprofit Venture')
ON CONFLICT (kind, value) DO NOTHING;

-- ============================================================================
-- Step 6: Update tags table comment
-- ============================================================================

COMMENT ON TABLE tags IS 'Universal tags: business_model, ownership, certification, worker_structure, impact_area, community_served, service_area, population, org_leadership, safety, verification_source';

-- ============================================================================
-- Example: Bailey Aro (cause-driven retail business)
-- ============================================================================

/*
-- 1. Create beneficiary org
INSERT INTO organizations (name, website, organization_type, verified)
VALUES ('Community Legal Aid Fund', 'https://example.org', 'nonprofit', true)
RETURNING id;  -- Assume: 'legal-aid-uuid'

-- 2. Create business org
INSERT INTO organizations (name, website, organization_type, description)
VALUES (
  'Bailey Aro',
  'https://www.baileyaro.com/',
  'business',
  'Apparel and accessories brand supporting immigrant communities'
) RETURNING id;  -- Assume: 'bailey-aro-uuid'

-- 3. Add business properties (fields)
INSERT INTO business_organizations (
  organization_id,
  business_type,
  proceeds_percentage,
  proceeds_beneficiary_id,
  proceeds_description,
  impact_statement,
  online_store_url,
  ships_nationally
) VALUES (
  'bailey-aro-uuid',
  'retail',
  15.00,
  'legal-aid-uuid',
  '15% of all sales support immigrant families',
  'Each purchase helps fund legal consultations for families navigating the immigration system',
  'https://www.baileyaro.com/',
  true
);

-- 4. Add categorical metadata (tags)
INSERT INTO taggables (tag_id, taggable_type, taggable_id) VALUES
  -- Business model
  ((SELECT id FROM tags WHERE kind='business_model' AND value='cause_driven'), 'organization', 'bailey-aro-uuid'),
  -- Ownership
  ((SELECT id FROM tags WHERE kind='ownership' AND value='women_owned'), 'organization', 'bailey-aro-uuid'),
  -- Impact areas
  ((SELECT id FROM tags WHERE kind='impact_area' AND value='immigrant_rights'), 'organization', 'bailey-aro-uuid'),
  ((SELECT id FROM tags WHERE kind='impact_area' AND value='legal_aid'), 'organization', 'bailey-aro-uuid');

-- 5. Create listing (what Bailey Aro offers)
INSERT INTO listings (
  organization_id,
  listing_type,
  category,
  title,
  description,
  status
) VALUES (
  'bailey-aro-uuid',
  'business',
  'shopping',
  'Shop Apparel & Accessories',
  'Browse our collection of sustainably-made apparel and accessories. Every purchase supports immigrant legal aid.',
  'active'
) RETURNING id;  -- Assume: 'listing-uuid'

-- 6. Add listing-specific commerce details
INSERT INTO business_listings (
  listing_id,
  product_category,
  price_range
) VALUES (
  'listing-uuid',
  'merchandise',
  '$$'
);
*/
