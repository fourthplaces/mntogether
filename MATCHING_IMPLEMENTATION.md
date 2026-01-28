# Member Matching Implementation - Complete

**Date**: 2026-01-27
**Status**: ✅ Implemented, Ready for Testing

## Overview

Implemented location-based member matching using distance filtering + vector similarity ranking. Members register with city/state, system geocodes to lat/lng, and matches within 30km radius when needs are approved.

---

## What Was Built

### 1. Database Schema

**Migration 13: `20260127000013_add_location_coordinates.sql`**
- Renamed `volunteers` → `members`
- Added `latitude`, `longitude`, `location_name` to members/organizations/sources
- Added `haversine_distance()` SQL function for distance calculations
- Indexes and constraints for spatial queries

**Migration 14: `20260127000014_create_notifications.sql`**
- Tracks which members were notified about which needs
- Fields: `need_id`, `member_id`, `why_relevant`, `clicked`, `responded`
- Unique constraint prevents duplicate notifications

### 2. Geocoding Utility (`src/common/utils/geocoding.rs`)

```rust
// Geocode city/state → lat/lng using Nominatim (OpenStreetMap)
geocode_city("Minneapolis", "MN") → (44.98, -93.27, "Minneapolis, MN")

// Privacy: Round to 2 decimal places (city-level, not block-level)
coarsen_coords(44.977753, -93.265011) → (44.98, -93.27)

// Distance calculation
calculate_distance_km(lat1, lng1, lat2, lng2) → distance in km
```

**Features:**
- Free (uses Nominatim API, no API key needed)
- Privacy-preserving (coarse coordinates only)
- Comprehensive tests

### 3. Member Domain (`src/domains/member/`)

**Full layered architecture:**
```
member/
├── models/member.rs          ✅ SQL persistence
├── data/member.rs           ✅ GraphQL types
├── events/mod.rs            ✅ RegisterMemberRequested, MemberRegistered
├── commands/mod.rs          ✅ RegisterMember, UpdateMemberStatus
├── machines/mod.rs          ✅ State machine
├── effects/mod.rs           ✅ Registration with geocoding
└── edges/
    ├── query.rs             ✅ get_member, get_members
    └── mutation.rs          ✅ register_member, update_member_status
```

**Key Features:**
- Text-first: `searchable_text` is source of truth
- Privacy-first: Only stores coarse location, no PII
- Auto-geocoding: City/state input → lat/lng automatically
- Throttling: `notification_count_this_week` (max 3)

**GraphQL API:**
```graphql
mutation RegisterMember {
  registerMember(
    expoPushToken: "ExponentPushToken[xyz]"
    searchableText: "Can drive, Spanish speaker, legal aid volunteer"
    city: "Minneapolis"
    state: "MN"
  ) {
    id
    latitude
    longitude
    locationName
  }
}

query GetMembers {
  members {
    id
    searchableText
    locationName
    notificationCountThisWeek
  }
}
```

### 4. Matching Domain (`src/domains/matching/`)

**Full implementation:**
```
matching/
├── commands/mod.rs          ✅ FindMatches (background job)
├── events/mod.rs            ✅ MatchesFound, NoMatchesFound
├── machines/mod.rs          ✅ Matching state machine
└── effects/
    ├── vector_search.rs     ✅ Distance-filtered vector search
    └── mod.rs               ✅ Full matching pipeline
```

**Matching Pipeline:**
```
Need Approved
    ↓
dispatch_request(FindMatchesRequested)
    ↓
1. Get need + embedding
2. Vector search with distance filter (30km radius)
    SQL: WHERE haversine_distance() <= 30 AND similarity DESC
3. AI relevance check (placeholder: similarity > 0.6)
4. Throttle check (atomic: notification_count < 3)
5. Send notifications (top 5 members)
6. Record in notifications table
    ↓
MatchesFound (or NoMatchesFound)
```

**SQL Query:**
```sql
SELECT m.id, m.expo_push_token, m.searchable_text,
       1 - (m.embedding <=> $1) AS similarity,
       haversine_distance($2, $3, m.latitude, m.longitude) AS distance_km
FROM members m
WHERE m.active = true
  AND m.latitude IS NOT NULL
  AND m.longitude IS NOT NULL
  AND m.embedding IS NOT NULL
  AND m.notification_count_this_week < 3
  AND haversine_distance($2, $3, m.latitude, m.longitude) <= $4
ORDER BY similarity DESC
LIMIT 20
```

