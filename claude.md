# Claude Code Development Rules

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

## Seesaw Effect Rules

### HARD RULE: Effects Must Be Thin Orchestration Layers

**Effects should dispatch to handler functions, not contain business logic directly.**

Effects are the integration point between Seesaw's event-driven architecture and your business logic. They should be thin dispatchers that route commands to handler functions containing the actual logic.

#### ✅ Correct Pattern (Thin Effect):

```rust
use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};

pub struct ResearchEffect;

#[async_trait]
impl Effect<DomainApprovalCommand, ServerDeps> for ResearchEffect {
    type Event = DomainApprovalEvent;

    async fn execute(
        &self,
        cmd: DomainApprovalCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<DomainApprovalEvent> {
        match cmd {
            DomainApprovalCommand::FetchOrCreateResearch {
                domain_id,
                job_id,
                requested_by,
            } => {
                // Effect just dispatches to handler
                handle_fetch_or_create_research(domain_id, job_id, requested_by, &ctx).await
            }
        }
    }
}

// ============================================================================
// Handler Functions (Business Logic)
// ============================================================================

async fn handle_fetch_or_create_research(
    domain_id: DomainId,
    job_id: JobId,
    requested_by: MemberId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<DomainApprovalEvent> {
    // All business logic lives here
    info!(domain_id = %domain_id, "Fetching or creating research");

    // Check for existing research
    let existing = DomainResearch::find_latest_by_domain_id(
        domain_id.into(),
        &ctx.deps().db_pool,
    )
    .await?;

    if let Some(research) = existing {
        let age_days = (chrono::Utc::now() - research.created_at).num_days();
        if age_days < 7 {
            return Ok(DomainApprovalEvent::DomainResearchFound {
                research_id: research.id,
                domain_id,
                job_id,
                age_days,
            });
        }
    }

    // Create new research...
    // (rest of logic)
}
```

#### ❌ Incorrect Pattern (DO NOT USE):

```rust
// NEVER PUT BUSINESS LOGIC DIRECTLY IN EFFECT!
#[async_trait]
impl Effect<DomainApprovalCommand, ServerDeps> for ResearchEffect {
    type Event = DomainApprovalEvent;

    async fn execute(
        &self,
        cmd: DomainApprovalCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<DomainApprovalEvent> {
        match cmd {
            DomainApprovalCommand::FetchOrCreateResearch {
                domain_id,
                job_id,
                requested_by,
            } => {
                // BAD: All this logic should be in a handler function!
                info!(domain_id = %domain_id, "Fetching or creating research");

                let existing = DomainResearch::find_latest_by_domain_id(
                    domain_id.into(),
                    &ctx.deps().db_pool,
                )
                .await?;

                // ... 100 more lines of logic ...
            }
        }
    }
}
```

### Key Points:

1. **Effects are thin dispatchers**: They only route commands to handlers
2. **Handler functions contain business logic**: All actual work happens here
3. **One handler per command variant**: Clear 1:1 mapping
4. **Handler signature**: `async fn handle_xyz(..., ctx: &EffectContext<ServerDeps>) -> Result<Event>`
5. **Testability**: Handler functions are easier to unit test
6. **Readability**: Separates routing from logic

### Benefits:

- **Testability**: Test handler functions without Effect trait overhead
- **Reusability**: Handler functions can be called from other contexts
- **Clarity**: Clear separation between routing and business logic
- **Maintainability**: Easier to find and modify specific business logic

### Example: Multi-Command Effect

```rust
pub struct DomainApprovalCompositeEffect {
    research: ResearchEffect,
    search: SearchEffect,
    assessment: AssessmentEffect,
}

#[async_trait]
impl Effect<DomainApprovalCommand, ServerDeps> for DomainApprovalCompositeEffect {
    type Event = DomainApprovalEvent;

    async fn execute(
        &self,
        cmd: DomainApprovalCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<DomainApprovalEvent> {
        // Composite effect just routes to specialized effects
        match &cmd {
            DomainApprovalCommand::FetchOrCreateResearch { .. } => {
                self.research.execute(cmd, ctx).await
            }
            DomainApprovalCommand::ConductResearchSearches { .. } => {
                self.search.execute(cmd, ctx).await
            }
            DomainApprovalCommand::GenerateAssessmentFromResearch { .. } => {
                self.assessment.execute(cmd, ctx).await
            }
        }
    }
}
```
