# Root Editorial — Data Model

## Core Architecture

The system uses a **minimal, flexible schema** with three key principles:
1. **Fields only for queries, relationships, and CTAs**
2. **Description field for narrative content** (not split into multiple fields)
3. **Tags for categorical metadata** (ownership, certifications, etc.)

## Key Entities

### Posts
The central content unit. Posts represent community-relevant content items.

```
posts
├── id, title, description, tldr
├── post_type (service, opportunity, business — expanding to 12+ types)
├── category, status, urgency
├── location, latitude, longitude
├── source_url, content_hash, embedding
└── created_at, updated_at
```

### Organizations
Community organizations that posts are associated with.

```
organizations
├── id, name, description
├── website, phone, email, primary_address
├── latitude, longitude
├── organization_type (nonprofit, business, community, other)
├── verified, verified_at
├── embedding
└── created_at, updated_at
```

### Sources
Content sources linked to organizations (websites, newsletters, etc.).

```
sources
├── id, organization_id
├── source_type (website, newsletter, etc.)
├── status, last_crawled_at
└── created_at
```

### Post→Organization Relationship

Posts do NOT have a direct FK to organizations. The relationship is:

```
posts → post_sources → sources → organizations
                       (has organization_id FK)
```

### Tags (Polymorphic)

Tags provide flexible categorization for any entity.

```
tags: (id, kind, value, display_name)
taggables: (tag_id, taggable_type, taggable_id)
tag_kinds: (slug, is_public) — controls visibility
```

Key tag kinds:
- `public` — User-visible badges (Donate, Volunteer, Food, Help)
- `post_type` — Filter tabs (offering, seeking, announcement)
- `service_offered` — Category dropdown (food-assistance, housing, legal-aid)

### Members & Auth

```
members
├── id, searchable_text
├── latitude, longitude, location_name
├── active, notification_count_this_week, paused_until
└── created_at

identifiers
├── id, member_id
├── phone_hash (SHA256 of phone or email — never plaintext)
├── is_admin
└── created_at
```

**Privacy**: No PII stored in members. Phone/email identifiers are hashed.

### Contacts & Schedules

```
listing_contacts: (id, listing_id, contact_type, contact_value, display_order)
schedules: (schedulable_type='post', schedulable_id, day_of_week, opens_at, closes_at, timezone)
```

### Notes (Editorial)

```
notes
├── id, body, note_type
├── notable_type, notable_id (polymorphic — attached to posts, orgs, etc.)
├── created_by
└── created_at
```

## Design Patterns

### 1. Polymorphic Tagging System

```sql
-- Tag kinds
kind='ownership', value='women_owned'
kind='certification', value='b_corp'
kind='community_served', value='somali'
kind='service_area', value='minneapolis'

-- Applied to any entity
taggable_type='post', taggable_id='...'
taggable_type='organization', taggable_id='...'
```

### 2. Extension Tables

Type-specific properties are stored in extension tables that join 1:1 with the base entity. Example: `business_organizations` extends `organizations` with proceeds_percentage, donation_link, etc.

### 3. Source-of-Truth for Schema

The canonical schema is defined by the migration files in `packages/server/migrations/`. See also [DATABASE_SCHEMA.md](DATABASE_SCHEMA.md) for a more complete reference.

## Data Flow

### Content Ingestion (future: Root Signal integration)
```
Root Signal API → posts (via sync/import) → editorial review → publish
```

### Editorial Flow
```
posts (pending) → admin review → posts (active) → public display
```

### Auth Flow
```
phone/email → Twilio OTP → identifiers (hashed) → JWT → admin access
```