**Fallback:** If organization has no location, searches statewide (no distance filter).

### 5. Integration Points

**Server Wiring (`src/server/app.rs`):**
```rust
// Registered machines + effects
.with_machine(MemberMachine::new())
.with_effect::<MemberCommand, _>(RegistrationEffect)
.with_machine(MatchingMachine::new())
.with_effect::<MatchingCommand, _>(MatchingEffect)
```

**GraphQL Schema (`src/server/graphql/schema.rs`):**
- Added `member(id)` and `members()` queries
- Added `registerMember()` and `updateMemberStatus()` mutations

**Trigger on Approval (`src/domains/organization/edges/mutation.rs`):**
```rust
// In approve_need() and edit_and_approve_need()
dispatch_request(
    MatchingEvent::FindMatchesRequested { need_id },
    &ctx.bus,
    |m| { /* await completion */ }
).await;
```

**Seesaw-compliant:** Uses `dispatch_request` only, no direct `bus.emit()` in edges.

---

## Architecture Principles Followed

✅ **Location is a FILTER, not a ranking signal**
- Filter by 30km radius first
- Then rank by embedding similarity
- No distance boosting or geo math

✅ **Simple fallback logic**
- Has location → filter by distance
- No location → search statewide

✅ **Text-first storage**
- `searchable_text` is source of truth
- Anti-fragile, evolvable

✅ **Privacy-preserving**
- Coarse coordinates (2 decimal places ≈ 1km)
- No exact addresses

✅ **Generous relevance threshold**
- Bias toward recall, not precision
- Better to over-notify than under-notify

✅ **Seesaw-compliant**
- Edges only use `dispatch_request`
- No direct event emission
- Proper layering (models/data/events/commands/machines/effects/edges)

---

## What Still Needs to Be Done

### 1. Generate Embeddings

**For Members:**
```rust
// Background job after registration
use rig_core::embeddings::EmbeddingsBuilder;

let embedding = embeddings_builder
    .simple_document(&member.searchable_text)
    .await?;

// Update member
UPDATE members SET embedding = $1 WHERE id = $2
```

**For Needs:**
```rust
// After need approval/creation
let embedding = embeddings_builder
    .simple_document(&need.searchable_text)
    .await?;

UPDATE organization_needs SET embedding = $1 WHERE id = $2
```

### 2. Implement AI Relevance Check

Replace placeholder in `src/domains/matching/effects/mod.rs`:

```rust
async fn check_relevance(
    need: &OrganizationNeed,
    candidate: &MatchCandidate,
) -> Result<(bool, String)> {
    // Use GPT-4o to evaluate relevance
    let prompt = format!(
        "Is this member relevant for this need?\n\
         Need: {}\n\
         Member: {}\n\
         Respond with: YES|NO followed by one sentence explanation.",
        need.description,
        candidate.searchable_text
    );

    // Call OpenAI via rig.rs
    // ...

    Ok((is_relevant, explanation))
}
```

### 3. Implement Expo Push Notifications

Add dependency:
```toml
expo-push-notification-client = "0.6"
```

Send notifications in `src/domains/matching/effects/mod.rs`:
```rust
async fn send_push_notification(
    candidate: &MatchCandidate,
    need: &OrganizationNeed,
    why_relevant: &str,
) -> Result<()> {
    let message = ExpoMessage::builder(&candidate.expo_push_token)
        .title("Thought you might be interested")
        .body(format!("{} - {}", need.organization_name, need.title))
        .data("need_id", need.id.to_string())
        .data("why_relevant", why_relevant)
        .build()?;

    expo_client.send_push_notifications(vec![message]).await?;
    Ok(())
}
```

### 4. Weekly Notification Reset Job

Add to `src/kernel/scheduled_tasks.rs`:
```rust
// Every Monday at midnight
scheduler.add(
    Job::new_async("0 0 * * MON", |_uuid, _lock| {
        Box::pin(async move {
            Member::reset_weekly_counts(&pool).await?;
            Ok(())
        })
    })?
)?;
```

### 5. Add ServerDeps for Expo Client

