# Claude Code Development Rules

## Rule Zero: database modifications require explicit permission

Database-modifying commands are blocked by a PreToolUse hook
(`.claude/hooks/guard-db-commands.sh`). The hook will deny any Bash command
that runs migrations, executes write SQL (INSERT, UPDATE, DELETE, ALTER, DROP,
TRUNCATE, CREATE, GRANT, REVOKE) via psql or docker exec, or pipes a SQL file
into the database.

**What to do:**
1. Write the migration file (this is allowed).
2. Tell the user: "Migration file created at `path`. Run it when ready."
3. Wait for explicit approval ("go", "do it", "yes", "proceed").
4. Only then execute.

Read-only commands (SELECT, `\dt`, `pg_dump`) pass through without restriction.

---

## Migration files are immutable

Edits to existing migration files are blocked by a PreToolUse hook
(`.claude/hooks/guard-migrations.sh`). The hook will deny any Edit to a file
in `packages/server/migrations/`, and any Write that would overwrite an
existing file in that directory.

SQLx checksums every migration file. Modifying one that has been applied breaks
production deployments with no recovery short of manual database surgery.

If you need to fix a migration, create a new migration file with the next
sequential number.

---

## SQLx Query Rules

### HARD RULE: Never use `sqlx::query_as!` macro

**Always use the function version `sqlx::query_as::<_, Type>` instead.**

The macro version (`query_as!`) requires compile-time type checking against the database schema and can cause issues with nullable fields and type inference. The function version is more flexible and handles JSON/JSONB columns and nullable fields correctly.

#### ✅ Correct Pattern:

```rust
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct MyModel {
    pub id: Uuid,
    pub name: String,
    pub metadata: Option<serde_json::Value>,
}

impl MyModel {
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM my_table WHERE id = $1"
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_all(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM my_table ORDER BY created_at DESC"
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}
```

#### ❌ Incorrect Pattern (DO NOT USE):

```rust
// NEVER USE THIS!
sqlx::query_as!(
    MyModel,
    r#"SELECT * FROM my_table WHERE id = $1"#,
    id
)
```

### Key Points:

1. **Always use turbofish syntax**: `sqlx::query_as::<_, Type>`
2. **Derive `FromRow` on your structs**: `#[derive(FromRow)]`
3. **Use `.bind()` for parameters**: Chain `.bind(value)` for each `$1`, `$2`, etc.
4. **Handle errors properly**: Use `.map_err(Into::into)` or `.context("message")`
5. **JSON fields work automatically**: `serde_json::Value` fields deserialize correctly
6. **Nullable fields are `Option<T>`**: No special annotations needed

### Examples:

```rust
// Simple query
pub async fn find_by_email(email: &str, pool: &PgPool) -> Result<Option<User>> {
    sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE email = $1"
    )
    .bind(email)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

// Insert with RETURNING
pub async fn create(name: String, pool: &PgPool) -> Result<Self> {
    sqlx::query_as::<_, Self>(
        "INSERT INTO items (name) VALUES ($1) RETURNING *"
    )
    .bind(name)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

// Join query
pub async fn find_with_relations(id: Uuid, pool: &PgPool) -> Result<Vec<Self>> {
    sqlx::query_as::<_, Self>(
        "SELECT t.*
         FROM tags t
         INNER JOIN tag_relations tr ON t.id = tr.tag_id
         WHERE tr.entity_id = $1
         ORDER BY t.name ASC"
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}
```

## Why This Rule Exists

The `query_as!` macro:
- Requires compile-time database access
- Can fail with `Option<Option<T>>` type errors for nullable fields
- Needs explicit type annotations in SQL for custom types
- Makes builds slower and more fragile

The `query_as` function:
- Works without database at compile time
- Uses Rust's type system for inference
- Handles nullability naturally with `Option<T>`
- Faster compilation
- More flexible for complex queries

## Database Migration Rules

Migration immutability is enforced by the `guard-migrations.sh` hook (see above).
When you need to change the schema, always create a new migration file:

