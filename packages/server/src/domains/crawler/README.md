# Multi-Type Listing Crawler

AI-powered web crawler that automatically discovers and extracts Services, Opportunities, and Business listings from websites.

## Architecture Overview

The crawler follows a **Seesaw event sourcing pattern** with clear separation between policy (business logic) and infrastructure (effects):

```
┌─────────────────────────────────────────────────────────────┐
│                     GraphQL/API Layer                        │
│              (submit_resource_link mutation)                 │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│                   Crawler Coordinator                        │
│         Routes events to state machines, collects            │
│         commands from machines                               │
└───────────────┬─────────────────────────────────────────────┘
                │
                ├──────► ResourceDiscoveryMachine
                │        (decides when to discover resources)
                │
                └──────► PageLifecycleMachine
                         (decides when to flag/extract pages)
                │
                ▼
┌─────────────────────────────────────────────────────────────┐
│                  Effect Executor                             │
│    Executes commands by running intelligent-crawler effects  │
└───────────┬─────────────────────────────────────────────────┘
            │
            ├──────► FlaggingEffect
            │        Uses MultiTypeListingEvaluator to flag pages
            │
            └──────► ExtractionEffect
                     Uses MultiTypeListingEvaluator to extract data
            │
            ▼
┌─────────────────────────────────────────────────────────────┐
│                   Listing Adapter                            │
│         Converts RawExtraction → Database Listings           │
│         Handles deduplication and type routing               │
└─────────────────────────────────────────────────────────────┘
```

## Components

### 1. Extraction Schemas (`extraction_schemas.rs`)

Defines the structure of AI-extracted data for each listing type.

**Envelope Pattern:**
```rust
enum ExtractedListingEnvelope {
    Service(ExtractedService),
    Opportunity(ExtractedOpportunity),
    Business(ExtractedBusiness),
}
```

Each type includes:
- **Core fields**: organization_name, title, description, tldr, location, etc.
- **Type-specific fields**: Based on the listing type

**Example Service Extraction:**
```json
{
  "listing_type": "service",
  "organization_name": "Legal Aid Society",
  "title": "Free Immigration Legal Services",
  "description": "We provide free legal consultation...",
  "tldr": "Free legal help for immigrants",
  "location": "Minneapolis, MN",
  "category": "legal",
  "confidence": "high",
  "free_service": true,
  "requires_identification": false,
  "interpretation_available": true
}
```

**Example Opportunity Extraction:**
```json
{
  "listing_type": "opportunity",
  "organization_name": "Food Bank",
  "title": "Weekend Food Sorters Needed",
  "description": "Help sort donations every Saturday...",
  "opportunity_type": "volunteer",
  "time_commitment": "4 hours per week",
  "remote_ok": false,
  "minimum_age": 16
}
```

**Example Business Extraction:**
```json
{
  "listing_type": "business",
  "organization_name": "Community Coffee",
  "title": "Coffee for a Cause",
  "description": "10% of proceeds support immigrants...",
  "proceeds_percentage": 10,
  "proceeds_beneficiary": "Immigrant Law Center",
  "online_store_url": "https://shop.example.com"
}
```

### 2. Listing Evaluator (`listing_evaluator.rs`)

Implements the `PageEvaluator` trait from intelligent-crawler to detect and extract listings using AI.

**Three-Step Evaluation:**

1. **Pre-filter** (fast heuristics):
   ```rust
   fn pre_filter(&self, url: &Url, content_snippet: &str) -> bool
   ```
   - Checks URL patterns (e.g., "/volunteer", "/services")
   - Checks content keywords (e.g., "donate", "help needed")
   - Avoids costly AI calls for irrelevant pages

2. **Flagging** (AI decision):
   ```rust
   async fn should_flag(&self, content: &PageContent) -> Result<FlagDecision>
   ```
   - AI determines if page contains any listing type
   - Returns confidence score (0.0-1.0)
   - Threshold: ≥ 0.4 to flag

3. **Extraction** (structured data):
   ```rust
   async fn extract_data(&self, content: &PageContent) -> Result<Vec<RawExtraction>>
   ```
   - AI extracts all listings from page
   - Auto-detects listing type
   - Returns type-specific fields
   - Handles multiple listings per page

**Prompt Injection Protection:**
- Sanitizes all user input before AI prompts
- Uses boundary markers to isolate user content
- Validates extracted data for suspicious patterns

### 3. Listing Adapter (`listing_adapter.rs`)

Converts `RawExtraction` (from intelligent-crawler) to database `Listing` records.

