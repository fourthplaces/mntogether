# Final Schema Summary - Minimal Design

## Design Philosophy

**Keep it minimal. Only add fields that are absolutely essential.**

- **Fields**: For queries, relationships, and CTAs
- **description**: For all narrative content (mission, story, impact, etc.)
- **Tags**: For categorical metadata (ownership, certifications, etc.)

Don't split hairs. Don't create fields for data we might not have.

## Schema

### organizations (base table - already exists)
```sql
CREATE TABLE organizations (
  id UUID PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,              -- All narrative content here
  summary TEXT,                  -- For AI embeddings
  website TEXT,
  phone TEXT,
  email TEXT,
  primary_address TEXT,
  latitude FLOAT,
  longitude FLOAT,
  organization_type TEXT,        -- nonprofit | business | community | other
  verified BOOL,
  verified_at TIMESTAMPTZ,
  claim_token TEXT,
  claim_email TEXT,
  claimed_at TIMESTAMPTZ,
  embedding VECTOR(1024),        -- For semantic search
  created_at TIMESTAMPTZ,
  updated_at TIMESTAMPTZ
);
```

### business_organizations (NEW - minimal)
```sql
CREATE TABLE business_organizations (
  organization_id UUID PRIMARY KEY REFERENCES organizations(id),

  -- Cause-driven commerce
  proceeds_percentage DECIMAL(5,2),       -- For queries: "businesses giving >10%"
  proceeds_beneficiary_id UUID,           -- Links to recipient org

  -- Support CTAs
  donation_link TEXT,                     -- "Donate Here" button
  gift_card_link TEXT,                    -- "Buy Gift Cards" button
  online_store_url TEXT,                  -- "Shop Here" button

  created_at TIMESTAMPTZ
);
```

**That's it. 6 fields total (excluding PK + timestamp).**

### NO nonprofit_organizations table
**We are NOT creating this.** Nonprofits are just organizations with type='nonprofit'.

## What's NOT Included (By Design)

### Fields we're NOT creating:
- ❌ mission_statement → use description
- ❌ founded_year → use description
- ❌ ein → not essential
- ❌ tax_exempt_status → not essential
- ❌ annual_budget → not essential
- ❌ business_type → use description or tags
- ❌ employee_count → use description
- ❌ proceeds_description → use description
- ❌ impact_statement → use description
- ❌ accepts_donations → just check if donation_link exists
- ❌ donation_methods → not essential
- ❌ gift_cards_available → just check if gift_card_link exists
- ❌ delivery_available → use description
- ❌ pickup_available → use description
- ❌ ships_nationally → use description
- ❌ minority_owned → use tags
- ❌ women_owned → use tags
- ❌ lgbtq_owned → use tags
- ❌ certified_b_corp → use tags
- ❌ worker_owned → use tags

## Tags (Categorical Metadata)

```sql
-- Ownership
('ownership', 'women_owned')
('ownership', 'minority_owned')
('ownership', 'lgbtq_owned')
('ownership', 'veteran_owned')
('ownership', 'immigrant_owned')
('ownership', 'bipoc_owned')

-- Certifications
('certification', 'b_corp')
('certification', 'benefit_corp')

-- Worker structure
('worker_structure', 'worker_owned')
('worker_structure', 'cooperative')

-- Business models
('business_model', 'cause_driven')
('business_model', 'social_enterprise')

-- Impact areas (already exist)
('impact_area', 'immigrant_rights')
('impact_area', 'legal_aid')
('impact_area', 'education')
('impact_area', 'healthcare')
...
```

## Example: Bailey Aro

### Step 1: Create Organizations
```sql
-- Beneficiary
INSERT INTO organizations (name, organization_type, description)
VALUES (
  'Community Legal Aid',
  'nonprofit',
  'Provides free and low-cost legal services to immigrant families.'
);

-- Business
INSERT INTO organizations (name, website, organization_type, description)
VALUES (
  'Bailey Aro',
  'https://www.baileyaro.com/',
  'business',
  'We create sustainable apparel and accessories. 15% of all proceeds support immigrant families through Community Legal Aid. Every purchase helps fund legal consultations. We ship nationwide.'
);
```

### Step 2: Add Business Properties
```sql
INSERT INTO business_organizations (
  organization_id,
  proceeds_percentage,
  proceeds_beneficiary_id,
  online_store_url
) VALUES (
  'bailey-aro-uuid',
  15.00,
  'legal-aid-uuid',
  'https://www.baileyaro.com/'
);
```

### Step 3: Add Tags
```sql
INSERT INTO taggables (tag_id, taggable_type, taggable_id) VALUES
  ((SELECT id FROM tags WHERE kind='business_model' AND value='cause_driven'), 'organization', 'bailey-aro-uuid'),
  ((SELECT id FROM tags WHERE kind='ownership' AND value='women_owned'), 'organization', 'bailey-aro-uuid'),
  ((SELECT id FROM tags WHERE kind='impact_area' AND value='immigrant_rights'), 'organization', 'bailey-aro-uuid');
```

## Queries

### Find Cause-Driven Businesses
```sql
SELECT o.name, o.description, bo.proceeds_percentage, bo.online_store_url
FROM organizations o
JOIN business_organizations bo ON o.id = bo.organization_id
WHERE bo.proceeds_percentage > 0
ORDER BY bo.proceeds_percentage DESC;
```

### Find Women-Owned Businesses Supporting Immigrant Rights
```sql
SELECT DISTINCT o.*, bo.*
FROM organizations o
JOIN business_organizations bo ON o.id = bo.organization_id
JOIN taggables t1 ON t1.taggable_id = o.id
JOIN tags tag1 ON tag1.id = t1.tag_id AND tag1.kind = 'ownership' AND tag1.value = 'women_owned'
JOIN taggables t2 ON t2.taggable_id = o.id
JOIN tags tag2 ON tag2.id = t2.tag_id AND tag2.kind = 'impact_area' AND tag2.value = 'immigrant_rights';
```

## Migration File

**packages/server/migrations/000045_create_business_organizations_minimal.sql**
- Creates business_organizations table (6 fields)
- Seeds essential tags (ownership, certification, worker_structure, business_model)
- No nonprofit_organizations table

## Benefits

1. **Simple**: 6 fields, not 17+
2. **Flexible**: description holds all narrative content
3. **Extensible**: Add new tags without migrations
4. **Realistic**: Don't pretend we have structured data when we don't
5. **Easy to populate**: Scrapers just grab description, don't parse into multiple fields
6. **Less maintenance**: Fewer fields = fewer schema changes

## When to Add Fields Later

Only add fields when you absolutely need to:
1. **Query/filter** on a numeric value (proceeds_percentage)
2. **Create relationships** between entities (proceeds_beneficiary_id)
3. **Power CTAs** in the UI (donation_link, online_store_url)

Otherwise, put it in description or tags.
