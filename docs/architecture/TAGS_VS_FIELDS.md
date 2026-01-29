# Tags vs Fields Design Decision

## Philosophy

**Fields** = Quantitative data, foreign keys, operational capabilities, structured data that needs to be queried with comparisons

**Tags** = Categorical metadata, discovery attributes, boolean-like properties that are used for filtering

## Business Organization Properties

### ✅ Keep as FIELDS

#### Quantitative & Transactional
```sql
-- Numeric values that need range queries
proceeds_percentage DECIMAL(5,2)  -- Can query: WHERE proceeds_percentage > 10

-- Foreign key relationships
proceeds_beneficiary_id UUID REFERENCES organizations(id)

-- Text descriptions (not categorical)
proceeds_description TEXT
impact_statement TEXT

-- URLs (structured data)
donation_link TEXT
gift_card_link TEXT
online_store_url TEXT

-- Operational capabilities (may have nuance)
delivery_available BOOL
pickup_available BOOL
ships_nationally BOOL

-- Structured metadata
business_type TEXT  -- 'retail', 'restaurant', 'service', 'manufacturer'
founded_year INT
employee_count TEXT  -- '1-10', '11-50', etc.
donation_methods TEXT[]  -- ['paypal', 'venmo', 'cash_app']
```

### ✅ Move to TAGS

#### Ownership (kind = 'ownership')
```sql
-- These are pure categorical filters
minority_owned → tag(ownership, minority_owned)
women_owned → tag(ownership, women_owned)
lgbtq_owned → tag(ownership, lgbtq_owned)
veteran_owned → tag(ownership, veteran_owned)
immigrant_owned → tag(ownership, immigrant_owned)
bipoc_owned → tag(ownership, bipoc_owned)
family_owned → tag(ownership, family_owned)
```

**Why tags?**
- Used for discovery: "Show me women-owned businesses"
- Combinable: Can be both women-owned AND minority-owned
- Extensible: Easy to add new ownership types without schema changes
- Already have org_leadership tags (community_led, immigrant_founded)

#### Certifications (kind = 'certification')
```sql
certified_b_corp → tag(certification, b_corp)
benefit_corporation → tag(certification, benefit_corp)
1_percent_for_planet → tag(certification, one_percent_planet)
fair_trade_certified → tag(certification, fair_trade)
organic_certified → tag(certification, organic)
living_wage_employer → tag(certification, living_wage)
```

**Why tags?**
- Certifications are badges/markers for discovery
- Organizations can have multiple certifications
- New certifications emerge over time
- Used primarily for filtering, not computation

#### Worker Structure (kind = 'worker_structure')
```sql
worker_owned → tag(worker_structure, worker_owned)
worker_cooperative → tag(worker_structure, cooperative)
employee_owned → tag(worker_structure, employee_owned)
union_shop → tag(worker_structure, union_shop)
profit_sharing → tag(worker_structure, profit_sharing)
```

**Why tags?**
- Categorical descriptor of how business is structured
- Used for filtering: "Show me worker cooperatives"
- Can have multiple structures (e.g., union_shop + profit_sharing)

## Updated business_organizations Schema

```sql
CREATE TABLE business_organizations (
  organization_id UUID PRIMARY KEY REFERENCES organizations(id) ON DELETE CASCADE,

  -- Business info (structured data)
  business_type TEXT,
  founded_year INT,
  employee_count TEXT,

  -- Cause-driven commerce (quantitative + relationships)
  proceeds_percentage DECIMAL(5,2) CHECK (proceeds_percentage >= 0 AND proceeds_percentage <= 100),
  proceeds_beneficiary_id UUID REFERENCES organizations(id),
  proceeds_description TEXT,
  impact_statement TEXT,

  -- Direct support (URLs)
  accepts_donations BOOL DEFAULT false,
  donation_link TEXT,
  donation_methods TEXT[],

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

-- NO ownership, certification, or worker_structure fields!
-- Those are all tags.
```

## Tag Seeds

```sql
-- Ownership tags
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

-- Certification tags
INSERT INTO tags (kind, value, display_name) VALUES
  ('certification', 'b_corp', 'Certified B Corporation'),
  ('certification', 'benefit_corp', 'Benefit Corporation'),
  ('certification', 'one_percent_planet', '1% for the Planet'),
  ('certification', 'fair_trade', 'Fair Trade Certified'),
  ('certification', 'organic', 'USDA Organic'),
  ('certification', 'living_wage', 'Living Wage Employer'),
  ('certification', 'green_business', 'Green Business Certified')
ON CONFLICT (kind, value) DO NOTHING;

-- Worker structure tags
INSERT INTO tags (kind, value, display_name) VALUES
  ('worker_structure', 'worker_owned', 'Worker-Owned'),
  ('worker_structure', 'cooperative', 'Worker Cooperative'),
  ('worker_structure', 'employee_owned', 'Employee-Owned (ESOP)'),
  ('worker_structure', 'union_shop', 'Union Shop'),
  ('worker_structure', 'profit_sharing', 'Profit Sharing')
ON CONFLICT (kind, value) DO NOTHING;

-- Business model tags (already exist from 000043)
INSERT INTO tags (kind, value, display_name) VALUES
  ('business_model', 'cause_driven', 'Cause-Driven Commerce'),
  ('business_model', 'social_enterprise', 'Social Enterprise'),
  ('business_model', 'nonprofit_venture', 'Nonprofit Venture')
ON CONFLICT (kind, value) DO NOTHING;
```

