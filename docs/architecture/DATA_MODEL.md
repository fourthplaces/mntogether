# MN Digital Aid - Data Model

## Core Architecture

The system uses a **minimal, flexible schema** with three key principles:
1. **Fields only for queries, relationships, and CTAs**
2. **Description field for narrative content** (not split into multiple fields)
3. **Tags for categorical metadata** (ownership, certifications, etc.)

## Entity-Relationship Diagram

```mermaid
erDiagram
    %% Core Organizations
    organizations ||--o{ business_organizations : "extends"
    organizations ||--o{ listings : "has many"
    organizations }o--o{ taggables : "tagged with"
    organizations ||--o| domains : "scraped from"

    %% Business Organizations
    organizations ||--o| business_organizations : "proceeds to"

    %% Domains & Scraping
    domains ||--o{ domain_scrape_urls : "has urls"
    domains ||--o{ listings : "scrapes"

    %% Listings (Base + Type-Specific)
    listings ||--o| service_listings : "extends"
    listings ||--o| opportunity_listings : "extends"
    listings ||--o| business_listings : "extends"
    listings ||--o{ listing_contacts : "has contacts"
    listings ||--o{ listing_delivery_modes : "has modes"
    listings }o--o{ taggables : "tagged with"
    listings ||--o{ posts : "announced as"
    listings ||--o| containers : "comments"

    %% Tags (Universal)
    tags ||--o{ taggables : "applied to"

    %% Members & Auth
    members ||--o{ identifiers : "authenticated by"
    members ||--o{ messages : "authors"

    %% Containers & Messages
    containers ||--o{ messages : "contains"
    containers ||--o{ referral_documents : "generates"

    %% Referral Documents
    referral_documents ||--o{ referral_document_translations : "translated to"
    referral_documents ||--o{ document_references : "references"

    %% Organizations
    organizations {
        uuid id PK
        text name "NOT NULL UNIQUE"
        text description "All narrative content"
        text website
        text phone
        text email
        text primary_address
        float latitude
        float longitude
        text organization_type "nonprofit|business|community|other"
        bool verified
        timestamptz verified_at
        text claim_token "For self-service claiming"
        text claim_email
        timestamptz claimed_at
        vector_1024 embedding "For semantic search"
        uuid domain_id FK
        timestamptz created_at
        timestamptz updated_at
    }

    %% Business Organizations (Minimal - 6 fields)
    business_organizations {
        uuid organization_id PK_FK "References organizations"
        decimal proceeds_percentage "0-100"
        uuid proceeds_beneficiary_id FK "References organizations"
        text donation_link "CTA"
        text gift_card_link "CTA"
        text online_store_url "CTA"
        timestamptz created_at
    }

    %% Domains
    domains {
        uuid id PK
        text domain_url "UNIQUE NOT NULL"
        int scrape_frequency_hours
        timestamptz last_scraped_at
        bool active
        timestamptz created_at
        timestamptz updated_at
    }

    %% Domain Scrape URLs
    domain_scrape_urls {
        uuid id PK
        uuid domain_id FK
        text url "Specific page to scrape"
        bool active
        timestamptz added_at
    }

    %% Listings (Base Table)
    listings {
        uuid id PK
        text listing_type "service|opportunity|business"
        text category "food|housing|healthcare|legal"
        text title
        text description
        text tldr
        text status "pending_approval|active|filled|rejected|expired"
        text capacity_status "accepting|paused|at_capacity"
        text urgency "low|medium|high|urgent"
        text submission_type "scraped|admin|org_submitted"
        uuid organization_id FK
        uuid domain_id FK
        uuid submitted_by_admin_id FK
        text source_language "Default: en"
        text location
        float latitude
        float longitude
        timestamptz verified_at
        text content_hash "For deduplication"
        vector_1024 embedding "For semantic search"
        timestamptz created_at
        timestamptz updated_at
    }

    %% Service Listings
    service_listings {
        uuid listing_id PK_FK
        bool requires_id "Fear constraint"
        bool contacts_authorities "Fear constraint"
        bool avoids_facility_visit "Fear constraint"
        bool remote_ok
    }

    %% Opportunity Listings
    opportunity_listings {
        uuid listing_id PK_FK
        text opportunity_type "volunteer|donation|customer|partnership"
        text time_commitment
        bool requires_background_check
        int minimum_age
        text_array skills_needed
        bool remote_ok
    }

    %% Business Listings
    business_listings {
        uuid listing_id PK_FK
        text business_type
        text_array support_needed
        text current_situation
        bool accepts_donations
        text donation_link
        bool gift_cards_available
        text gift_card_link
        bool remote_ok
        bool delivery_available
        text online_ordering_link
    }

    %% Listing Contacts
    listing_contacts {
        uuid id PK
        uuid listing_id FK
        text contact_type "phone|email|website|address"
        text contact_value
        text contact_label
        int display_order
    }

    %% Listing Delivery Modes
    listing_delivery_modes {
        uuid listing_id FK
        text delivery_mode "in_person|phone|online|mail|home_visit"
    }

    %% Tags (Universal)
    tags {
        uuid id PK
        text kind "community_served|service_area|ownership|certification"
        text value "women_owned|somali|minneapolis"
        text display_name "For UI"
        timestamptz created_at
    }

    %% Taggables (Polymorphic)
    taggables {
        uuid id PK
        uuid tag_id FK
        text taggable_type "listing|organization|referral_document|domain"
        uuid taggable_id "UUID of tagged entity"
        timestamptz added_at
    }

    %% Members (formerly volunteers)
    members {
        uuid id PK
        text expo_push_token "UNIQUE - anonymous identifier"
        text searchable_text "All capabilities, skills, interests"
        float latitude
        float longitude
        text location_name
        bool active
        int notification_count_this_week
        timestamptz paused_until
        vector_1024 embedding "For matching"
        timestamptz created_at
    }

    %% Identifiers (Phone Auth)
    identifiers {
        uuid id PK
        uuid member_id FK
        varchar_64 phone_hash "UNIQUE - hashed phone number"
        bool is_admin
        timestamptz created_at
        timestamptz updated_at
    }

    %% Posts (Temporal Announcements)
    posts {
        uuid id PK
        uuid listing_id FK "References listings"
        text status "draft|published|expired|archived"
        timestamptz published_at
        timestamptz expires_at
        text custom_title "Override listing title"
        text custom_description
        text custom_tldr
        jsonb targeting_hints
        int view_count
        int click_count
        int response_count
        uuid created_by "Admin who approved"
        timestamptz created_at
        timestamptz updated_at
        timestamptz last_displayed_at
    }

    %% Containers (Generalized Message Containers)
    containers {
        uuid id PK
        text container_type "ai_chat|listing_comments|org_discussion"
        uuid entity_id "Related entity ID (nullable)"
        text language
        timestamptz created_at
        timestamptz last_activity_at
    }

    %% Messages
    messages {
        uuid id PK
        uuid container_id FK
        text role "user|assistant|comment"
        text content
        uuid author_id FK "Nullable - for public comments"
        text moderation_status "approved|pending|flagged|removed"
        uuid parent_message_id FK "For threaded discussions"
        int sequence_number
        timestamptz created_at
        timestamptz updated_at
        timestamptz edited_at
    }

    %% Referral Documents
    referral_documents {
        uuid id PK
        uuid container_id FK
        text source_language FK
        text content "Markdown + JSX components"
        text slug "UNIQUE - human-readable URL"
        text title
        text status "draft|published|archived"
        text edit_token "UNIQUE - secret for editing"
        int view_count
        timestamptz last_viewed_at
        timestamptz created_at
        timestamptz updated_at
    }

    %% Referral Document Translations
    referral_document_translations {
        uuid id PK
        uuid document_id FK
        text language_code FK
        text content "Translated markdown"
        text title "Translated title"
        timestamptz translated_at
        text translation_model
    }

    %% Document References
    document_references {
        uuid id PK
        uuid document_id FK
        text reference_kind "listing|organization|contact"
        text reference_id "UUID of referenced entity"
        int display_order
        timestamptz referenced_at
    }
```