```sql
-- Need to add a column to an existing table?
-- Create 000058_add_user_email.sql (never edit the original migration)
ALTER TABLE users ADD COLUMN email TEXT;
```

---

## Database Schema Rules

### HARD RULE: Avoid JSONB Columns

**Always normalize data into proper relational tables instead of using JSONB columns.**

JSONB columns make data harder to query, analyze, and maintain. Use proper foreign keys and normalized tables instead.

#### ✅ Correct Pattern (Normalized):

```sql
-- Store search results in proper tables
CREATE TABLE search_queries (
    id UUID PRIMARY KEY,
    domain_id UUID REFERENCES domains(id),
    query TEXT NOT NULL,
    executed_at TIMESTAMP WITH TIME ZONE
);

CREATE TABLE search_results (
    id UUID PRIMARY KEY,
    query_id UUID REFERENCES search_queries(id),
    title TEXT NOT NULL,
    url TEXT NOT NULL,
    content TEXT,
    score DECIMAL(3,2),
    published_date TEXT
);
```

**Benefits:**
- Can query individual results directly: `SELECT * FROM search_results WHERE content LIKE '%fraud%'`
- Can join on results: `JOIN search_results ON ...`
- Can add indexes on specific columns: `CREATE INDEX ON search_results(score)`
- Type safety in Rust models
- No JSON parsing overhead

#### ❌ Incorrect Pattern (DO NOT USE):

```sql
-- AVOID THIS!
CREATE TABLE domains (
    id UUID PRIMARY KEY,
    search_results JSONB  -- Makes querying hard!
);
```

**Problems with JSONB:**
- Harder to query: `WHERE search_results->>'title' = ...` (awkward syntax)
- No foreign key constraints
- No type safety
- Requires JSON parsing in application code
- Can't add proper indexes on nested fields
- Makes migrations harder (changing JSON structure)

### When JSONB is Acceptable

Use JSONB **only** for:
1. **Truly unstructured data** with unknown schema (e.g., arbitrary user metadata)
2. **External API responses** that you don't control and only store for auditing
3. **Configuration objects** with frequent schema changes

If your data has a known structure, **normalize it into proper tables**.

### Example: Normalizing Nested Data

Instead of:
```sql
CREATE TABLE assessments (
    id UUID PRIMARY KEY,
    tavily_results JSONB  -- AVOID!
);
```

Use:
```sql
CREATE TABLE assessments (
    id UUID PRIMARY KEY
);

CREATE TABLE tavily_queries (
    id UUID PRIMARY KEY,
    assessment_id UUID REFERENCES assessments(id),
    query TEXT NOT NULL
);

CREATE TABLE tavily_results (
    id UUID PRIMARY KEY,
    query_id UUID REFERENCES tavily_queries(id),
    title TEXT,
    url TEXT,
    score DECIMAL(3,2)
);
```

## Domain Structure Rules

### HARD RULE: Database Queries Live in Models

**All database queries must live in `domains/*/models/` modules, not in effects or handlers.**

Models are the data access layer. Effects are thin orchestrators that call actions. Never put SQL queries directly in effect handlers.

#### ✅ Correct Pattern:

```rust
// domains/crawling/models/extraction_page.rs
pub struct ExtractionPage;

impl ExtractionPage {
    pub async fn find_by_domain(domain: &str, pool: &PgPool) -> Result<Vec<(Uuid, String, String)>> {
        sqlx::query_as::<_, (String, String)>(
            "SELECT url, content FROM extraction_pages WHERE site_url = $1"
        )
        .bind(domain)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}
```

Then in the effect handler:
```rust
// domains/crawling/effects/handlers.rs
let pages = ExtractionPage::find_by_domain(&website.domain, &ctx.deps().db_pool).await?;
```

#### ❌ Incorrect Pattern:

