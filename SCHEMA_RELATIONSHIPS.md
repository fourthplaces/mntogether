# Schema Relationships Diagram

## Entity Relationship Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         ORGANIZATIONS                            │
│  (Base table - all organizations regardless of type)            │
│                                                                   │
│  • id, name, description, website                               │
│  • organization_type: nonprofit | business | community | other  │
│  • location, contact info, verification status                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ 1:1 (optional)
              ┌───────────────┴───────────────┐
              │                               │
              ▼                               ▼
┌──────────────────────────┐    ┌──────────────────────────┐
│ BUSINESS_ORGANIZATIONS   │    │ NONPROFIT_ORGANIZATIONS  │
│ (Business-specific)      │    │ (Nonprofit-specific)     │
│                          │    │                          │
│ • proceeds_percentage    │    │ • ein, tax_status       │
│ • proceeds_beneficiary ──┼────┼─→ Can point to any org  │
│ • donation_link          │    │ • mission_statement     │
│ • gift_cards_available   │    │ • annual_budget         │
│ • certified_b_corp       │    │ • charity_navigator     │
│ • online_store_url       │    │                          │
└──────────────────────────┘    └──────────────────────────┘
              │                               │
              │ 1:many                        │ 1:many
              │                               │
              └───────────────┬───────────────┘
                              │
                              ▼
            ┌─────────────────────────────────────┐
            │           LISTINGS                  │
            │  (What organizations offer)         │
            │                                     │
            │  • title, description, status       │
            │  • listing_type: service |          │
            │                  opportunity |      │
            │                  business           │
            │  • organization_id (FK)             │
            └─────────────────────────────────────┘
                              │
                              │ 1:1 (based on type)
              ┌───────────────┼───────────────┐
              │               │               │
              ▼               ▼               ▼
    ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
    │  SERVICE    │  │ OPPORTUNITY │  │  BUSINESS   │
    │  LISTINGS   │  │  LISTINGS   │  │  LISTINGS   │
    │             │  │             │  │             │
    │ • requires  │  │ • volunteer │  │ • product   │
    │   _id       │  │   _type     │  │   _category │
    │ • remote    │  │ • skills    │  │ • price     │
    │ • free      │  │ • time      │  │   _range    │
    └─────────────┘  └─────────────┘  └─────────────┘

            ┌─────────────────────────────────────┐
            │              TAGS                   │
            │  (Universal categorization)         │
            │                                     │
            │  • kind, value, display_name       │
            │  • business_model, impact_area,    │
            │    product_type, etc.              │
            └─────────────────────────────────────┘
                              │
                              │ many:many
                              ▼
            ┌─────────────────────────────────────┐
            │           TAGGABLES                 │
            │  (Polymorphic join)                 │
            │                                     │
            │  • tag_id (FK)                     │
            │  • taggable_type (org | listing)   │
            │  • taggable_id                     │
            └─────────────────────────────────────┘
```

## Example: Bailey Aro Case Study

```
┌────────────────────────────────────────────────────────┐
│ ORGANIZATION: Bailey Aro                                │
│ ├─ id: bailey-aro-uuid                                 │
│ ├─ name: "Bailey Aro"                                  │
│ ├─ organization_type: "business"                       │
│ ├─ website: "https://www.baileyaro.com/"              │
│ └─ description: "Apparel brand supporting immigrants"  │
└────────────────────────────────────────────────────────┘
                    │
                    │ has business properties
                    ▼
┌────────────────────────────────────────────────────────┐
│ BUSINESS_ORGANIZATION                                   │
│ ├─ organization_id: bailey-aro-uuid                    │
│ ├─ proceeds_percentage: 15.00                          │
│ ├─ proceeds_beneficiary_id: legal-aid-uuid ────────┐  │
│ ├─ proceeds_description: "15% supports families"    │  │
│ ├─ impact_statement: "Each purchase funds legal..."│  │
│ ├─ online_store_url: "https://baileyaro.com/"      │  │
│ ├─ ships_nationally: true                           │  │
│ └─ women_owned: true                                │  │
└────────────────────────────────────────────────────────┘
                    │                                   │
                    │ has listings                      │
                    ▼                                   │
        ┌──────────────────────────┐                   │
        │ LISTING 1                │                   │
        │ "Shop Apparel"           │                   │
        │ type: business           │                   │
        └──────────────────────────┘                   │
                    │                                   │
                    ▼                                   │
        ┌──────────────────────────┐                   │
        │ BUSINESS_LISTING         │                   │
        │ product_category: merch  │                   │
        │ price_range: "$$"        │                   │
        └──────────────────────────┘                   │
                                                        │
                    ┌──────────────────────────┐       │
                    │ LISTING 2                │       │
                    │ "Gift Cards"             │       │
                    │ type: business           │       │
                    └──────────────────────────┘       │
                                                        │
        ┌──────────────────────────┐                   │
        │ LISTING 3                │                   │
        │ "Volunteer at Pop-Ups"   │                   │
        │ type: opportunity        │                   │
        └──────────────────────────┘                   │
                    │                                   │
                    ▼                                   │
        ┌──────────────────────────┐                   │
        │ OPPORTUNITY_LISTING      │                   │
        │ opportunity_type: vol.   │                   │
        └──────────────────────────┘                   │
                                                        │
