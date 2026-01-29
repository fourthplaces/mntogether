# Cause-Driven Commerce Architecture

## Overview

This document explains how the platform supports businesses where a percentage of proceeds go to charitable causes (e.g., Bailey Aro selling merchandise where 15% supports immigrant families).

## Architecture Components

### 1. Database Schema (Migration 000043)

#### Extended `business_listings` Table

```sql
ALTER TABLE business_listings ADD COLUMN
  proceeds_percentage DECIMAL(5,2),           -- e.g., 15.00
  proceeds_beneficiary_id UUID,              -- Links to organizations table
  proceeds_description TEXT,                 -- "15% of sales support..."
  impact_statement TEXT;                     -- "Your purchase funds..."
```

#### Universal Tagging System (Already Exists - Migration 000033)

```sql
-- Tags for discovery and categorization
tags (
  kind: 'business_model',     -- business_model tags
  value: 'cause_driven',      -- cause_driven, social_enterprise, b_corp, etc.
)

-- Tags for impact areas
tags (
  kind: 'impact_area',        -- What cause benefits?
  value: 'immigrant_rights'   -- immigrant_rights, legal_aid, education, etc.
)

-- Polymorphic join table
taggables (
  tag_id: UUID,
  taggable_type: 'listing',   -- Can tag listings, organizations, etc.
  taggable_id: UUID
)
```

### 2. Data Model (Rust)

#### BusinessListing Struct

```rust
pub struct BusinessListing {
    pub listing_id: ListingId,

    // Direct donations
    pub accepts_donations: bool,
    pub donation_link: Option<String>,

    // Gift cards
    pub gift_cards_available: bool,
    pub gift_card_link: Option<String>,

    // Cause-driven commerce (NEW)
    pub proceeds_percentage: Option<f64>,
    pub proceeds_beneficiary_id: Option<OrganizationId>,
    pub proceeds_description: Option<String>,
    pub impact_statement: Option<String>,

    // Commerce
    pub online_ordering_link: Option<String>,
    pub delivery_available: bool,
}
```

#### Key Methods

```rust
impl BusinessListing {
    // Check if business shares proceeds
    pub fn is_cause_driven(&self) -> bool {
        self.proceeds_percentage.is_some()
            && self.proceeds_percentage.unwrap() > 0.0
    }

    // Update proceeds allocation
    pub async fn update_proceeds(
        &mut self,
        proceeds_percentage: Option<f64>,
        beneficiary_id: Option<OrganizationId>,
        description: Option<String>,
        impact_statement: Option<String>,
        pool: &PgPool,
    ) -> Result<()>

    // Find all cause-driven businesses
    pub async fn find_cause_driven(pool: &PgPool) -> Result<Vec<Self>>
}
```

### 3. GraphQL Schema

#### Extended ListingType

```graphql
type ListingType {
  id: UUID!
  organizationName: String!
  title: String!
  description: String!
  listingType: String!  # 'service', 'opportunity', 'business'
  category: String!

  # Business-specific (only populated when listingType = 'business')
  businessInfo: BusinessInfo
}

type BusinessInfo {
  acceptsDonations: Boolean!
  donationLink: String
  giftCardsAvailable: Boolean!
  giftCardLink: String
  onlineOrderingLink: String
  deliveryAvailable: Boolean!

  # Cause-driven commerce
  proceedsPercentage: Float
  proceedsBeneficiaryId: UUID
  proceedsDescription: String
  impactStatement: String
}
```

## Example: Bailey Aro Business

### 1. Create Organizations

```sql
-- Beneficiary organization (where proceeds go)
INSERT INTO organizations (name, website, organization_type, verified)
VALUES (
  'Community Legal Aid',
  'https://example.org',
  'nonprofit',
  true
) RETURNING id; -- Returns: abc-123-legal-aid

-- Business organization
INSERT INTO organizations (name, website, organization_type)
VALUES (
  'Bailey Aro',
  'https://www.baileyaro.com/',
  'business'
) RETURNING id; -- Returns: xyz-789-bailey-aro
```

### 2. Create Listing

```sql
INSERT INTO listings (
  organization_id,
  listing_type,
  category,
  title,
  description,
  status
) VALUES (
  'xyz-789-bailey-aro',
  'business',
  'shopping',
  'Support Bailey Aro - Merchandise That Gives Back',
  'Shop apparel and accessories where 15% of proceeds directly support immigrant legal aid services.',
  'active'
) RETURNING id; -- Returns: listing-456
```

### 3. Add Business Properties

```sql
INSERT INTO business_listings (
  listing_id,
  online_ordering_link,
  proceeds_percentage,
  proceeds_beneficiary_id,
  proceeds_description,
  impact_statement
) VALUES (
  'listing-456',
  'https://www.baileyaro.com/',
  15.00,
  'abc-123-legal-aid',
  '15% of all sales support immigrant families',
  'Each purchase helps fund legal consultations for families navigating the immigration system'
);
```