```rust
// BAD: SQL query directly in effect handler
async fn handle_extract_posts(ctx: &EffectContext<...>) -> Result<()> {
    let pages = sqlx::query_as::<_, (String, String)>(
        "SELECT url, content FROM extraction_pages WHERE site_url = $1"
    )
    .bind(&domain)
    .fetch_all(&ctx.deps().db_pool)
    .await?;
    // ...
}
```

#### Why This Rule Exists:

- **Single source of truth**: All queries for a table are in one place
- **Reusability**: Multiple handlers can use the same query methods
- **Testability**: Models can be unit tested without effect machinery
- **Discoverability**: Easy to find all queries for a given table

---

## GraphQL Mutation Pattern

### Simple CRUD Mutations Call Activities Directly

GraphQL mutations call activity functions directly — no framework needed for simple operations:

```rust
async fn approve_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<PostType> {
    let user = ctx.auth_user.as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    let id = post_activities::approve_post(post_id, user.member_id.into_uuid(), user.is_admin, ctx.deps())
        .await
        .map_err(to_field_error)?;

    let post = Post::find_by_id(id, &ctx.db_pool).await.map_err(to_field_error)?;
    Ok(PostType::from(post))
}
```

### Multi-Step Operations Use Restate Workflows

For operations that need durability or multi-step orchestration, invoke a Restate workflow:

```rust
async fn register_member(ctx: &GraphQLContext, ...) -> FieldResult<MemberType> {
    let result: RegisterMemberResult = ctx.workflow_client
        .invoke("RegisterMember", "run", RegisterMemberRequest { ... })
        .await
        .map_err(to_field_error)?;

    let member = Member::find_by_id(result.member_id, &ctx.db_pool).await?;
    Ok(MemberType::from(member))
}
```

### Activities Are Pure Functions

Activities take `&ServerDeps` explicitly and return simple values:

```rust
pub async fn approve_post(post_id: PostId, reviewer: MemberId, is_admin: bool, deps: &ServerDeps) -> Result<PostId> {
    // Auth check, business logic, return data
}
```

## Restate Workflow Architecture (v0.2.0)

### Overview

We use Restate for durable workflow execution. Restate provides:
- Durable async/await - workflows survive process restarts
- At-least-once execution guarantees
- Built-in retry and recovery
- HTTP-based invocation

### Architecture

```
GraphQL API (port 8080)
    ↓ HTTP
WorkflowClient
    ↓ HTTP (port 9070)
Restate Runtime (proxy/gateway)
    ↓ HTTP (port 9080)
Workflow Server (actual implementations)
```

### Workflow Pattern

#### 1. Define Workflow Trait with Macro

```rust
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendOtpRequest {
    pub phone_number: String,
}

impl_restate_serde!(SendOtpRequest);

#[restate_sdk::workflow]
pub trait SendOtpWorkflow {
    async fn run(request: SendOtpRequest) -> Result<OtpSent, HandlerError>;
}
```

**Key points:**
- Trait signature does NOT include `&self` or `ctx` - those are added in the impl
- Request/response types must implement Restate's custom serialization via `impl_restate_serde!` macro

#### 2. Implement Workflow with Dependencies

```rust
pub struct SendOtpWorkflowImpl {
    deps: Arc<ServerDeps>,
}

impl SendOtpWorkflowImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl SendOtpWorkflow for SendOtpWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        request: SendOtpRequest,
    ) -> Result<OtpSent, HandlerError> {
        // Durable execution - wrapped in ctx.run()
        let result = ctx
            .run(|| async {
                activities::send_otp(request.phone_number.clone(), &self.deps)
                    .await
                    .map_err(Into::into)
            })
            .await?;

        Ok(result)
    }
}
```

**Key points:**
- Impl struct contains `Arc<ServerDeps>` for dependencies
- Constructor `with_deps()` accepts Arc for efficient cloning
- Implementation adds `&self` and `ctx: WorkflowContext<'_>` parameters
- Business logic wrapped in `ctx.run()` for durability

#### 3. Register Workflows in Server