**Features:**
- **Type Routing**: Routes to type-specific handlers
- **Deduplication**: SHA256 fingerprint of (org_name + title)
- **Status Management**: All scraped listings start as `pending_approval`
- **Last Seen Tracking**: Updates `last_seen_at` for existing listings
- **Type-Specific Tables**: Inserts into service_listings, opportunity_listings, or business_organizations

**Processing Flow:**
```rust
process_extraction(RawExtraction) -> ListingId
    ├─ Parse JSON → ExtractedListingEnvelope
    ├─ Calculate fingerprint
    ├─ Check if listing exists
    │  ├─ If exists: Update last_seen_at
    │  └─ If new: Route to type handler
    │      ├─ process_service_extraction()
    │      ├─ process_opportunity_extraction()
    │      └─ process_business_extraction()
    └─ Return listing_id
```

### 4. Effect Executor (`effect_executor.rs`)

Glue layer that wires intelligent-crawler effects to domain adapters.

**Responsibilities:**
- Instantiates `FlaggingEffect` and `ExtractionEffect`
- Routes commands to appropriate effects
- Processes `DataExtracted` events through `ListingAdapter`
- Returns events to coordinator

**Usage:**
```rust
let executor = CrawlerEffectExecutor::new(
    ai_client,
    crawler_storage,
    listings_pool,
);

let commands = coordinator.process_event(&event);
let new_events = executor.execute_commands(commands).await?;
```

### 5. State Machines (`machines/`)

**ResourceDiscoveryMachine:**
- Receives: `ResourceSubmitted` event
- Decides: `DiscoverResource` command
- Tracks: Discovery status (Pending → Discovering → Completed/Failed)

**PageLifecycleMachine:**
- Receives: `PageDiscovered` → Decides: `FlagPage`
- Receives: `PageFlagged` → Decides: `ExtractFromPage`
- Receives: `PageContentChanged` → Decides: re-flag page
- Tracks: Page status through lifecycle

### 6. Coordinator (`coordinator.rs`)

Routes events to state machines and collects commands.

**Event Routing:**
- `AggregateKey::Resource` → ResourceDiscoveryMachine
- `AggregateKey::Page` → PageLifecycleMachine
- `AggregateKey::Extraction` → Terminal (no machine)

## Data Flow

### Complete End-to-End Flow

```
1. User submits URL via submit_resource_link mutation
   ↓
2. ResourceSubmitted event → ResourceDiscoveryMachine
   ↓ decides
3. DiscoverResource command → DiscoveryEffect (intelligent-crawler)
   ↓ executes
4. PageDiscovered events (one per page found)
   ↓
5. PageLifecycleMachine decides → FlagPage command
   ↓
6. FlaggingEffect executes
   ├─ Pre-filter (heuristics)
   ├─ AI evaluation (should_flag)
   └─ Emits: PageFlagged or PageUnflagged
   ↓
7. If PageFlagged → PageLifecycleMachine decides → ExtractFromPage
   ↓
8. ExtractionEffect executes
   ├─ Calls MultiTypeListingEvaluator.extract_data()
   ├─ AI extracts listings with type-specific fields
   └─ Emits: DataExtracted events (one per listing)
   ↓
9. Effect Executor processes DataExtracted
   ├─ Calls ListingAdapter.process_extraction()
   ├─ Deduplicates by fingerprint
   ├─ Routes to type-specific handler
   └─ Creates Listing + type-specific records
   ↓
10. Listing created with status: pending_approval
```

## Database Schema

### Core Listing Table

```sql
CREATE TABLE listings (
  id UUID PRIMARY KEY,
  organization_id UUID REFERENCES organizations(id),
  organization_name TEXT,
  title TEXT,
  description TEXT,
  tldr TEXT,
  listing_type TEXT, -- 'service' | 'opportunity' | 'business'
  category TEXT,
  status TEXT, -- 'pending_approval' | 'active' | 'filled' | 'rejected'
  submission_type TEXT, -- 'scraped' | 'admin' | 'org_submitted'
  source_url TEXT,
  content_hash TEXT UNIQUE, -- SHA256 fingerprint for deduplication
  last_seen_at TIMESTAMPTZ,
  disappeared_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ,
  updated_at TIMESTAMPTZ
);
```

### Type-Specific Tables

**service_listings:**
- accessibility flags (wheelchair_accessible, interpretation_available)
- cost model (free_service, sliding_scale_fees, accepts_insurance)
- delivery method (remote_available, in_person_available, home_visits_available)
- hours (evening_hours, weekend_hours)

**opportunity_listings:**
- opportunity_type (volunteer, donation, customer, partnership)
- requirements (time_commitment, requires_background_check, minimum_age)
- logistics (remote_ok, skills_needed[])