### 4. Add Tags for Discovery

```sql
-- Tag as cause-driven business
INSERT INTO taggables (tag_id, taggable_type, taggable_id)
VALUES (
  (SELECT id FROM tags WHERE kind='business_model' AND value='cause_driven'),
  'listing',
  'listing-456'
);

-- Tag impact areas
INSERT INTO taggables (tag_id, taggable_type, taggable_id) VALUES
  ((SELECT id FROM tags WHERE kind='impact_area' AND value='immigrant_rights'), 'listing', 'listing-456'),
  ((SELECT id FROM tags WHERE kind='impact_area' AND value='legal_aid'), 'listing', 'listing-456');
```

## Query Examples

### Find All Cause-Driven Businesses

```sql
SELECT
  l.*,
  bl.proceeds_percentage,
  bl.proceeds_description,
  bl.impact_statement,
  bo.name as beneficiary_name
FROM listings l
JOIN business_listings bl ON l.id = bl.listing_id
LEFT JOIN organizations bo ON bl.proceeds_beneficiary_id = bo.id
WHERE bl.proceeds_percentage > 0
ORDER BY bl.proceeds_percentage DESC;
```

### Find Businesses Supporting Immigrant Rights

```sql
SELECT l.*, bl.*
FROM listings l
JOIN business_listings bl ON l.id = bl.listing_id
JOIN taggables t ON t.taggable_id = l.id AND t.taggable_type = 'listing'
JOIN tags tag ON tag.id = t.tag_id
WHERE tag.kind = 'impact_area'
  AND tag.value = 'immigrant_rights'
  AND l.listing_type = 'business';
```

### GraphQL Query

```graphql
query {
  listings(status: ACTIVE) {
    nodes {
      id
      title
      description
      listingType
      businessInfo {
        proceedsPercentage
        proceedsDescription
        impactStatement
        onlineOrderingLink
      }
    }
  }
}
```

## Frontend Display Example

### Business Card Component (React)

```tsx
function BusinessCard({ listing }) {
  const { businessInfo } = listing;

  if (!businessInfo?.proceedsPercentage) {
    return <StandardBusinessCard listing={listing} />;
  }

  return (
    <div className="business-card cause-driven">
      <div className="badge">
        🤝 {businessInfo.proceedsPercentage}% goes to charity
      </div>

      <h3>{listing.title}</h3>
      <p>{listing.description}</p>

      <div className="impact-section">
        <p className="proceeds-description">
          {businessInfo.proceedsDescription}
        </p>
        <p className="impact-statement">
          💡 {businessInfo.impactStatement}
        </p>
      </div>

      <a href={businessInfo.onlineOrderingLink} className="shop-button">
        Shop & Support
      </a>
    </div>
  );
}
```

## Use Cases Supported

### ✅ Direct Donations
- `accepts_donations: true`
- `donation_link: "https://paypal.me/..."`

### ✅ Gift Card Sales
- `gift_cards_available: true`
- `gift_card_link: "https://..."`

### ✅ Cause-Driven Commerce (NEW)
- `proceeds_percentage: 15.00`
- `proceeds_beneficiary_id: "org-uuid"`
- `proceeds_description: "15% supports..."`
- `impact_statement: "Your purchase funds..."`

### ✅ Tag-Based Discovery
- Filter by business model: `business_model=cause_driven`
- Filter by impact area: `impact_area=immigrant_rights`
- Find B Corps: `business_model=b_corp`

## Migration Path

1. **Run migration**: `000043_add_cause_commerce_support.sql`
2. **Update Rust models**: Add `BusinessListing` (already created)
3. **Update GraphQL**: Extend `ListingType` with `business_info`
4. **Update queries**: Join `business_listings` table when fetching listings
5. **Seed tags**: Business model and impact area tags (included in migration)
6. **Update frontend**: Add cause-driven business UI components

## Benefits of This Design

1. **Flexible**: Supports multiple business models (donations, gift cards, proceeds)
2. **Transparent**: Clear percentage and impact statements
3. **Discoverable**: Tags enable filtering by business model and impact area
4. **Relational**: Links businesses to beneficiary organizations
5. **Extensible**: Easy to add more fields (e.g., impact metrics, donation totals)
6. **Backward Compatible**: Doesn't break existing business listings

## Future Enhancements

- **Impact tracking**: Track total donations generated over time
- **Verification badges**: Mark verified cause relationships
- **Impact reports**: Show cumulative impact per business
- **Multiple beneficiaries**: Support splitting proceeds across multiple orgs
- **Time-bound campaigns**: Proceeds for limited time periods
