# GraphQL Integration - Business Organizations

## ✅ Completed

### Schema Added
The `OrganizationData` type now includes business information:

```graphql
type OrganizationData {
  id: String!
  name: String!
  description: String
  contactInfo: ContactInfo
  location: String
  verified: Boolean!
  createdAt: String!
  updatedAt: String!
  tags: [TagData!]!
  sources: [SourceData!]!

  # NEW: Business organization properties
  businessInfo: BusinessOrganizationData
}

type BusinessOrganizationData {
  proceedsPercentage: Float
  proceedsBeneficiaryId: String
  donationLink: String
  giftCardLink: String
  onlineStoreUrl: String
  isCauseDriven: Boolean!
}
```

### Rust Implementation
- ✅ `BusinessOrganizationData` struct created
- ✅ `From<BusinessOrganization>` converter
- ✅ GraphQL object impl with all fields
- ✅ `business_info()` async resolver on `OrganizationData`
- ✅ `is_cause_driven()` helper method

## Example Queries

### Get Organization with Business Info
```graphql
query GetOrganization($id: String!) {
  organization(id: $id) {
    id
    name
    description
    verified

    businessInfo {
      proceedsPercentage
      proceedsBeneficiaryId
      donationLink
      giftCardLink
      onlineStoreUrl
      isCauseDriven
    }

    tags {
      kind
      value
    }
  }
}
```

### Find Cause-Driven Businesses
```graphql
query FindCauseDrivenBusinesses {
  organizations(limit: 100) {
    id
    name
    description

    businessInfo {
      proceedsPercentage
      onlineStoreUrl
      isCauseDriven
    }

    tags {
      kind
      value
    }
  }
}
```

Then filter client-side for `businessInfo.isCauseDriven === true`.

### Get Organization with Beneficiary Details
```graphql
query GetBusinessWithBeneficiary($id: String!) {
  organization(id: $id) {
    name
    description

    businessInfo {
      proceedsPercentage
      proceedsBeneficiaryId
      onlineStoreUrl
    }
  }
}

# Then make second query for beneficiary:
query GetBeneficiary($id: String!) {
  organization(id: $id) {
    name
    description
  }
}
```

**Note**: Currently requires 2 queries. Could add a `beneficiary` field to `BusinessOrganizationData` that resolves the full organization.

## Frontend Usage

### TypeScript Types
```typescript
interface Organization {
  id: string;
  name: string;
  description?: string;
  verified: boolean;
  businessInfo?: BusinessInfo;
  tags: Tag[];
}

interface BusinessInfo {
  proceedsPercentage?: number;
  proceedsBeneficiaryId?: string;
  donationLink?: string;
  giftCardLink?: string;
  onlineStoreUrl?: string;
  isCauseDriven: boolean;
}

interface Tag {
  kind: string;
  value: string;
}
```

### React Component Example
```tsx
function OrganizationCard({ org }: { org: Organization }) {
  const { businessInfo, tags } = org;

  // Check if cause-driven
  const isCauseDriven = businessInfo?.isCauseDriven;

  // Get ownership tags
  const ownershipTags = tags.filter(t => t.kind === 'ownership');

  return (
    <div className="org-card">
      <h3>{org.name}</h3>
      <p>{org.description}</p>

      {/* Cause-driven badge */}
      {isCauseDriven && businessInfo && (
        <div className="badge cause-driven">
          🤝 {businessInfo.proceedsPercentage}% goes to charity
        </div>
      )}

      {/* Ownership badges */}
      {ownershipTags.map(tag => (
        <span key={tag.value} className="badge">
          {tag.value === 'women_owned' && '👩'}
          {tag.value === 'minority_owned' && '🌍'}
          {tag.value === 'lgbtq_owned' && '🏳️‍🌈'}
          {formatTagLabel(tag.value)}
        </span>
      ))}

      {/* CTAs */}
      <div className="actions">
        {businessInfo?.onlineStoreUrl && (
          <a href={businessInfo.onlineStoreUrl} className="btn-primary">
            Shop & Support
          </a>
        )}
        {businessInfo?.donationLink && (
          <a href={businessInfo.donationLink} className="btn-secondary">
            Donate
          </a>
        )}
        {businessInfo?.giftCardLink && (
          <a href={businessInfo.giftCardLink} className="btn-secondary">
            Buy Gift Card
          </a>
        )}
      </div>
    </div>
  );
}
```

### Filter Cause-Driven Businesses
```typescript
function useCauseDrivenBusinesses(organizations: Organization[]) {
  return organizations.filter(org => org.businessInfo?.isCauseDriven);
}

// With sorting by percentage
function useSortedCauseDrivenBusinesses(organizations: Organization[]) {
  return organizations
    .filter(org => org.businessInfo?.isCauseDriven)
    .sort((a, b) => {
      const aPercent = a.businessInfo?.proceedsPercentage ?? 0;
      const bPercent = b.businessInfo?.proceedsPercentage ?? 0;
      return bPercent - aPercent; // Descending
    });
}
```

## Next Steps

### 1. Add Beneficiary Resolver (Optional)
Instead of requiring 2 queries, add beneficiary field:

```rust
// In BusinessOrganizationData
async fn beneficiary(
    &self,
    context: &GraphQLContext,
) -> juniper::FieldResult<Option<OrganizationData>> {
    if let Some(beneficiary_id) = &self.proceeds_beneficiary_id {
        let org_id = OrganizationId::parse(beneficiary_id)?;
        let org = Organization::find_by_id(org_id, &context.db_pool).await?;
        Ok(Some(OrganizationData::from(org)))
    } else {
        Ok(None)
    }
}
```

Then query like:
```graphql
query {
  organization(id: "...") {
    businessInfo {
      proceedsPercentage
      beneficiary {
        name
        description
      }
    }
  }
}
```

### 2. Add Search/Filter Mutations
```graphql
query SearchCauseDrivenBusinesses(
  $minProceedsPercentage: Float,
  $tags: [String!]
) {
  # Server-side filtering
}
```

### 3. Frontend Admin Panel
- Form to set proceeds_percentage
- Picker to select beneficiary organization
- Input fields for donation/gift card/store URLs
- Tag selector for ownership, certifications

## Compilation Status

⚠️ **Note**: There are pre-existing compilation errors in other parts of the codebase (chatrooms, listings effects) from migration 000046. These are **not related** to the business_organizations integration.

The business_organizations GraphQL code compiles correctly. The errors are:
- `chatrooms/models/chatroom.rs` - ContainerId conversion issues
- `listings/effects/listing.rs` - Actor::new() arity mismatch

These need to be fixed separately.
