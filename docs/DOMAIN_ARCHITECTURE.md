# Domain Architecture - Layered Separation

## Overview

Each domain follows a strict layered architecture with clear separation of concerns. This document explains the purpose and responsibility of each layer.

## Directory Structure

```
packages/server/src/domains/{domain}/
├── models/      # SQL models with database queries
├── data/        # GraphQL data types with resolvers
├── events/      # Event definitions (facts and requests)
├── commands/    # Command definitions (intent for IO)
├── machines/    # State machines (pure decision logic)
├── effects/     # IO handlers (execute commands, emit events)
└── edges/       # Business logic (thin request dispatchers)
```

## Layer Responsibilities

### 1. Models (`models/`)

**Purpose**: SQL persistence layer - database queries ONLY

**Rules**:
- ALL SQL queries must be in this directory
- NO queries outside models/
- NO business logic
- NO GraphQL types
- Use `sqlx::FromRow` for SQL mapping
- Methods are CRUD operations only

**Example**:
```rust
// models/need.rs
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct OrganizationNeed {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

impl OrganizationNeed {
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>("SELECT * FROM organization_needs WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_status(status: &str, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM organization_needs WHERE status = $1")
            .bind(status)
            .fetch_all(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn insert(&self, pool: &PgPool) -> Result<Self> {
        // INSERT query here
    }

    pub async fn update_status(id: Uuid, status: &str, pool: &PgPool) -> Result<Self> {
        // UPDATE query here
    }
}
```

### 2. Data (`data/`)

**Purpose**: GraphQL API layer - public data types with resolvers

**Rules**:
- Implement `Serialize + Deserialize`
- Use `#[juniper::graphql_object]` for resolvers
- Convert from models via `From<Model>` trait
- Nested resolvers for related data (lazy-loading)
- Handle field-level access control
- String IDs (not Uuid) for GraphQL compatibility

**Example**:
```rust
// data/need.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeedData {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub source_id: Option<String>,
}

impl From<OrganizationNeed> for NeedData {
    fn from(need: OrganizationNeed) -> Self {
        Self {
            id: need.id.to_string(),
            title: need.title,
            description: need.description,
            status: need.status,
            source_id: need.source_id.map(|id| id.to_string()),
        }
    }
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl NeedData {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    // Nested resolver - lazy-load related data
    async fn source(&self, ctx: &GraphQLContext) -> juniper::FieldResult<Option<SourceData>> {
        let Some(source_id_str) = &self.source_id else {
            return Ok(None);
        };
        let source_id = Uuid::parse_str(source_id_str)?;
        let source = OrganizationSource::find_by_id(source_id, &ctx.db_pool).await?;
        Ok(Some(source.into()))
    }
}
```

### 3. Events (`events/`)

**Purpose**: Immutable facts and request definitions

**Rules**:
- `#[derive(Debug, Clone)]` - auto-implements Event trait
- Two types: Request events (from edges) and Fact events (from effects)
- Request events end with "Requested"
- Fact events describe what happened (past tense)
- NO business logic
- Just data structures

**Example**:
```rust
// events/mod.rs
#[derive(Debug, Clone)]
pub enum OrganizationEvent {
    // Request events (from edges)
    ScrapeSourceRequested {
        source_id: Uuid,
    },
    ApproveNeedRequested {
        need_id: Uuid,
    },

    // Fact events (from effects)
    SourceScraped {
        source_id: Uuid,
        content: String,
    },
    NeedCreated {
        need_id: Uuid,
    },
    NeedApproved {
        need_id: Uuid,
    },
}
```

### 4. Commands (`commands/`)

**Purpose**: Intent for IO operations

**Rules**:
- `impl Command` trait explicitly
- One command = one transaction
- Specify execution mode (Inline, Background, Scheduled)
- For Background/Scheduled: implement `job_spec()` and `serialize_to_json()`
- NO business logic
- Just data structures

**Example**:
```rust
// commands/mod.rs
use seesaw::Command;

#[derive(Debug, Clone)]
pub enum OrganizationCommand {
    ScrapeSource {
        source_id: Uuid,
    },
    CreateNeed {
        organization_name: String,
        title: String,
        description: String,
        content_hash: String,
    },
    UpdateNeedStatus {
        need_id: Uuid,
        status: String,
    },
}

impl Command for OrganizationCommand {
    fn execution_mode(&self) -> ExecutionMode {
        match self {
            Self::ScrapeSource { .. } => ExecutionMode::Background,
            Self::CreateNeed { .. } => ExecutionMode::Inline,
            Self::UpdateNeedStatus { .. } => ExecutionMode::Inline,
        }
    }

    fn job_spec(&self) -> Option<JobSpec> {
        match self {
            Self::ScrapeSource { source_id } => Some(JobSpec {
                job_type: "scrape_source".to_string(),
                unique_key: Some(source_id.to_string()),
                max_retries: 3,
                timeout_seconds: 300,
            }),
            _ => None,
        }
    }
}
```

