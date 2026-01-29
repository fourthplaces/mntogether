-- Add support for cause-driven commerce (businesses where proceeds benefit charitable causes)
-- Example: Bailey Aro selling merchandise where 15% goes to immigrant legal aid

-- Step 1: Extend business_listings with proceeds allocation fields
ALTER TABLE business_listings
  ADD COLUMN IF NOT EXISTS proceeds_percentage DECIMAL(5,2) CHECK (proceeds_percentage >= 0 AND proceeds_percentage <= 100),
  ADD COLUMN IF NOT EXISTS proceeds_beneficiary_id UUID REFERENCES organizations(id),
  ADD COLUMN IF NOT EXISTS proceeds_description TEXT,
  ADD COLUMN IF NOT EXISTS impact_statement TEXT;

CREATE INDEX idx_business_listings_beneficiary ON business_listings(proceeds_beneficiary_id) WHERE proceeds_beneficiary_id IS NOT NULL;

COMMENT ON COLUMN business_listings.proceeds_percentage IS 'Percentage of sales/proceeds that go to a cause (0-100)';
COMMENT ON COLUMN business_listings.proceeds_beneficiary_id IS 'Organization that receives proceeds (null if general cause)';
COMMENT ON COLUMN business_listings.proceeds_description IS 'Short description: "15% of sales support immigrant families"';
COMMENT ON COLUMN business_listings.impact_statement IS 'Impact per purchase: "Your purchase funds 30 minutes of legal consultation"';

-- Step 2: Add social enterprise tags for discovery
INSERT INTO tags (kind, value, display_name) VALUES
  ('business_model', 'cause_driven', 'Cause-Driven Commerce'),
  ('business_model', 'social_enterprise', 'Social Enterprise'),
  ('business_model', 'b_corp', 'B Corporation'),
  ('business_model', 'cooperative', 'Worker Cooperative'),
  ('business_model', 'nonprofit_venture', 'Nonprofit Venture')
ON CONFLICT (kind, value) DO NOTHING;

-- Step 3: Add impact area tags (what causes do proceeds support?)
INSERT INTO tags (kind, value, display_name) VALUES
  ('impact_area', 'immigrant_rights', 'Immigrant Rights'),
  ('impact_area', 'legal_aid', 'Legal Aid'),
  ('impact_area', 'education', 'Education'),
  ('impact_area', 'healthcare', 'Healthcare Access'),
  ('impact_area', 'housing', 'Housing Security'),
  ('impact_area', 'food_security', 'Food Security'),
  ('impact_area', 'youth_programs', 'Youth Programs'),
  ('impact_area', 'environmental', 'Environmental Justice'),
  ('impact_area', 'arts_culture', 'Arts & Culture')
ON CONFLICT (kind, value) DO NOTHING;

COMMENT ON TABLE tags IS 'Universal tags. business_model tags mark social enterprises. impact_area tags show what causes proceeds support.';

-- Step 4: Example data for Bailey Aro style business
-- (This would be populated by admin or scraper, shown here for documentation)

/*
Example: Bailey Aro merchandise business

INSERT INTO organizations (name, website, organization_type, verified) VALUES
  ('Community Legal Aid', 'https://example.org', 'nonprofit', true)
RETURNING id; -- assume returns 'abc-123...'

INSERT INTO organizations (name, website, organization_type) VALUES
  ('Bailey Aro', 'https://www.baileyaro.com/', 'business')
RETURNING id; -- assume returns 'xyz-789...'

INSERT INTO listings (
  organization_id,
  listing_type,
  category,
  title,
  description,
  status
) VALUES (
  'xyz-789...',
  'business',
  'shopping',
  'Support Bailey Aro - Merchandise That Gives Back',
  'Shop apparel and accessories where a portion of proceeds directly supports immigrant legal aid services.',
  'active'
) RETURNING id; -- assume returns 'listing-456...'

INSERT INTO business_listings (
  listing_id,
  online_ordering_link,
  proceeds_percentage,
  proceeds_beneficiary_id,
  proceeds_description,
  impact_statement
) VALUES (
  'listing-456...',
  'https://www.baileyaro.com/',
  15.00,
  'abc-123...', -- Community Legal Aid org id
  '15% of all sales support immigrant families',
  'Each purchase helps fund legal consultations for families navigating the immigration system'
);

-- Tag the business
INSERT INTO taggables (tag_id, taggable_type, taggable_id) VALUES
  ((SELECT id FROM tags WHERE kind='business_model' AND value='cause_driven'), 'listing', 'listing-456...'),
  ((SELECT id FROM tags WHERE kind='impact_area' AND value='immigrant_rights'), 'listing', 'listing-456...'),
  ((SELECT id FROM tags WHERE kind='impact_area' AND value='legal_aid'), 'listing', 'listing-456...');
*/
