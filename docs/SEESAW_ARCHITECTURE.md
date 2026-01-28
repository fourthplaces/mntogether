# Seesaw-rs Architecture Refactoring

## Overview

The project has been restructured to follow the seesaw-rs event-driven architecture pattern, separating **facts** (Events) from **intent** (Commands).

## Core Principle

**One Command = One Transaction.** If multiple writes must be atomic, they belong in one command handled by one effect.

## Directory Structure

```
packages/
├── seesaw-rs/              # Event-driven framework (copied from Shay)
├── server/
│   └── src/
│       ├── kernel/
│       │   └── jobs/       # Postgres-based job queue (copied from Shay)
│       └── domains/
│           └── organization/
│               ├── models/      # SQL models with queries ONLY
│               │   ├── need.rs
│               │   └── source.rs
│               ├── data/        # GraphQL data types with resolvers
│               │   ├── need.rs
│               │   └── source.rs
│               ├── events/      # Event definitions (facts)
│               │   └── mod.rs
│               ├── commands/    # Command definitions (intent)
│               │   └── mod.rs
│               ├── machines/    # State machines (pure decisions)
│               │   └── mod.rs
│               ├── effects/     # IO handlers (execute commands)
│               │   ├── scraper_effects.rs
│               │   ├── ai_effects.rs
│               │   └── sync_effects.rs
│               └── edges/       # THIN wrappers (dispatch requests)
│                   ├── query.rs
│                   └── mutation.rs
└── app/                    # Renamed from expo-app
```

## Architecture Flow

```
Edge (dispatch_request) → Event → Machine.decide() → Command → Effect.execute() → Event → Runtime emits
                                                                                             ↓
                                                                                        EventTaps
```

## Models Pattern (Following Shay)

All SQL queries MUST be in `models/`. No queries outside this directory.

```rust
// packages/server/src/domains/organization/models/need.rs
impl OrganizationNeed {
    /// Find need by ID
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, OrganizationNeed>(
            "SELECT * FROM organization_needs WHERE id = $1"
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Find needs by status
    pub async fn find_by_status(status: &str, limit: i64, offset: i64, pool: &PgPool) -> Result<Vec<Self>> {
        // SQL query here
    }

    /// Insert new need
    pub async fn insert(&self, pool: &PgPool) -> Result<Self> {
        // SQL query here
    }

    // ... all other queries
}
```

## Events Pattern

Events are immutable facts. Auto-implemented for `Clone + Send + Sync + 'static`.

```rust
// packages/server/src/domains/organization/events/mod.rs
#[derive(Debug, Clone)]
pub enum OrganizationEvent {
    // Request events (from edges)
    ScrapeSourceRequested {
        source_id: Uuid,
    },
    SubmitNeedRequested {
        organization_name: String,
        title: String,
        description: String,
        contact_info: Option<JsonValue>,
        volunteer_id: Uuid,
        ip_address: Option<IpAddr>,
    },
    ApproveNeedRequested {
        need_id: Uuid,
    },

    // Fact events (from effects)
    SourceScraped {
        source_id: Uuid,
        content: String,
    },
    NeedsExtracted {
        source_id: Uuid,
        needs: Vec<ExtractedNeed>,
    },
    NeedCreated {
        need: OrganizationNeed,
    },
    NeedApproved {
        need_id: Uuid,
    },
    NeedRejected {
        need_id: Uuid,
    },
}
// Event trait is automatically implemented!
```

## Commands Pattern

Commands are requests for IO. Require explicit `impl Command`.

```rust
// packages/server/src/domains/organization/commands/mod.rs
use seesaw::Command;

#[derive(Debug, Clone)]
pub enum OrganizationCommand {
    ScrapeSource {
        source_id: Uuid,
    },
    ExtractNeeds {
        source_id: Uuid,
        content: String,
    },
    CreateNeed {
        organization_name: String,
        title: String,
        description: String,
        content_hash: String,
        // ... other fields
    },
    UpdateNeedStatus {
        need_id: Uuid,
        status: String,
    },
}

impl Command for OrganizationCommand {}
```

## Machines Pattern

Pure state machines that decide on commands. No IO.

```rust
// packages/server/src/domains/organization/machines/mod.rs
use seesaw::Machine;

pub struct OrganizationMachine {
    // Internal state tracking
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
            // ... other event -> command decisions
            _ => None,
        }
    }
}
```

## Effects Pattern