### 5. Machines (`machines/`)

**Purpose**: Pure state machines - make decisions based on events

**Rules**:
- `impl Machine` trait
- Pure functions - NO IO
- Update internal state
- Return `Option<Command>` based on event
- Deterministic logic

**Example**:
```rust
// machines/mod.rs
use seesaw::Machine;
use std::collections::HashMap;

pub struct OrganizationMachine {
    pending_scrapes: HashMap<Uuid, ()>,
}

impl Machine for OrganizationMachine {
    type Event = OrganizationEvent;
    type Command = OrganizationCommand;

    fn decide(&mut self, event: &OrganizationEvent) -> Option<OrganizationCommand> {
        match event {
            OrganizationEvent::ScrapeSourceRequested { source_id } => {
                self.pending_scrapes.insert(*source_id, ());
                Some(OrganizationCommand::ScrapeSource {
                    source_id: *source_id,
                })
            }
            OrganizationEvent::SourceScraped { source_id, content } => {
                self.pending_scrapes.remove(source_id);
                Some(OrganizationCommand::ExtractNeeds {
                    source_id: *source_id,
                    content: content.clone(),
                })
            }
            _ => None,
        }
    }
}
```

### 6. Effects (`effects/`)

**Purpose**: Execute IO operations - the ONLY layer that does IO

**Rules**:
- `impl Effect<Command, Deps>` trait
- Stateless - no internal state
- Access dependencies via `ctx.deps()`
- Execute IO (database, API calls, etc.)
- Return ONE event (fact of what happened)
- Use `EffectContext` for deps and correlation
- Authorization happens HERE

**Example**:
```rust
// effects/scraper_effects.rs
use seesaw::{Effect, EffectContext};

pub struct ScraperEffect;

#[async_trait]
impl Effect<OrganizationCommand, ServerDeps> for ScraperEffect {
    type Event = OrganizationEvent;

    async fn execute(
        &self,
        cmd: OrganizationCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<OrganizationEvent> {
        match cmd {
            OrganizationCommand::ScrapeSource { source_id } => {
                // Get source from DB
                let source = OrganizationSource::find_by_id(
                    source_id,
                    &ctx.deps().db_pool
                ).await?;

                // Scrape using external API
                let result = ctx.deps()
                    .firecrawl_client
                    .scrape(&source.source_url)
                    .await?;

                // Update timestamp
                OrganizationSource::update_last_scraped(
                    source_id,
                    &ctx.deps().db_pool
                ).await?;

                // Return fact event
                Ok(OrganizationEvent::SourceScraped {
                    source_id,
                    content: result.markdown,
                })
            }
            _ => Err(anyhow::anyhow!("Unexpected command")),
        }
    }
}
```

### 7. Edges (`edges/`)

**Purpose**: Business logic entry points - THIN request dispatchers

**Rules**:
- Use `dispatch_request` to emit events and await responses
- NO direct database writes
- Can do read queries for return values
- Authorization checks (before dispatching)
- Validation
- Convert between GraphQL types and events
- THIN - most logic in machines/effects

**Example**:
```rust
// edges/mutation.rs
use seesaw::{dispatch_request, EnvelopeMatch};

pub async fn approve_need(
    need_id: Uuid,
    ctx: &GraphQLContext,
) -> FieldResult<NeedData> {
    // Dispatch request event and await fact event
    dispatch_request(
        OrganizationEvent::ApproveNeedRequested { need_id },
        &ctx.bus,
        |m| m.try_match(|e: &OrganizationEvent| match e {
            OrganizationEvent::NeedApproved { need_id: nid } if nid == &need_id => {
                Some(Ok(()))
            }
            _ => None,
        })
        .result()
    ).await?;

    // Query result from DB (read queries OK in edges)
    let need = OrganizationNeed::find_by_id(need_id, &ctx.db_pool).await?;
    Ok(NeedData::from(need))
}
```

## Data Flow

### Write Operations (with events)

```
1. GraphQL Request
   ↓
2. Edge (dispatch_request)
   ↓
3. Event (request)
   ↓
4. Machine.decide()
   ↓
5. Command
   ↓
6. Effect.execute() [IO happens here]
   ↓
7. Event (fact)
   ↓
8. Runtime emits to EventBus
   ↓
9. Edge awaits and returns
```

### Read Operations (no events)

```
1. GraphQL Request
   ↓
2. Data resolver (or Edge function)
   ↓
3. Model.find_by_*()
   ↓
4. Return Data type
```