┌────────────────────────────────────────────────────────┐
│ ORGANIZATION: Community Legal Aid Fund          ◄──────┘
│ ├─ id: legal-aid-uuid                                 │
│ ├─ name: "Community Legal Aid Fund"                   │
│ ├─ organization_type: "nonprofit"                     │
│ └─ description: "Immigration legal services"          │
└────────────────────────────────────────────────────────┘
                    │
                    │ has nonprofit properties
                    ▼
┌────────────────────────────────────────────────────────┐
│ NONPROFIT_ORGANIZATION                                  │
│ ├─ organization_id: legal-aid-uuid                     │
│ ├─ ein: "12-3456789"                                   │
│ ├─ tax_exempt_status: "501c3"                          │
│ └─ mission_statement: "Provide legal aid to..."        │
└────────────────────────────────────────────────────────┘
```

## Tag Relationships

```
┌────────────────────────┐
│ TAG: cause_driven      │
│ kind: business_model   │
└────────────────────────┘
            │
            │ applied to
            ▼
┌────────────────────────┐         ┌────────────────────────┐
│ TAGGABLE               │         │ TAG: immigrant_rights  │
│ taggable_type: org     │◄────────│ kind: impact_area      │
│ taggable_id: bailey-aro│         └────────────────────────┘
└────────────────────────┘
            │
            │ also tagged with
            ▼
┌────────────────────────┐
│ TAG: apparel           │
│ kind: product_type     │
└────────────────────────┘
```

## Key Design Principles

### 1. Organization Properties vs Listing Properties

**Organization Level** (business_organizations)
- Applies to the entire organization
- Shared across all listings
- Examples: proceeds policy, certifications, donation links

**Listing Level** (business_listings)
- Specific to what this listing offers
- Can vary per listing
- Examples: product category, price range, appointment requirements

### 2. One Organization, Many Listings

Bailey Aro (1 organization) can have:
- "Shop Apparel" (business listing)
- "Buy Gift Cards" (business listing)
- "Volunteer at Events" (opportunity listing)
- "Donate Directly" (opportunity listing)

All listings inherit the organization's properties:
- 15% proceeds policy ✓
- Women-owned badge ✓
- Online store URL ✓

### 3. Organization-to-Organization Links

```
Bailey Aro ──proceeds_beneficiary_id──> Community Legal Aid
(business)                               (nonprofit)

Both are organizations in the system!
Both can have their own listings!
```

### 4. Type-Specific Extensions

```
organizations
├── organization_type = 'business' → business_organizations
├── organization_type = 'nonprofit' → nonprofit_organizations
├── organization_type = 'community' → (future) community_organizations
└── organization_type = 'other' → no extension table
```

### 5. Tagging Flexibility

**Tag Organizations** for:
- Business model (cause_driven, social_enterprise, b_corp)
- Impact areas (immigrant_rights, education, healthcare)
- Ownership (women_owned, minority_owned, lgbtq_owned)
- Leadership (community_led, immigrant_founded)

**Tag Listings** for:
- Product types (apparel, food, services)
- Service categories (legal, healthcare, housing)
- Opportunity types (volunteer, donation, mentorship)

## Query Patterns

### Get Organization with Full Context

```sql
SELECT
  o.*,
  bo.*,
  array_agg(DISTINCT l.*) as listings,
  array_agg(DISTINCT t.value) FILTER (WHERE t.kind = 'business_model') as business_models,
  array_agg(DISTINCT t.value) FILTER (WHERE t.kind = 'impact_area') as impact_areas
FROM organizations o
LEFT JOIN business_organizations bo ON o.id = bo.organization_id
LEFT JOIN listings l ON o.id = l.organization_id
LEFT JOIN taggables tg ON tg.taggable_id = o.id AND tg.taggable_type = 'organization'
LEFT JOIN tags t ON t.id = tg.tag_id
WHERE o.id = 'bailey-aro-uuid'
GROUP BY o.id, bo.organization_id;
```

### Find Impact Relationships

```sql
-- Which businesses support which nonprofits?
SELECT
  biz.name as business_name,
  bo.proceeds_percentage,
  np.name as nonprofit_name,
  np.organization_type
FROM organizations biz
JOIN business_organizations bo ON biz.id = bo.organization_id
JOIN organizations np ON bo.proceeds_beneficiary_id = np.id
WHERE bo.proceeds_percentage > 0
ORDER BY bo.proceeds_percentage DESC;
```

### Browse by Business Model

```sql
-- Find all B Corps that support education
SELECT DISTINCT o.*, bo.*
FROM organizations o
JOIN business_organizations bo ON o.id = bo.organization_id
JOIN taggables t1 ON t1.taggable_id = o.id AND t1.taggable_type = 'organization'
JOIN tags tag1 ON tag1.id = t1.tag_id AND tag1.kind = 'business_model' AND tag1.value = 'b_corp'
JOIN taggables t2 ON t2.taggable_id = o.id AND t2.taggable_type = 'organization'
JOIN tags tag2 ON tag2.id = t2.tag_id AND tag2.kind = 'impact_area' AND tag2.value = 'education'
WHERE bo.certified_b_corp = true;
```
