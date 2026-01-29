# Organization-Level Business Properties Schema Design

## Pattern: Same as Listings

### Listings Pattern (Current)
```
listings (base table)
├── service_listings (service-specific properties)
├── opportunity_listings (opportunity-specific properties)
└── business_listings (currently has both org + listing properties - WRONG)
```

### Organizations Pattern (Proposed)
```
organizations (base table)
├── nonprofit_organizations (nonprofit-specific properties)
├── business_organizations (business-specific properties)
└── community_organizations (future - community-specific properties)
```

## Schema Design

### Base Table: `organizations`

```sql
-- Already exists, no changes needed
CREATE TABLE organizations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL,
  description TEXT,
  summary TEXT,  -- For embeddings

  -- Contact
  website TEXT,
  phone TEXT,
  email TEXT,

  -- Location
  primary_address TEXT,
  latitude FLOAT,
  longitude FLOAT,

  -- Type
  organization_type TEXT CHECK (organization_type IN ('nonprofit', 'business', 'community', 'other')),

  -- Verification
  verified BOOL DEFAULT false,
  verified_at TIMESTAMPTZ,

  -- Claiming
  claim_token TEXT UNIQUE,
  claim_email TEXT,
  claimed_at TIMESTAMPTZ,

  -- Embeddings
  embedding VECTOR(1536),

  -- Timestamps
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

### Extension Table: `business_organizations`

```sql
-- NEW: Business-specific properties at organization level
CREATE TABLE business_organizations (
  organization_id UUID PRIMARY KEY REFERENCES organizations(id) ON DELETE CASCADE,

  -- Business info
  business_type TEXT,  -- 'retail', 'restaurant', 'service', 'manufacturer', etc.
  founded_year INT,
  employee_count TEXT,  -- '1-10', '11-50', etc.

  -- Cause-driven commerce (Bailey Aro use case)
  proceeds_percentage DECIMAL(5,2) CHECK (proceeds_percentage >= 0 AND proceeds_percentage <= 100),
  proceeds_beneficiary_id UUID REFERENCES organizations(id),  -- Which org receives proceeds
  proceeds_description TEXT,  -- "15% of all sales support immigrant families"
  impact_statement TEXT,      -- "Each purchase funds 30 minutes of legal consultation"

  -- Direct support options
  accepts_donations BOOL DEFAULT false,
  donation_link TEXT,
  donation_methods TEXT[],  -- ['paypal', 'venmo', 'cash_app', 'credit_card']

  -- Gift cards
  gift_cards_available BOOL DEFAULT false,
  gift_card_link TEXT,

  -- Commerce capabilities
  online_store_url TEXT,
  delivery_available BOOL DEFAULT false,
  pickup_available BOOL DEFAULT false,
  ships_nationally BOOL DEFAULT false,

  -- These moved to tags: ownership, certification, worker_structure kinds
  -- Use tags for: minority_owned, women_owned, lgbtq_owned, veteran_owned,
  --               worker_owned, certified_b_corp, benefit_corp, etc.

  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_business_orgs_proceeds ON business_organizations(proceeds_percentage)
  WHERE proceeds_percentage IS NOT NULL AND proceeds_percentage > 0;

CREATE INDEX idx_business_orgs_beneficiary ON business_organizations(proceeds_beneficiary_id)
  WHERE proceeds_beneficiary_id IS NOT NULL;

COMMENT ON TABLE business_organizations IS 'Business-specific properties including cause-driven commerce and social enterprise markers';
COMMENT ON COLUMN business_organizations.proceeds_percentage IS 'Percentage of sales/proceeds donated to cause (0-100)';
COMMENT ON COLUMN business_organizations.proceeds_beneficiary_id IS 'Organization receiving proceeds (can be another org in system)';
```

### Extension Table: `nonprofit_organizations` (Optional)

```sql
-- OPTIONAL: Nonprofit-specific properties
CREATE TABLE nonprofit_organizations (
  organization_id UUID PRIMARY KEY REFERENCES organizations(id) ON DELETE CASCADE,

  -- Tax status
  ein TEXT,  -- Employer Identification Number
  tax_exempt_status TEXT,  -- '501c3', '501c4', etc.
  tax_exempt_verified BOOL DEFAULT false,

  -- Registration
  state_registration TEXT[],  -- States where registered
  charity_navigator_rating DECIMAL(3,1),  -- 0.0 - 4.0

  -- Mission
  mission_statement TEXT,
  founded_year INT,
  annual_budget_range TEXT,  -- '$0-$50k', '$50k-$250k', etc.

  -- Focus areas (could also use tags)
  focus_areas TEXT[],  -- ['legal_aid', 'immigration', 'education']

  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE nonprofit_organizations IS 'Nonprofit-specific properties including tax status and mission details';
```

### Simplified: `business_listings`

```sql
-- SIMPLIFIED: Remove org-level properties, keep only listing-specific commerce details
CREATE TABLE business_listings (
  listing_id UUID PRIMARY KEY REFERENCES listings(id) ON DELETE CASCADE,

  -- Listing-specific commerce (what THIS listing offers)
  product_category TEXT,  -- 'merchandise', 'food', 'services', 'gift_cards'
  price_range TEXT,       -- '$', '$$', '$$$', '$$$$'
  requires_appointment BOOL DEFAULT false,

  -- Delivery for THIS listing
  delivery_available BOOL DEFAULT false,
  pickup_available BOOL DEFAULT false,
  remote_fulfillment BOOL DEFAULT false,

  created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE business_listings IS 'Commerce properties specific to this listing (NOT organization-level properties)';
```

## Example: Bailey Aro

### 1. Organization (Bailey Aro)

```sql
INSERT INTO organizations (name, website, organization_type, description) VALUES (
  'Bailey Aro',
  'https://www.baileyaro.com/',
  'business',
  'Apparel and accessories brand supporting immigrant communities'
) RETURNING id;  -- Returns: 'bailey-aro-uuid'
```

### 2. Business Properties (Organization Level)

```sql
-- Beneficiary org
INSERT INTO organizations (name, organization_type) VALUES (
  'Community Legal Aid Fund',
  'nonprofit'
) RETURNING id;  -- Returns: 'legal-aid-uuid'

-- Business properties
INSERT INTO business_organizations (
  organization_id,
  business_type,
  proceeds_percentage,
  proceeds_beneficiary_id,
  proceeds_description,
  impact_statement,
  online_store_url,
  ships_nationally,
  women_owned
) VALUES (
  'bailey-aro-uuid',
  'retail',
  15.00,
  'legal-aid-uuid',
  '15% of all sales support immigrant families',
  'Each purchase helps fund legal consultations for families navigating the immigration system',
  'https://www.baileyaro.com/',
  true,
  true
);
```

### 3. Listing (What Bailey Aro Offers)

```sql
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
) RETURNING id;  -- Returns: 'listing-uuid'

-- Listing-specific commerce details
INSERT INTO business_listings (
  listing_id,
  product_category,
  price_range,
  delivery_available,
  pickup_available
) VALUES (
  'listing-uuid',
  'merchandise',
  '$$',
  true,
  false
);
```

### 4. Tags (Discovery)

```sql
-- Tag the ORGANIZATION with business model
INSERT INTO taggables (tag_id, taggable_type, taggable_id) VALUES
  ((SELECT id FROM tags WHERE kind='business_model' AND value='cause_driven'), 'organization', 'bailey-aro-uuid'),
  ((SELECT id FROM tags WHERE kind='impact_area' AND value='immigrant_rights'), 'organization', 'bailey-aro-uuid'),
  ((SELECT id FROM tags WHERE kind='impact_area' AND value='legal_aid'), 'organization', 'bailey-aro-uuid');

-- Tag the LISTING with product type
INSERT INTO taggables (tag_id, taggable_type, taggable_id) VALUES
  ((SELECT id FROM tags WHERE kind='product_type' AND value='apparel'), 'listing', 'listing-uuid');
```

## Query Examples

### Find All Cause-Driven Businesses

```sql
SELECT
  o.id,
  o.name,
  o.website,
  bo.proceeds_percentage,
  bo.proceeds_description,
  bo.impact_statement,
  beneficiary.name as beneficiary_name
FROM organizations o
JOIN business_organizations bo ON o.id = bo.organization_id
LEFT JOIN organizations beneficiary ON bo.proceeds_beneficiary_id = beneficiary.id
WHERE bo.proceeds_percentage > 0
ORDER BY bo.proceeds_percentage DESC;
```

### Get Organization with All Details

```sql
-- Single query to get org + business properties + listings
SELECT
  o.*,
  bo.*,
  json_agg(json_build_object(
    'id', l.id,
    'title', l.title,
    'description', l.description,
    'listing_type', l.listing_type,
    'status', l.status
  )) as listings
FROM organizations o
LEFT JOIN business_organizations bo ON o.id = bo.organization_id
LEFT JOIN listings l ON o.id = l.organization_id
WHERE o.id = 'bailey-aro-uuid'
GROUP BY o.id, bo.organization_id;
```

### Find Businesses by Impact Area

```sql
SELECT DISTINCT o.*, bo.*
FROM organizations o
JOIN business_organizations bo ON o.id = bo.organization_id
JOIN taggables t ON t.taggable_id = o.id AND t.taggable_type = 'organization'
JOIN tags tag ON tag.id = t.tag_id
WHERE tag.kind = 'impact_area'
  AND tag.value = 'immigrant_rights'
  AND o.organization_type = 'business';
```

## Benefits of This Design

### ✅ Clear Separation of Concerns
- **Organization**: Who they are (Bailey Aro)
- **Business Properties**: How they operate (15% proceeds model)
- **Listing**: What they offer ("Shop merchandise")

### ✅ Follows Existing Pattern
Same pattern as `listings` → `service_listings`, `opportunity_listings`, `business_listings`

### ✅ One Organization, Many Listings
Bailey Aro can have multiple listings:
- "Shop Apparel" (business listing)
- "Volunteer at Pop-Up Events" (opportunity listing)
- All share the same organization-level business properties

### ✅ Organization-to-Organization Relationships
`proceeds_beneficiary_id` links two organizations:
- Bailey Aro (business) → Community Legal Aid (nonprofit)
- Both are first-class organizations in the system

### ✅ Flexible Tagging
- Tag organizations: business_model, impact_area, ownership_type
- Tag listings: product_type, service_category, opportunity_type

### ✅ Extensible
Easy to add more extension tables:
- `government_organizations`
- `community_organizations`
- `cooperative_organizations`

## Migration from Current State

```sql
-- Step 1: Create business_organizations table
CREATE TABLE business_organizations (...);

-- Step 2: Migrate data from business_listings → business_organizations
INSERT INTO business_organizations (
  organization_id,
  proceeds_percentage,
  proceeds_beneficiary_id,
  proceeds_description,
  impact_statement,
  accepts_donations,
  donation_link,
  gift_cards_available,
  gift_card_link,
  online_store_url,
  delivery_available
)
SELECT DISTINCT
  l.organization_id,
  bl.proceeds_percentage,
  bl.proceeds_beneficiary_id,
  bl.proceeds_description,
  bl.impact_statement,
  bl.accepts_donations,
  bl.donation_link,
  bl.gift_cards_available,
  bl.gift_card_link,
  bl.online_ordering_link,
  bl.delivery_available
FROM business_listings bl
JOIN listings l ON bl.listing_id = l.id
WHERE l.organization_id IS NOT NULL
GROUP BY l.organization_id, bl.*;  -- Handle multiple listings per org

-- Step 3: Drop migrated columns from business_listings
ALTER TABLE business_listings
  DROP COLUMN proceeds_percentage,
  DROP COLUMN proceeds_beneficiary_id,
  DROP COLUMN proceeds_description,
  DROP COLUMN impact_statement,
  DROP COLUMN accepts_donations,
  DROP COLUMN donation_link,
  DROP COLUMN gift_cards_available,
  DROP COLUMN gift_card_link,
  DROP COLUMN online_ordering_link,
  DROP COLUMN remote_ok;

-- Step 4: Add listing-specific columns to business_listings
ALTER TABLE business_listings
  ADD COLUMN product_category TEXT,
  ADD COLUMN price_range TEXT,
  ADD COLUMN requires_appointment BOOL DEFAULT false;
```

## Comparison

| Property | Current (WRONG) | Proposed (RIGHT) |
|----------|----------------|------------------|
| `proceeds_percentage` | business_listings | business_organizations |
| `donation_link` | business_listings | business_organizations |
| `gift_cards_available` | business_listings | business_organizations |
| `online_store_url` | business_listings | business_organizations |
| `certified_b_corp` | ❌ Missing | business_organizations |
| `product_category` | ❌ Missing | business_listings |
| `price_range` | ❌ Missing | business_listings |

**Rationale**: Bailey Aro's "15% proceeds" policy applies to the ENTIRE ORGANIZATION, not just one listing. They might have multiple listings (merchandise, gift cards, event tickets), but all share the same proceeds policy.