## Key Design Patterns

### 1. Minimal Business Organizations (6 fields only)

```sql
business_organizations (
  organization_id,           -- Links to base org
  proceeds_percentage,       -- For queries: "businesses giving >10%"
  proceeds_beneficiary_id,   -- Relationship to recipient org
  donation_link,             -- CTA button
  gift_card_link,            -- CTA button
  online_store_url          -- CTA button
)
```

**Everything else goes in:**
- `organizations.description` - narrative content (mission, story, impact)
- `tags` - categorical metadata (women_owned, b_corp, cause_driven)

### 2. Polymorphic Tagging System

```sql
-- Tag kinds
kind='ownership', value='women_owned'
kind='certification', value='b_corp'
kind='business_model', value='cause_driven'
kind='community_served', value='somali'
kind='service_area', value='minneapolis'
kind='population', value='seniors'

-- Applied to any entity
taggable_type='listing', taggable_id='...'
taggable_type='organization', taggable_id='...'
```

### 3. Type-Specific Listing Properties

**Base:** `listings` (shared fields)
**Type-specific:**
- `service_listings` - fear constraints (requires_id, contacts_authorities)
- `opportunity_listings` - volunteer properties (time_commitment, skills_needed)
- `business_listings` - economic solidarity (support_needed, donation_link)

