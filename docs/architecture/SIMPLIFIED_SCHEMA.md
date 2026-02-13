# Simplified Schema - Essential Fields Only

## Philosophy

**Keep it minimal.** Don't create fields for every possible attribute. Use:
- **description** field for narrative content (mission, story, impact, proceeds info, etc.)
- **tags** for categorical metadata
- **fields** only for structured data that needs queries/filters

Don't split hairs with proceeds_description vs impact_statement vs mission_statement. Just use description.

## Organizations Table (Already Exists - No Changes)

```sql
CREATE TABLE organizations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

  -- Essentials
  name TEXT NOT NULL,
  description TEXT,  -- Everything narrative goes here
  website TEXT,

  -- Contact
  phone TEXT,
  email TEXT,

  -- Location
  primary_address TEXT,
  latitude FLOAT,
  longitude FLOAT,

  -- Type & verification
  organization_type TEXT CHECK (organization_type IN ('nonprofit', 'business', 'community', 'other')),
  verified BOOL DEFAULT false,
  verified_at TIMESTAMPTZ,

  -- Claiming
  claim_token TEXT UNIQUE,
  claim_email TEXT,
  claimed_at TIMESTAMPTZ,

  -- Timestamps
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

**No summary, no tldr, no embedding - just description.**

## Business Organizations (Minimal)

```sql
CREATE TABLE business_organizations (
  organization_id UUID PRIMARY KEY REFERENCES organizations(id) ON DELETE CASCADE,

  -- Cause-driven commerce (only if they share proceeds)
  proceeds_percentage DECIMAL(5,2) CHECK (proceeds_percentage >= 0 AND proceeds_percentage <= 100),
  proceeds_beneficiary_id UUID REFERENCES organizations(id),

  -- Support links (essential CTAs)
  donation_link TEXT,
  gift_card_link TEXT,
  online_store_url TEXT,

  created_at TIMESTAMPTZ DEFAULT NOW()
);
```

**That's it. 6 fields total.**

Everything else goes in:
- **description**: "Bailey Aro sells apparel where 15% of proceeds support immigrant legal aid. We're women-owned and ship nationally."
- **tags**: women_owned, cause_driven, immigrant_rights

## No Nonprofit Organizations Table

Don't create it. Nonprofits are just organizations with type='nonprofit'. If they need specific fields later, add them then.

## Comparison

### Before (Over-Engineered)
```sql
business_organizations (
  organization_id,
  business_type,              -- ❌ Remove
  founded_year,               -- ❌ Remove
  employee_count,             -- ❌ Remove
  proceeds_percentage,        -- ✅ Keep
  proceeds_beneficiary_id,    -- ✅ Keep
  proceeds_description,       -- ❌ Remove (use org.description)
  impact_statement,           -- ❌ Remove (use org.description)
  accepts_donations,          -- ❌ Remove (just check if donation_link exists)
  donation_link,              -- ✅ Keep
  donation_methods,           -- ❌ Remove (not essential)
  gift_cards_available,       -- ❌ Remove (just check if gift_card_link exists)
  gift_card_link,             -- ✅ Keep
  online_store_url,           -- ✅ Keep
  delivery_available,         -- ❌ Remove (not essential for v1)
  pickup_available,           -- ❌ Remove (not essential for v1)
  ships_nationally            -- ❌ Remove (not essential for v1)
)
-- 17 fields!

nonprofit_organizations (
  organization_id,
  ein,                        -- ❌ Don't create this table
  tax_exempt_status,          -- ❌ Not essential
  mission_statement,          -- ❌ Use org.description
  founded_year,               -- ❌ Not essential
  annual_budget_range         -- ❌ Not essential
)
```

### After (Minimal)
```sql
business_organizations (
  organization_id,
  proceeds_percentage,        -- ✅ Keep - need for queries
  proceeds_beneficiary_id,    -- ✅ Keep - relationship
  donation_link,              -- ✅ Keep - CTA
  gift_card_link,             -- ✅ Keep - CTA
  online_store_url            -- ✅ Keep - CTA
)
-- 6 fields total (including PK + timestamp)

-- NO nonprofit_organizations table
```

## Example: Bailey Aro

### Organization
```sql
INSERT INTO organizations (name, website, organization_type, description) VALUES (
  'Bailey Aro',
  'https://www.baileyaro.com/',
  'business',
  'We create sustainable apparel and accessories. 15% of all proceeds support immigrant families through Community Legal Aid Fund. Every purchase helps fund legal consultations for families navigating the immigration system. We ship nationwide and offer free returns.'
);
```

### Business Properties (Just Links + Proceeds)
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

### Tags
```sql
INSERT INTO taggables (tag_id, taggable_type, taggable_id) VALUES
  ((SELECT id FROM tags WHERE kind='ownership' AND value='women_owned'), 'organization', 'bailey-aro-uuid'),
  ((SELECT id FROM tags WHERE kind='business_model' AND value='cause_driven'), 'organization', 'bailey-aro-uuid'),
  ((SELECT id FROM tags WHERE kind='impact_area' AND value='immigrant_rights'), 'organization', 'bailey-aro-uuid');
```

## Query: Find Cause-Driven Businesses

```sql
SELECT
  o.name,
  o.description,
  o.website,
  bo.proceeds_percentage,
  beneficiary.name as beneficiary_name,
  bo.online_store_url
FROM organizations o
JOIN business_organizations bo ON o.id = bo.organization_id
LEFT JOIN organizations beneficiary ON bo.proceeds_beneficiary_id = beneficiary.id
WHERE bo.proceeds_percentage > 0
ORDER BY bo.proceeds_percentage DESC;
```

## Benefits

1. **Simple** - 6 fields instead of 17+
2. **Flexible** - Description field holds all narrative content
3. **No false precision** - Don't pretend we have structured data when we don't
4. **Easy to populate** - Scrapers just need to grab description, not parse into 5 different fields
5. **Less migration pain** - Fewer fields = fewer schema changes

## When to Add Fields

Only add fields when you need to:
1. **Query/filter on it** (proceeds_percentage - find businesses giving >10%)
2. **Create relationships** (proceeds_beneficiary_id - link to recipient org)
3. **Power CTAs** (donation_link, online_store_url - "Shop Here" buttons)

Otherwise, it goes in description or tags.