Stateless command handlers. Execute IO, emit events.

```rust
// packages/server/src/domains/organization/effects/scraper_effects.rs
use seesaw::{Effect, EffectContext};

pub struct ScraperEffect;

#[async_trait]
impl Effect<OrganizationCommand, ServerDeps> for ScraperEffect {
    type Event = OrganizationEvent;

    async fn execute(
        &self,
        cmd: OrganizationCommand,
        ctx: EffectContext<ServerDeps>
    ) -> Result<OrganizationEvent> {
        match cmd {
            OrganizationCommand::ScrapeSource { source_id } => {
                // Get source from DB
                let source = OrganizationSource::find_by_id(source_id, &ctx.deps().db).await?;

                // Scrape using Firecrawl
                let content = ctx.deps().firecrawl_client
                    .scrape(&source.source_url)
                    .await?;

                // Update last_scraped_at
                OrganizationSource::update_last_scraped(source_id, &ctx.deps().db).await?;

                // Emit fact event
                Ok(OrganizationEvent::SourceScraped {
                    source_id,
                    content: content.markdown,
                })
            }
            _ => Err(anyhow::anyhow!("Unexpected command")),
        }
    }
}
```

## Edges Pattern (THIN!)

Edges dispatch request events and await responses. NO business logic.

```rust
// packages/server/src/domains/organization/edges/mutation.rs
use seesaw::{dispatch_request, EnvelopeMatch};

pub async fn scrape_organization_source(
    source_id: Uuid,
    ctx: &GraphQLContext,
) -> FieldResult<bool> {
    // Dispatch request event and await response
    let _event = dispatch_request(
        OrganizationEvent::ScrapeSourceRequested { source_id },
        &ctx.bus,
        |m| m.try_match(|e: &OrganizationEvent| match e {
            OrganizationEvent::NeedsExtracted { source_id: sid, .. } if sid == &source_id => {
                Some(Ok(true))
            }
            _ => None,
        })
        .result()
    ).await?;

    Ok(true)
}

pub async fn approve_need(
    need_id: Uuid,
    ctx: &GraphQLContext,
) -> FieldResult<Need> {
    // Dispatch request and await fact event
    let need = dispatch_request(
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

    // Query result from DB (edges can do read queries)
    let need = OrganizationNeed::find_by_id(need_id, &ctx.db_pool).await?;
    Ok(Need::from(need))
}
```

## Engine Wiring

```rust
// packages/server/src/server/main.rs
use seesaw::EngineBuilder;

let engine = EngineBuilder::new(server_deps)
    .with_machine(OrganizationMachine::new())
    .with_effect::<OrganizationCommand, _>(ScraperEffect)
    .with_effect::<OrganizationCommand, _>(AIEffect)
    .with_effect::<OrganizationCommand, _>(SyncEffect)
    .with_job_queue(job_queue) // Postgres job queue
    .build();

let bus = engine.start();

// Store bus in GraphQLContext for edges to use
let context = GraphQLContext {
    db_pool,
    bus,
    // ... other deps
};
```

## Read Queries (No Event Flow)

Simple read queries can bypass the event flow:

```rust
// packages/server/src/domains/organization/edges/query.rs
pub async fn get_needs(
    status: Option<String>,
    limit: i64,
    offset: i64,
    ctx: &GraphQLContext,
) -> FieldResult<Vec<Need>> {
    // Direct query - no event flow needed for reads
    let status_str = status.as_deref().unwrap_or("active");
    let needs = OrganizationNeed::find_by_status(status_str, limit, offset, &ctx.db_pool).await?;
    Ok(needs.into_iter().map(Need::from).collect())
}
```

## Migration Status

### ✅ Completed
- Renamed expo-app to app
- Copied seesaw-rs framework
- Copied postgres job queue code
- Updated Cargo workspace
- Refactored models with SQL queries only

### 🚧 In Progress
- Create event definitions
- Create command definitions
- Create state machines
- Refactor effects to use seesaw pattern
- Make edges thin (dispatch_request pattern)
- Wire up seesaw engine

### 📋 Next Steps
1. Define all events in `events/mod.rs`
2. Define all commands in `commands/mod.rs`
3. Create state machines in `machines/mod.rs`
4. Refactor effects to implement `Effect` trait
5. Refactor edges to use `dispatch_request`
6. Update `server/main.rs` to wire up engine
7. Update tests to work with event-driven architecture
8. Remove old GraphQL mutation code that's now in effects

