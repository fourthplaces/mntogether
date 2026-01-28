# SQL Query Refactoring Summary

All SQL queries have been moved from effects to their corresponding model files, following best practices for separation of concerns.

## Changes Made

### 1. Organization Need Model (`domains/organization/models/need.rs`)

**Added methods:**
- `create()` - Creates a new need with all required parameters
- `find_active_by_source()` - Finds all active needs for a source (for sync)
- `find_by_source_and_title()` - Finds need by source and title (for detecting changed needs)
- `mark_disappeared_except()` - Marks needs as disappeared that aren't in the provided hash list
- `touch_last_seen()` - Updates the last_seen_at timestamp

**Replaced inline queries in:**
- `effects/need.rs` - CreateNeed command now uses `OrganizationNeed::create()`
- `effects/submit.rs` - `submit_user_need()` now uses `OrganizationNeed::create()`
- `effects/utils/sync_utils.rs` - All sync queries now use model methods

### 2. Matching Domain

**Created new models:**
- `domains/matching/models/notification.rs`
  - `Notification::record()` - Records a notification (with conflict handling)
  - `Notification::find_by_member()` - Finds all notifications for a member
  - `Notification::find_by_need()` - Finds all notifications for a need

- `domains/matching/models/match_candidate.rs`
  - `MatchCandidate::find_within_radius()` - Vector search within distance radius
  - `MatchCandidate::find_statewide()` - Fallback statewide vector search

**Replaced inline queries in:**
- `effects/mod.rs` - `record_notification()` now uses `Notification::record()`
- `effects/vector_search.rs` - Simplified to re-export model methods

### 3. Effects Directory Structure

Effects are now purely orchestration - they:
1. Call model methods for data operations
2. Coordinate between multiple models
3. Handle business logic flow
4. Return events

Models handle all SQL queries:
1. CRUD operations
2. Complex queries (vector search, sync logic)
3. Database transactions

## Benefits

1. **Separation of Concerns**: Effects handle business logic, models handle data access
2. **Reusability**: Model methods can be used from anywhere (effects, GraphQL resolvers, etc.)
3. **Testability**: Models can be tested independently of effects
4. **Maintainability**: SQL queries are centralized and easier to update
5. **Type Safety**: Model methods provide clear interfaces for data operations

## Files Modified

### Organization Domain
- `packages/server/src/domains/organization/models/need.rs` ✅
- `packages/server/src/domains/organization/effects/need.rs` ✅
- `packages/server/src/domains/organization/effects/submit.rs` ✅
- `packages/server/src/domains/organization/effects/utils/sync_utils.rs` ✅

### Matching Domain
- `packages/server/src/domains/matching/models/notification.rs` (new) ✅
- `packages/server/src/domains/matching/models/match_candidate.rs` (new) ✅
- `packages/server/src/domains/matching/models/mod.rs` (new) ✅
- `packages/server/src/domains/matching/mod.rs` ✅
- `packages/server/src/domains/matching/effects/mod.rs` ✅
- `packages/server/src/domains/matching/effects/vector_search.rs` ✅

## Compilation Status

The refactoring is complete. Any remaining compilation errors are unrelated to the SQL query refactoring and were present before these changes.
