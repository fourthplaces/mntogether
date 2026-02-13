# Domain Architecture - Restate Workflows & Activities

## Overview

Each domain follows a layered architecture built on **Restate** for durable workflow execution. The project migrated from seesaw-rs (event-driven state machines) to Restate SDK 0.4.0 in early 2026.

## Directory Structure

```
packages/server/src/domains/{domain}/
├── models/       # SQL models with database queries (sqlx)
├── data/         # GraphQL-style data types (Serialize/Deserialize)
├── activities/   # Pure async functions (business logic + IO)
├── restate/      # Restate service/workflow/virtual object definitions
│   ├── services/         # Stateless request handlers
│   ├── workflows/        # Durable multi-step orchestrations
│   └── virtual_objects/  # Keyed stateful objects
└── mod.rs        # Domain module exports
```

Not every domain has all layers. Simpler domains may only have `models/` and `restate/services/`.

## Layer Responsibilities

### 1. Models (`models/`)

**Purpose**: SQL persistence layer - database queries ONLY

**Rules**:
- ALL SQL queries must be in this directory
- NO queries outside models/
- NO business logic
- Use `sqlx::FromRow` for SQL mapping
- Always use `sqlx::query_as::<_, Self>()` (never the `query_as!` macro)

**Example** (from `organization/models/organization.rs`):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub submitted_by: Option<MemberId>,
    pub last_extracted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Organization {
    pub async fn find_by_id(id: OrganizationId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM organizations WHERE id = $1"
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn create(name: &str, description: Option<&str>, submitter_type: &str, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO organizations (name, description, submitter_type, status) VALUES ($1, $2, $3, 'pending_review') RETURNING *",
        )
        .bind(name)
        .bind(description)
        .bind(submitter_type)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }
}
```

### 2. Data (`data/`)

**Purpose**: Serializable data types for API responses

**Rules**:
- Implement `Serialize + Deserialize`
- Convert from models via `From<Model>` trait
- String IDs (not Uuid) for API compatibility
- Used by Restate services to return structured data

**Example**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostData {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub source_url: Option<String>,
}

impl From<Post> for PostData {
    fn from(post: Post) -> Self {
        Self {
            id: post.id.to_string(),
            title: post.title,
            description: post.description,
            status: post.status,
            source_url: post.source_url,
        }
    }
}
```

### 3. Activities (`activities/`)

**Purpose**: Pure async functions that contain business logic and IO

**Rules**:
- Take `&ServerDeps` explicitly as a parameter
- Return simple data types (not domain events)
- Can call models for database access
- Can call external APIs (LLM, scraping, etc.)
- Use `deps.memo()` for caching expensive operations
- Stateless - no internal state

**Example** (from `curator/activities/brief_extraction.rs`):
```rust
pub async fn extract_briefs_for_org(
    org_name: &str,
    pages: &[CachedPage],
    deps: &ServerDeps,
) -> Result<Vec<PageBriefExtraction>> {
    let mut briefs = Vec::new();
    for page in pages {
        let brief = deps.ai.complete(GPT_5_MINI, &format_prompt(org_name, page)).await?;
        briefs.push(brief);
    }
    Ok(briefs)
}
```

### 4. Restate (`restate/`)

**Purpose**: Durable execution layer - service definitions, workflows, and virtual objects

Restate provides three handler types:

#### Services (`restate/services/`)
Stateless request handlers. Most domains have these.

```rust
#[restate_sdk::service]
pub trait PostsService {
    async fn list_posts(req: ListPostsRequest) -> Result<PostsResponse, HandlerError>;
    async fn approve_post(req: ApprovePostRequest) -> Result<PostResponse, HandlerError>;
}
```

#### Workflows (`restate/workflows/`)
Durable multi-step orchestrations that survive process restarts.

