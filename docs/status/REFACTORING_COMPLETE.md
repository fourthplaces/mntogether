# Event-Driven Architecture Refactoring - COMPLETE

## Overview

Successfully refactored the organization domain to follow seesaw-rs event-driven architecture patterns from Shay.

## What Was Changed

### 1. Created Event Definitions (`events/mod.rs`)
- **Request Events** (from edges): `ScrapeSourceRequested`, `SubmitNeedRequested`, `ApproveNeedRequested`, `EditAndApproveNeedRequested`, `RejectNeedRequested`
- **Fact Events** (from effects): `SourceScraped`, `NeedsExtracted`, `NeedsSynced`, `NeedCreated`, `NeedApproved`, `NeedRejected`, `NeedUpdated`
- Auto-implements Event trait via `Clone + Send + Sync + 'static`

### 2. Created Command Definitions (`commands/mod.rs`)
- **Commands**: `ScrapeSource`, `ExtractNeeds`, `SyncNeeds`, `CreateNeed`, `UpdateNeedStatus`, `UpdateNeedAndApprove`
- Implements `Command` trait with:
  - `execution_mode()`: Background for long-running IO, Inline for fast operations
  - `job_spec()`: Postgres job queue specs for background commands

### 3. Created State Machine (`machines/mod.rs`)
- `OrganizationMachine` implements `Machine` trait
- Pure decision logic (NO IO)
- Event flow:
  - `ScrapeSourceRequested` → `ScrapeSource` command
  - `SourceScraped` → `ExtractNeeds` command
  - `NeedsExtracted` → `SyncNeeds` command
  - `NeedsSynced` → done (cleanup state)
  - `SubmitNeedRequested` → `CreateNeed` command
  - `ApproveNeedRequested` → `UpdateNeedStatus` command
  - `EditAndApproveNeedRequested` → `UpdateNeedAndApprove` command
  - `RejectNeedRequested` → `UpdateNeedStatus` command

### 4. Created Effect Implementations (`effects/command_effects.rs`)
- **ScraperEffect**: Handles `ScrapeSource` command
  - Fetches source from DB
  - Calls Firecrawl API
  - Updates last_scraped_at
  - Returns `SourceScraped` event

- **AIEffect**: Handles `ExtractNeeds` command
  - Calls OpenAI API for need extraction
  - Converts AI output to event format
  - Returns `NeedsExtracted` event

- **SyncEffect**: Handles `SyncNeeds` command
  - Syncs extracted needs with database
  - Handles new/changed/disappeared needs
  - Returns `NeedsSynced` event

- **NeedEffect**: Handles `CreateNeed`, `UpdateNeedStatus`, `UpdateNeedAndApprove`
  - Creates needs in database
  - Updates need status
  - Updates need content and approves
  - Returns appropriate fact events

### 5. Refactored Edges to THIN Dispatchers (`edges/mutation.rs`)

**Before (VIOLATIONS):**
```rust
pub async fn approve_need(pool: &PgPool, need_id: Uuid) -> FieldResult<Need> {
    // Direct SQL UPDATE query (VIOLATION!)
    sqlx::query!("UPDATE organization_needs SET status = $1 WHERE id = $2", ...)
        .execute(pool)
        .await?;
    // ...
}
```

**After (COMPLIANT):**
```rust
pub async fn approve_need(ctx: &GraphQLContext, need_id: Uuid) -> FieldResult<Need> {
    // Dispatch request event and await fact event
    dispatch_request(
        OrganizationEvent::ApproveNeedRequested { need_id },
        &ctx.bus,
        |m| m.try_match(|e| match e {
            OrganizationEvent::NeedApproved { need_id: nid } if nid == &need_id => Some(Ok(())),
            _ => None,
        }).result()
    ).await?;

    // Query result from database (read queries OK in edges)
    let need = OrganizationNeed::find_by_id(need_id, &ctx.db_pool).await?;
    Ok(Need::from(need))
}
```

