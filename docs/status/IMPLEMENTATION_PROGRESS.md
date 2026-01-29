# Implementation Progress

## ✅ Completed

### Task 1: Seed Script
- Created `packages/server/src/bin/seed_organizations.rs`
- Parses JSON and uses OpenAI to extract tags
- Creates organizations with service/language/community tags
- **Run with**: `cd packages/server && cargo run --bin seed_organizations`

### Task 2: Embedding Generation (In Progress - 70% Complete)
**Completed:**
- ✅ Created migration `20260127000015_add_embeddings.sql`
- ✅ Created `EmbeddingService` utility in `common/utils/embeddings.rs`
- ✅ Added `GenerateEmbedding` command to `MemberCommand`
- ✅ Added `EmbeddingGenerated/EmbeddingFailed` events to `MemberEvent`
- ✅ Updated `MemberMachine` to emit `GenerateEmbedding` after registration
- ✅ Implemented effect in `member/effects/mod.rs`
- ✅ Added `update_embedding()` method to `Member` model
- ✅ Added `embedding_service` to `ServerDeps`
- ✅ Added `GenerateNeedEmbedding` command to `OrganizationCommand`

**Remaining:**
- Add `EmbeddingGenerated/EmbeddingFailed` events to `OrganizationEvent`
- Update `OrganizationMachine` to emit `GenerateNeedEmbedding` after need approval
- Implement effect handler in `organization/effects/command_effects.rs`
- Add `update_embedding()` method to `OrganizationNeed` model
- Wire up `EmbeddingService` in `server/app.rs` ServerDeps initialization

## 🚧 Pending Tasks

### Task 3: Expo Push Notifications
**Need to:**
- Add `expo-push-notification-client` dependency to Cargo.toml
- Create `ExpoClient` wrapper in `common/utils/expo.rs`
- Add `expo_client` to `ServerDeps`
- Implement `send_push_notification()` in `matching/effects/mod.rs`
- Wire up in `server/app.rs`

### Task 4: AI Relevance Check
**Need to:**
- Replace similarity threshold in `matching/effects/mod.rs:check_relevance()`
- Use GPT-4o to evaluate member relevance
- Return (bool, String) for relevance + explanation

### Task 5: Organization GraphQL API
**Need to:**
- Create `organization/data/organization.rs` with GraphQL types
- Create `organization/edges/query.rs` with queries
- Add mutations to `organization/edges/mutation.rs`
- Wire up in `server/graphql/schema.rs`

### Task 6: Weekly Reset Job
**Need to:**
- Add job to `kernel/scheduled_tasks.rs`
- Call `Member::reset_weekly_counts()` every Monday

## Next Steps

1. **Finish Task 2** (Embedding Generation)
   - Add organization events/machine/effect for need embeddings
   - Wire up EmbeddingService in app.rs

2. **Run migrations**
   ```bash
   cd packages/server
   sqlx migrate run
   ```

3. **Test seed script**
   ```bash
   cargo run --bin seed_organizations
   ```

4. **Implement Task 3** (Expo notifications)

5. **Implement Task 4** (AI relevance check)

6. **Test end-to-end flow**
   - Register member → embedding generated
   - Approve need → embedding generated → matching triggered → notifications sent

## Files Created/Modified

### New Files:
- `migrations/20260127000015_add_embeddings.sql`
- `src/common/utils/embeddings.rs`
- `src/bin/seed_organizations.rs`
- `data/README.md`
- `IMPLEMENTATION_PROGRESS.md` (this file)

### Modified Files:
- `Cargo.toml` - Added seed_organizations binary
- `src/common/utils/mod.rs` - Exported embeddings
- `src/domains/member/commands/mod.rs` - Added GenerateEmbedding
- `src/domains/member/events/mod.rs` - Added embedding events
- `src/domains/member/machines/mod.rs` - Triggers embedding after registration
- `src/domains/member/effects/mod.rs` - Handles GenerateEmbedding command
- `src/domains/member/models/member.rs` - Added update_embedding()
- `src/domains/organization/commands/mod.rs` - Added GenerateNeedEmbedding
- `src/domains/organization/effects/command_effects.rs` - Added EmbeddingService to ServerDeps

## Configuration Needed

```bash
# .env
DATABASE_URL=postgresql://...
OPENAI_API_KEY=sk-...
FIRECRAWL_API_KEY=...
EXPO_ACCESS_TOKEN=...  # TODO: Add when implementing Task 3
```

## Testing Plan

1. Unit tests for EmbeddingService (already in embeddings.rs)
2. Integration test: member registration → embedding generation
3. Integration test: need approval → embedding generation
4. Integration test: matching with embeddings
5. Manual test: seed script imports all organizations
6. Manual test: full flow with real Expo push token