```rust
#[restate_sdk::workflow]
#[name = "CurateOrgWorkflow"]
pub trait CurateOrgWorkflow {
    async fn run(req: CurateOrgRequest) -> Result<CurateOrgResult, HandlerError>;
    #[shared]
    async fn get_status(req: EmptyRequest) -> Result<String, HandlerError>;
}
```

#### Virtual Objects (`restate/virtual_objects/`)
Keyed stateful objects with concurrency guarantees per key.

```rust
#[restate_sdk::object]
pub trait PostObject {
    async fn get(req: EmptyRequest) -> Result<PostResponse, HandlerError>;
    async fn update(req: UpdatePostRequest) -> Result<PostResponse, HandlerError>;
}
```

## Workflow Implementation Pattern

All workflows follow the same structure:

```rust
// 1. Request/Response types with Restate serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurateOrgRequest {
    pub organization_id: Uuid,
}
impl_restate_serde!(CurateOrgRequest);

// 2. Trait definition (no &self or ctx in signature)
#[restate_sdk::workflow]
pub trait CurateOrgWorkflow {
    async fn run(req: CurateOrgRequest) -> Result<CurateOrgResult, HandlerError>;
}

// 3. Implementation struct with Arc<ServerDeps>
pub struct CurateOrgWorkflowImpl {
    deps: Arc<ServerDeps>,
}

impl CurateOrgWorkflowImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

// 4. Implementation adds &self and ctx parameters
impl CurateOrgWorkflow for CurateOrgWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        req: CurateOrgRequest,
    ) -> Result<CurateOrgResult, HandlerError> {
        // Use ctx.set() for status tracking
        ctx.set("status", "Loading organization...".to_string());

        // Call activities with &self.deps
        let briefs = extract_briefs_for_org(&org.name, &pages, &self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        // Return result directly (no events)
        Ok(CurateOrgResult { ... })
    }
}
```

## Data Flow

### Write Operations (via Restate)

```
1. Next.js API Route / Server Action
   ↓ HTTP
2. WorkflowClient (HTTP to Restate runtime on port 9070)
   ↓
3. Restate Runtime (durable execution proxy)
   ↓ HTTP (port 9080)
4. Service/Workflow handler
   ↓
5. Activities (business logic + IO)
   ↓
6. Models (database queries)
   ↓
7. Return result to caller
```

### Read Operations

```
1. Next.js API Route / Server Action
   ↓ HTTP
2. Restate Service handler
   ↓
3. Model.find_by_*()
   ↓
4. Return data type
```

## Registration

All Restate handlers are registered in `src/bin/server.rs`:

```rust
let endpoint = Endpoint::builder()
    // Auth domain
    .bind(AuthServiceImpl::with_deps(server_deps.clone()).serve())
    // Curator domain
    .bind(CurateOrgWorkflowImpl::with_deps(server_deps.clone()).serve())
    .bind(RefineProposalWorkflowImpl::with_deps(server_deps.clone()).serve())
    // Posts domain
    .bind(PostObjectImpl::with_deps(server_deps.clone()).serve())
    .bind(PostsServiceImpl::with_deps(server_deps.clone()).serve())
    // ... all other domains
    .build();
```

## Summary

| Layer        | Responsibility              | Can Do                            | Cannot Do                |
|--------------|-----------------------------|-----------------------------------|--------------------------|
| `models`     | Database persistence        | SQL queries, FromRow              | Business logic, IO       |
| `data`       | API data types              | Serialization, From<Model>        | SQL queries, IO          |
| `activities` | Business logic              | IO, LLM calls, model queries      | Hold state               |
| `restate`    | Durable execution           | Orchestrate activities, durability | Direct SQL, business logic |

## Anti-Patterns to Avoid

- SQL queries outside `models/`
- Business logic in Restate handlers (keep them thin, delegate to activities)
- Using `sqlx::query_as!` macro (always use the function version)
- Fat service handlers (should orchestrate activities, not do work directly)
- Holding mutable state in service/workflow impls (use `Arc<ServerDeps>` for shared deps)