## Benefits

1. **Clear separation of concerns**: Facts vs Intent
2. **Testable business logic**: Machines are pure functions
3. **Transaction safety**: One command = one transaction
4. **Observability**: Event taps for metrics, audit, webhooks
5. **Scalability**: Background job execution via postgres queue
6. **Type safety**: Rust's type system prevents common errors
7. **Maintainability**: Changes localized to specific layers

## Data Pattern (GraphQL Layer)

The `data/` directory contains GraphQL-friendly data structures with resolvers. These are separate from SQL models to provide a clean API layer.

### Key Characteristics

1. **Serializable**: Implement `Serialize + Deserialize`
2. **GraphQL Types**: Use `#[juniper::graphql_object]` for resolvers
3. **From Models**: Convert from SQL models via `From<Model>` trait
4. **Nested Resolvers**: Fetch related data lazily via edge functions
5. **Access Control**: Handle authorization in resolvers

### Example

```rust
// packages/server/src/domains/organization/data/need.rs
use crate::domains::organization::models::need::OrganizationNeed;
use crate::server::graphql::context::GraphQLContext;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeedData {
    pub id: String,
    pub organization_name: String,
    pub title: String,
    pub description: String,
    pub tldr: Option<String>,
    pub contact_info: Option<ContactInfoData>,
    pub status: String,
    pub source_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// Convert from SQL model
impl From<OrganizationNeed> for NeedData {
    fn from(need: OrganizationNeed) -> Self {
        Self {
            id: need.id.to_string(),
            organization_name: need.organization_name,
            title: need.title,
            description: need.description,
            tldr: need.tldr,
            contact_info: need.contact_info.and_then(|json| {
                serde_json::from_value(json).ok()
            }),
            status: need.status,
            source_id: need.source_id.map(|id| id.to_string()),
            created_at: need.created_at.to_rfc3339(),
            updated_at: need.updated_at.to_rfc3339(),
        }
    }
}

// GraphQL resolvers
#[juniper::graphql_object(Context = GraphQLContext)]
impl NeedData {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn organization_name(&self) -> String {
        self.organization_name.clone()
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    // ... other scalar fields

    /// Lazy-load related source (nested resolver)
    async fn source(&self, context: &GraphQLContext) -> juniper::FieldResult<Option<SourceData>> {
        let Some(source_id_str) = &self.source_id else {
            return Ok(None);
        };

        let source_id = Uuid::parse_str(source_id_str)?;
        let source = OrganizationSource::find_by_id(source_id, &context.db_pool).await?;
        Ok(Some(source.into()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfoData {
    pub email: Option<String>,
    pub phone: Option<String>,
    pub website: Option<String>,
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl ContactInfoData {
    fn email(&self) -> Option<String> {
        self.email.clone()
    }

    fn phone(&self) -> Option<String> {
        self.phone.clone()
    }

    fn website(&self) -> Option<String> {
        self.website.clone()
    }
}
```

### GraphQL Query Example

```graphql
query GetNeed {
  need(id: "123") {
    id
    organizationName
    title
    description
    tldr
    contactInfo {
      email
      phone
      website
    }
    status
    source {
      id
      organizationName
      sourceUrl
      lastScrapedAt
    }
    createdAt
  }
}
```

### Benefits

1. **Separation of Concerns**: SQL models focus on persistence, data types focus on API
2. **Type Safety**: Juniper validates GraphQL schema at compile time
3. **Performance**: Nested resolvers enable lazy loading (N+1 query optimization via DataLoader pattern)
4. **Evolution**: Can change GraphQL API without changing database schema
5. **Documentation**: GraphQL schema is self-documenting

### Layer Responsibilities

| Layer    | Responsibility                          | Example                           |
| -------- | --------------------------------------- | --------------------------------- |
| `models` | SQL queries, database persistence       | `find_by_id`, `insert`, `update`  |
| `data`   | GraphQL types, nested resolvers         | Field getters, lazy-load related  |
| `edges`  | Business logic, request dispatching     | `dispatch_request`, orchestration |
| `events` | Facts and requests                      | `NeedCreated`, `ApproveRequested` |
| `commands`| Intent for IO                          | `CreateNeed`, `UpdateStatus`      |
| `effects`| IO execution, emit facts               | Database writes, API calls        |
| `machines`| State transitions, pure decisions      | Event → Command logic             |