### 4. Generalized Containers

**One table for all message containers:**
- `container_type='ai_chat'` - anonymous AI conversations
- `container_type='listing_comments'` - public comments on listings
- `container_type='org_discussion'` - organization discussions

## Example Queries

### Find Cause-Driven Businesses Supporting Immigrant Rights

```sql
SELECT
  o.name,
  o.description,
  bo.proceeds_percentage,
  bo.online_store_url
FROM organizations o
JOIN business_organizations bo ON o.id = bo.organization_id
JOIN taggables t ON t.taggable_id = o.id
JOIN tags ON tags.id = t.tag_id
  AND tags.kind = 'business_model'
  AND tags.value = 'cause_driven'
WHERE bo.proceeds_percentage > 0
ORDER BY bo.proceeds_percentage DESC;
```

### Find Women-Owned Businesses

```sql
SELECT o.*, bo.*
FROM organizations o
JOIN business_organizations bo ON o.id = bo.organization_id
JOIN taggables t ON t.taggable_id = o.id
JOIN tags ON tags.id = t.tag_id
  AND tags.kind = 'ownership'
  AND tags.value = 'women_owned';
```

### Find Active Service Listings in Minneapolis

```sql
SELECT l.*, sl.*
FROM listings l
JOIN service_listings sl ON l.id = sl.listing_id
JOIN taggables t ON t.taggable_id = l.id
JOIN tags ON tags.id = t.tag_id
  AND tags.kind = 'service_area'
  AND tags.value = 'minneapolis'
WHERE l.listing_type = 'service'
  AND l.status = 'active'
  AND l.capacity_status = 'accepting';
```

## Data Flow

### Scraping Pipeline
```
domains → domain_scrape_urls → listings (scraped) → listing_* tables
```

### Organization Self-Service
```
organizations.claim_token → organizations.claimed_at → capacity updates
```

### Posting Flow
```
listings (approved) → posts (published) → members (notified) → containers/messages
```

### Referral Generation
```
containers (ai_chat) → referral_documents → document_references → translations
```

## Privacy & Security

### Anonymous Members
- No PII stored
- Only `expo_push_token` for push notifications
- Approximate location from IP geolocation only

### Phone-Based Auth
- Hashed phone numbers in `identifiers.phone_hash`
- Never store plain phone numbers
- Links to `members` table

### Public Containers
- No auth required for viewing
- Anonymous commenting supported
- `edit_token` for document editing (secret, not auth)