## Benefits of Layered Architecture

1. **Testability**: Each layer can be tested independently
2. **Maintainability**: Changes localized to specific layers
3. **Type Safety**: Rust's type system prevents layer violations
4. **Performance**: Lazy-loading via nested resolvers
5. **Observability**: Event taps for metrics, audit, webhooks
6. **Scalability**: Background jobs via postgres queue
7. **Transaction Safety**: One command = one transaction
8. **Separation**: SQL persistence vs API vs business logic

## Anti-Patterns to Avoid

❌ **SQL queries outside models/**
❌ **Business logic in data resolvers**
❌ **IO in machines (must be pure)**
❌ **Multiple events from one effect (emit ONE fact)**
❌ **Fat edges (should dispatch requests, not do work)**
❌ **Direct database writes in edges (use events/commands)**
❌ **GraphQL types in models (use data/ layer)**

## Examples by Use Case

### Creating a Need (with approval workflow)

```rust
// 1. Edge dispatches request
pub async fn submit_need(input: SubmitNeedInput, ctx: &GraphQLContext) -> FieldResult<NeedData> {
    dispatch_request(
        OrganizationEvent::SubmitNeedRequested {
            organization_name: input.organization_name,
            title: input.title,
            description: input.description,
            // ... other fields
        },
        &ctx.bus,
        |m| m.try_match(|e: &OrganizationEvent| match e {
            OrganizationEvent::NeedCreated { need_id } => {
                Some(Ok(*need_id))
            }
            _ => None,
        }).result()
    ).await?;

    // Query and return
}

// 2. Machine decides on command
impl Machine for OrganizationMachine {
    fn decide(&mut self, event: &OrganizationEvent) -> Option<OrganizationCommand> {
        match event {
            OrganizationEvent::SubmitNeedRequested { .. } => {
                Some(OrganizationCommand::CreateNeed { /* fields */ })
            }
            _ => None,
        }
    }
}

// 3. Effect executes IO
impl Effect for CreateNeedEffect {
    async fn execute(&self, cmd: OrganizationCommand, ctx: EffectContext<Deps>) -> Result<Event> {
        match cmd {
            OrganizationCommand::CreateNeed { .. } => {
                // Create need model
                let need = OrganizationNeed { /* fields */ };

                // Insert into DB
                let created = need.insert(&ctx.deps().db_pool).await?;

                // Return fact event
                Ok(OrganizationEvent::NeedCreated {
                    need_id: created.id,
                })
            }
        }
    }
}
```

### Fetching a Need with Source (GraphQL)

```graphql
query GetNeed {
  need(id: "123") {
    id
    title
    description
    source {
      organizationName
      sourceUrl
    }
  }
}
```

```rust
// 1. Query resolver (data layer)
#[juniper::graphql_object(Context = GraphQLContext)]
impl Query {
    async fn need(id: String, ctx: &GraphQLContext) -> FieldResult<Option<NeedData>> {
        let need_id = Uuid::parse_str(&id)?;
        let need = OrganizationNeed::find_by_id(need_id, &ctx.db_pool).await?;
        Ok(Some(NeedData::from(need)))
    }
}

// 2. Nested resolver (lazy-loads source only if requested)
#[juniper::graphql_object(Context = GraphQLContext)]
impl NeedData {
    async fn source(&self, ctx: &GraphQLContext) -> FieldResult<Option<SourceData>> {
        let Some(source_id_str) = &self.source_id else {
            return Ok(None);
        };
        let source_id = Uuid::parse_str(source_id_str)?;
        let source = OrganizationSource::find_by_id(source_id, &ctx.db_pool).await?;
        Ok(Some(SourceData::from(source)))
    }
}
```

## Summary

Each layer has a single, clear responsibility:

| Layer      | Responsibility                   | Can Do                          | Cannot Do              |
| ---------- | -------------------------------- | ------------------------------- | ---------------------- |
| `models`   | Database persistence             | SQL queries                     | Business logic, IO     |
| `data`     | GraphQL API                      | Resolvers, lazy-loading         | SQL queries, IO        |
| `events`   | Facts and requests               | Data structures                 | Logic, IO              |
| `commands` | Intent for IO                    | Data structures, execution mode | Logic, IO              |
| `machines` | State transitions                | Pure decisions                  | IO, side effects       |
| `effects`  | IO execution                     | Database, API calls, emit facts | Multiple events, state |
| `edges`    | Business logic entry points      | Dispatch requests, read queries | Direct writes, fat logic |

This architecture enables:
- **Clear boundaries** between layers
- **Easy testing** of business logic
- **Transaction safety** via commands
- **Type safety** via Rust + Juniper
- **Performance** via lazy-loading
- **Observability** via event taps
- **Scalability** via background jobs