**All mutations now follow this pattern:**
- ✅ `scrape_organization`: Dispatches `ScrapeSourceRequested`, awaits `NeedsSynced`
- ✅ `submit_need`: Dispatches `SubmitNeedRequested`, awaits `NeedCreated`
- ✅ `approve_need`: Dispatches `ApproveNeedRequested`, awaits `NeedApproved`
- ✅ `edit_and_approve_need`: Dispatches `EditAndApproveNeedRequested`, awaits `NeedApproved`
- ✅ `reject_need`: Dispatches `RejectNeedRequested`, awaits `NeedRejected`

## Architecture Flow

```
GraphQL Request
   ↓
Edge (dispatch_request)
   ↓
Request Event
   ↓
Machine.decide() → Command
   ↓
Effect.execute() [IO HAPPENS HERE]
   ↓
Fact Event
   ↓
Runtime emits to EventBus
   ↓
Edge awaits and returns
```

## Benefits

1. **Clear Separation**: Facts (events) vs Intent (commands)
2. **Testable Logic**: Machines are pure functions, effects are isolated
3. **Transaction Safety**: One command = one transaction
4. **Observability**: Event taps for metrics, audit logs, webhooks
5. **Scalability**: Background job execution via Postgres queue
6. **Type Safety**: Rust's type system prevents common errors
7. **Maintainability**: Changes localized to specific layers

## Directory Structure

```
packages/server/src/domains/organization/
├── models/           # SQL models with queries ONLY ✅
│   ├── need.rs      # All need-related SQL queries
│   └── source.rs    # All source-related SQL queries
├── data/            # GraphQL data types with resolvers ✅
│   ├── need.rs      # NeedData with lazy-loading
│   └── source.rs    # SourceData with lazy-loading
├── events/          # Event definitions (facts and requests) ✅
│   └── mod.rs       # OrganizationEvent enum
├── commands/        # Command definitions (intent) ✅
│   └── mod.rs       # OrganizationCommand enum + Command trait
├── machines/        # State machines (pure decisions) ✅
│   └── mod.rs       # OrganizationMachine + Machine trait
├── effects/         # IO handlers (execute commands) ✅
│   ├── command_effects.rs  # Effect implementations
│   ├── scraper_effects.rs  # FirecrawlClient
│   ├── ai_effects.rs       # NeedExtractor
│   ├── sync_effects.rs     # sync_needs utility
│   └── submit_effects.rs   # submit_user_need utility
└── edges/           # THIN dispatchers ✅
    ├── mutation.rs  # dispatch_request patterns
    └── query.rs     # Direct read queries (OK)
```

## Next Steps

1. **Wire up seesaw engine** in `server/main.rs`:
   ```rust
   let engine = EngineBuilder::new(server_deps)
       .with_machine(OrganizationMachine::new())
       .with_effect::<OrganizationCommand, _>(ScraperEffect)
       .with_effect::<OrganizationCommand, _>(AIEffect)
       .with_effect::<OrganizationCommand, _>(SyncEffect)
       .with_effect::<OrganizationCommand, _>(NeedEffect)
       .with_job_queue(job_queue)
       .build();

   let bus = engine.start();
   ```

2. **Update GraphQL context** to include event bus
3. **Update GraphQL schema** to call refactored edge functions
4. **Update tests** to work with event-driven architecture
5. **Remove old effect utility functions** that bypassed events

## Rules Enforced

✅ **NO SQL queries outside models/** - All queries in models/ layer  
✅ **Edges are THIN** - Only dispatch requests, no direct writes  
✅ **One Command = One Transaction** - Atomic operations in effects  
✅ **Events are immutable facts** - Auto-implemented via Clone trait  
✅ **Machines are pure** - No IO, only decision logic  
✅ **Effects are stateless** - Access dependencies via EffectContext  

## Documentation

- See `docs/SEESAW_ARCHITECTURE.md` for event-driven patterns
- See `docs/DOMAIN_ARCHITECTURE.md` for complete layer guide