## Example: Bailey Aro

### Organization
```sql
INSERT INTO organizations (name, website, organization_type)
VALUES ('Bailey Aro', 'https://baileyaro.com', 'business');
```

### Business Properties (Fields)
```sql
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
  'Each purchase helps fund legal consultations',
  'https://baileyaro.com',
  true
);
```

### Tags (Categorical Metadata)
```sql
-- Business model
INSERT INTO taggables (tag_id, taggable_type, taggable_id) VALUES
  ((SELECT id FROM tags WHERE kind='business_model' AND value='cause_driven'), 'organization', 'bailey-aro-uuid');

-- Ownership
INSERT INTO taggables (tag_id, taggable_type, taggable_id) VALUES
  ((SELECT id FROM tags WHERE kind='ownership' AND value='women_owned'), 'organization', 'bailey-aro-uuid');

-- Impact areas
INSERT INTO taggables (tag_id, taggable_type, taggable_id) VALUES
  ((SELECT id FROM tags WHERE kind='impact_area' AND value='immigrant_rights'), 'organization', 'bailey-aro-uuid'),
  ((SELECT id FROM tags WHERE kind='impact_area' AND value='legal_aid'), 'organization', 'bailey-aro-uuid');
```

## Query Examples

### Find Women-Owned Cause-Driven Businesses
```sql
SELECT DISTINCT o.*, bo.*
FROM organizations o
JOIN business_organizations bo ON o.id = bo.organization_id
JOIN taggables t1 ON t1.taggable_id = o.id AND t1.taggable_type = 'organization'
JOIN tags tag1 ON tag1.id = t1.tag_id AND tag1.kind = 'ownership' AND tag1.value = 'women_owned'
JOIN taggables t2 ON t2.taggable_id = o.id AND t2.taggable_type = 'organization'
JOIN tags tag2 ON tag2.id = t2.tag_id AND tag2.kind = 'business_model' AND tag2.value = 'cause_driven'
WHERE bo.proceeds_percentage > 0;
```

### Find B Corps That Ship Nationally
```sql
SELECT o.*, bo.*
FROM organizations o
JOIN business_organizations bo ON o.id = bo.organization_id
JOIN taggables t ON t.taggable_id = o.id AND t.taggable_type = 'organization'
JOIN tags tag ON tag.id = t.tag_id AND tag.kind = 'certification' AND tag.value = 'b_corp'
WHERE bo.ships_nationally = true;
```

### Filter by Multiple Ownership Types
```sql
-- Find businesses that are BOTH women-owned AND minority-owned
SELECT o.*,
  array_agg(DISTINCT tag.value) FILTER (WHERE tag.kind = 'ownership') as ownership_tags
FROM organizations o
JOIN taggables t ON t.taggable_id = o.id AND t.taggable_type = 'organization'
JOIN tags tag ON tag.id = t.tag_id
WHERE o.organization_type = 'business'
GROUP BY o.id
HAVING array_agg(DISTINCT tag.value) FILTER (WHERE tag.kind = 'ownership') @> ARRAY['women_owned', 'minority_owned'];
```

## Benefits

### 1. Cleaner Schema
- business_organizations table has ~10 fields instead of ~20
- No boolean bloat
- Clear separation of structured data vs categorical metadata

### 2. Extensibility
- Add new ownership types without migrations
- Add new certifications as they emerge
- Community can suggest new tags via UI

### 3. Flexible Queries
- Combine multiple tags: "women-owned AND veteran-owned AND b-corp"
- Tag aggregation: "Show all ownership types for this org"
- Tag clouds: "Most common certifications in our directory"

### 4. Consistent with Existing System
- Already using tags for: community_served, service_area, population, org_leadership, safety
- Ownership/certification/worker_structure fit the same pattern

### 5. UI-Friendly
- Badges display: Show all tags as visual badges
- Filter chips: Click to add/remove tag filters
- Auto-complete: Suggest existing tags when tagging new orgs

## Migration Strategy

Since this is pre-launch:
1. Don't add boolean fields to business_organizations
2. Seed ownership/certification/worker_structure tags immediately
3. Use tags from the start for categorical metadata