```rust
// server.rs
let server_deps = Arc::new(ServerDeps::new(...));

let endpoint = Endpoint::builder()
    .bind(SendOtpWorkflowImpl::with_deps(server_deps.clone()).serve())
    .bind(VerifyOtpWorkflowImpl::with_deps(server_deps.clone()).serve())
    .bind(CrawlWebsiteWorkflowImpl::with_deps(server_deps.clone()).serve())
    .build();

HttpServer::new(endpoint)
    .listen_and_serve("0.0.0.0:9080".parse()?)
    .await;
```

**Key points:**
- Create Arc once, clone for each workflow (cheap)
- Call `.serve()` on workflow instance - macro generates this method
- Must import both trait and impl for `.serve()` to be in scope

#### 4. Invoke from GraphQL

```rust
// GraphQL mutation
async fn send_verification_code(
    ctx: &GraphQLContext,
    phone_number: String,
) -> FieldResult<bool> {
    use crate::domains::auth::restate::SendOtpRequest;
    use crate::domains::auth::types::OtpSent;

    let result: OtpSent = ctx
        .workflow_client
        .invoke("SendOtp", "run", SendOtpRequest { phone_number })
        .await
        .map_err(to_field_error)?;

    Ok(result.success)
}
```

**Key points:**
- WorkflowClient makes HTTP calls to Restate runtime
- Service name is PascalCase trait name without "Workflow" suffix
- Handler name is the method name (usually "run")

### Custom Serialization (impl_restate_serde!)

Restate SDK has its own Serialize/Deserialize traits (NOT serde's). Use the macro:

```rust
// common/restate_serde.rs
#[macro_export]
macro_rules! impl_restate_serde {
    ($type:ty) => {
        impl restate_sdk::serde::Serialize for $type {
            type Error = serde_json::Error;
            fn serialize(&self) -> Result<bytes::Bytes, Self::Error> {
                serde_json::to_vec(self).map(bytes::Bytes::from)
            }
        }
        impl restate_sdk::serde::Deserialize for $type {
            type Error = serde_json::Error;
            fn deserialize(bytes: &mut bytes::Bytes) -> Result<Self, Self::Error> {
                serde_json::from_slice(bytes)
            }
        }
        impl restate_sdk::serde::WithContentType for $type {
            fn content_type() -> &'static str {
                "application/json"
            }
        }
    };
}
```

Apply to all workflow request/response types:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtpSent {
    pub phone_number: String,
    pub success: bool,
}

impl_restate_serde!(OtpSent);
```

### Workflow Best Practices

1. **Keep workflows thin** - Orchestrate activities, don't implement business logic directly
2. **Activities are pure functions** - Take deps explicitly, return simple data types
3. **No events** - Workflows return data types directly, not domain events
4. **Wrap in ctx.run()** - All external calls must be in durable blocks
5. **Use Arc for deps** - Efficient cloning across workflows
6. **Import traits for .serve()** - Both trait and impl must be in scope

### Binary

- **server** (`src/bin/server.rs`) - Restate services + SSE streaming

Run in development:
```bash
# Terminal 1: Start Restate runtime + infrastructure
docker-compose up -d postgres redis nats restate

# Terminal 2: Start server
cargo run --bin server

# Terminal 3: Register with Restate
./scripts/register-workflows.sh
```

---

## Coding Permission Rule

### HARD RULE: Never Code Without Explicit Permission

**NEVER start writing code, editing files, or making changes until the user explicitly says "go", "do it", "proceed", or similar explicit approval.**

When given a task:
1. **First**: Present the plan, explain what you'll do
2. **Wait**: For explicit user approval
3. **Only then**: Execute the plan

This applies to:
- Creating new files
- Editing existing files
- Running commands that modify state
- Any code generation

**Reading files for research is OK. Writing/editing is NOT until approved.**

Phrases that grant permission: "go", "do it", "proceed", "yes", "approved", "start coding", "make the changes"

Phrases that do NOT grant permission: silence, "sounds good" (without action word), "interesting", questions about the plan