Update `src/domains/organization/effects/command_effects.rs`:
```rust
pub struct ServerDeps {
    pub db_pool: PgPool,
    pub firecrawl_client: FirecrawlClient,
    pub need_extractor: NeedExtractor,
    pub expo_client: ExpoClient,  // Add this
}
```

### 6. Testing Plan

**Unit Tests:**
- ✅ Geocoding (tests already written)
- ✅ Haversine distance (tests already written)
- TODO: Member registration flow
- TODO: Matching pipeline

**Integration Tests:**
1. Register member with city/state → verify lat/lng stored
2. Approve need → verify matching triggered
3. Check notifications table → verify records created
4. Verify throttling → max 3 notifications per week

**Manual Testing:**
```graphql
# 1. Register a member
mutation {
  registerMember(
    expoPushToken: "test-token"
    searchableText: "Can drive, Spanish speaker"
    city: "Minneapolis"
    state: "MN"
  ) {
    id
    latitude
    longitude
  }
}

# 2. Approve a need (triggers matching)
mutation {
  approveNeed(needId: "uuid-here") {
    id
    status
  }
}

# 3. Check notifications
query {
  # TODO: Add notifications query
}
```

---

## Configuration Needed

**Environment Variables:**
```bash
# Already configured
DATABASE_URL=postgresql://...
OPENAI_API_KEY=sk-...

# TODO: Add for Expo push notifications
EXPO_ACCESS_TOKEN=...  # Optional, for higher rate limits
```

---

## Key Files Changed/Created

**New Files:**
- `migrations/20260127000013_add_location_coordinates.sql`
- `migrations/20260127000014_create_notifications.sql`
- `src/common/utils/geocoding.rs`
- `src/domains/member/` (entire domain - 9 files)
- `src/domains/matching/` (entire domain - 5 files)

**Modified Files:**
- `src/domains/mod.rs` - Added member + matching domains
- `src/server/app.rs` - Registered machines + effects
- `src/server/graphql/schema.rs` - Added member queries/mutations
- `src/domains/organization/edges/mutation.rs` - Trigger matching on approval
- `Cargo.toml` - Added `urlencoding` dependency

---

## Performance Characteristics

**Expected Query Times:**
- Geocoding API call: ~200-500ms (cached by Nominatim)
- Vector search (20 candidates): ~5-20ms (with proper indexes)
- AI relevance check (5 members): ~1-2s (parallel GPT-4o calls)
- Total matching time: ~2-3s per approved need

**Scalability:**
- Current approach: Good for <10K members
- With proper indexes: Good for <100K members
- For >100K: Consider PostGIS + spatial indexes

**Database Indexes Created:**
- `idx_members_lat`, `idx_members_lng` - Spatial filtering
- `idx_members_token` - Member lookup
- `idx_notifications_member`, `idx_notifications_need` - Analytics

---

## Next Steps

1. ✅ Run migrations: `sqlx migrate run`
2. TODO: Implement embedding generation (background job)
3. TODO: Implement AI relevance check (replace placeholder)
4. TODO: Implement Expo push notifications
5. TODO: Add weekly notification reset job
6. TODO: Write integration tests
7. TODO: Manual testing with real data

---

## Questions / Decisions Needed

1. **Embedding Model**: Use `text-embedding-3-small` (default) or `text-embedding-3-large`?
2. **Notification Radius**: Keep 30km or make configurable per-need?
3. **Relevance Threshold**: Keep generous (>0.6) or adjust based on real data?
4. **Max Notifications**: Keep 3/week or make configurable?
5. **Statewide Fallback**: Always search statewide if no local matches, or require location?

---

## Architecture Diagram

```
Member Registration:
User Input (city/state)
    ↓
Edge: dispatch_request(RegisterMemberRequested)
    ↓
Machine: decide() → RegisterMember command
    ↓
Effect: geocode_city() + insert DB
    ↓
Event: MemberRegistered
    ↓
Edge: return MemberData

Matching Flow:
Need Approved
    ↓
Edge: dispatch_request(FindMatchesRequested)
    ↓
Machine: decide() → FindMatches command
    ↓
Effect:
  1. Vector search (distance filtered)
  2. AI relevance check
  3. Throttle check
  4. Send notifications
  5. Record in DB
    ↓
Event: MatchesFound
    ↓
Edge: (ignores result, approval already completed)
```

---

**Implementation Complete** ✅
**Ready for Embedding Generation + Testing** 🚀