**business_organizations:**
- proceeds_percentage (0-100)
- proceeds_beneficiary_id (FK to organizations)
- support CTAs (donation_link, gift_card_link, online_store_url)

## API Usage

### Submit a URL for Crawling

```graphql
mutation SubmitResourceLink {
  submit_resource_link(
    input: {
      url: "https://example.org/volunteer"
      context: "Community volunteer opportunities"
      submitter_contact: "user@example.com"
    }
  ) {
    job_id
    status
  }
}
```

### Query Pending Listings (Admin)

```graphql
query PendingListings {
  query_listings(status: "pending_approval", limit: 50) {
    edges {
      node {
        id
        listing_type
        organization_name
        title
        description
        tldr
        source_url
        created_at
      }
    }
  }
}
```

### Approve a Listing (Admin)

```graphql
mutation ApproveListing {
  approve_listing(listing_id: "...") {
    id
    status # now "active"
  }
}
```

## Configuration

### AI Model Selection

Default: GPT-4 Turbo

To change, update `effect_executor.rs`:
```rust
let extraction_effect = ExtractionEffect::new(
    storage,
    extraction_evaluator,
    "v1.0.0".to_string(),
    "v1.0.0".to_string(),
    "gpt-4o".to_string(), // ← Change model here
);
```

### Confidence Threshold

Default: 0.4 (40%)

To change, update `intelligent-crawler/src/effects/flagging.rs`:
```rust
if decision.should_flag && decision.confidence >= 0.4 {
    // ↑ Adjust threshold
```

### Pre-filter Keywords

Edit `listing_evaluator.rs`:
```rust
let url_indicators = [
    "volunteer", "donate", "support", // Add more...
];

let content_indicators = [
    "help needed", "volunteers", // Add more...
];
```

## Testing

### Unit Tests

```bash
cargo test --package server -- crawler::
```

### Integration Tests (requires OpenAI API key)

```bash
OPENAI_API_KEY=sk-... cargo test --package server -- crawler:: --ignored
```

### Manual Testing

1. **Submit a test URL:**
   ```graphql
   mutation { submit_resource_link(input: {url: "https://example.org/volunteer"}) { job_id } }
   ```

2. **Check pending listings:**
   ```graphql
   query { query_listings(status: "pending_approval") { edges { node { title } } } }
   ```

3. **Approve a listing:**
   ```graphql
   mutation { approve_listing(listing_id: "...") { status } }
   ```

## Monitoring & Observability

### Key Metrics to Track

- **Pre-filter pass rate**: % of pages that pass heuristics
- **Flagging confidence**: Distribution of AI confidence scores
- **Extraction success rate**: % of flagged pages successfully extracted
- **Listing creation rate**: New listings per crawl session
- **Deduplication rate**: % of extractions that are duplicates

### Logging

All components emit structured logs:
- `listing_evaluator.rs`: Pre-filter decisions, AI flagging, extraction results
- `listing_adapter.rs`: Fingerprint collisions, listing creation, type routing
- `effect_executor.rs`: Command execution, adapter processing

View logs:
```bash
RUST_LOG=crawler=debug cargo run
```

## Troubleshooting

### Problem: AI extracts wrong listing type

**Solution:** Update the prompt in `listing_evaluator.rs` to provide better examples and type definitions.

### Problem: Too many false positives (irrelevant pages flagged)

**Solution:**
1. Tighten pre-filter keywords to be more specific
2. Increase confidence threshold from 0.4 to 0.6+

### Problem: Missing listings (false negatives)

**Solution:**
1. Loosen pre-filter to pass more pages to AI
2. Decrease confidence threshold to 0.3

### Problem: Duplicate listings created

**Solution:** Check fingerprint calculation in `listing_adapter.rs`. Ensure organization_name and title are being normalized correctly (lowercase, trimmed).

### Problem: AI prompt injection detected

**Solution:** The system already sanitizes inputs. Check logs for filtered content. If needed, strengthen sanitization in `listing_evaluator.rs`.

## Future Enhancements

- [ ] Add language detection for `source_language` field
- [ ] Implement domain_id and organization_id resolution
- [ ] Add confidence-based auto-approval for high-quality extractions
- [ ] Support image extraction for business logos
- [ ] Add webhook notifications for new pending listings
- [ ] Implement scheduled re-crawling for updated content
- [ ] Add admin UI for reviewing extraction quality

## Related Documentation

- [intelligent-crawler README](../../../intelligent-crawler/README.md) - Infrastructure layer
- [Listings Domain](../listings/README.md) - Listing models and GraphQL API
- [Seesaw Documentation](https://github.com/seesaw-rs/seesaw) - Event sourcing framework
