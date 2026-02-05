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

## Database Migration Rules

### HARD RULE: NEVER Modify Existing Migration Scripts

**NEVER, under ANY circumstances, edit or modify an existing migration file. ALWAYS create a new migration file instead.**

Once a migration file has been created, it is immutable. SQLx tracks migrations by their checksum - if you modify a migration that has already been applied, it will cause deployment failures with "migration was previously applied but has been modified" errors.

#### What To Do Instead:

1. **Need to fix a mistake?** Create a NEW migration file with the fix
2. **Need to add something you forgot?** Create a NEW migration file
3. **Need to change a column type?** Create a NEW migration file with `ALTER TABLE`
4. **Need to rename something?** Create a NEW migration file

#### Example:

If migration `000057_add_users.sql` is missing a column:

```sql
-- ❌ WRONG: DO NOT edit 000057_add_users.sql

-- ✅ CORRECT: Create 000058_add_user_email.sql
ALTER TABLE users ADD COLUMN email TEXT;
```

#### Why This Rule Exists:

- Migrations are checksummed by SQLx
- Modifying applied migrations breaks the checksum
- This causes "migration was previously applied but has been modified" errors
- Recovering from this requires manual database intervention
- It can cause data loss in production

**NO EXCEPTIONS. EVER.**

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

## Seesaw Architecture Rules (v0.7.3)

### Overview

Seesaw uses an event-driven architecture with these main components:

1. **Actions** - Reusable business logic in `domains/*/actions/` modules
2. **Effects** - Event handlers that run in response to domain events
3. **GraphQL Integration** - Thin mutations that call actions via `process()`

### CRITICAL: GraphQL Mutations Must Be Thin

**All GraphQL mutations and queries that invoke actions MUST use the `process()` pattern.**

The `process()` method is the synchronous gateway to call actions:
- Activates the engine with app state
- Executes the closure and returns its result
- Ensures events are emitted to the engine

#### ✅ Correct GraphQL Pattern (v0.7.3):

```rust
async fn approve_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<PostType> {
    use crate::domains::posts::events::PostEvent;

    // 1. Auth check at GraphQL layer
    let user = ctx.auth_user.as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

    // 2. Call action via process() - returns the event
    let event = ctx.engine
        .activate(ctx.app_state())
        .process(|ectx| {
            post_actions::approve_post(post_id, user.member_id.into_uuid(), user.is_admin, ectx.deps())
        })
        .await
        .map_err(to_field_error)?;

    // 3. Extract data from event and return
    let PostEvent::PostApproved { post_id } = event else {
        return Err(FieldError::new("Unexpected event type", juniper::Value::null()));
    };

    let post = Post::find_by_id(post_id, &ctx.db_pool).await.map_err(to_field_error)?;
    Ok(PostType::from(post))
}
```

#### ❌ Incorrect Pattern (Never Do This):

```rust
// BAD: Calling action directly without process()
async fn approve_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<PostType> {
    // Wrong: No event emitted to engine!
    post_actions::approve_post(post_id, user_id, is_admin, &ctx.deps()).await?;
    // ...
}
```

### Key Patterns for v0.7.3:

1. **`process()` returns the closure's return value** - Actions return events, `process()` returns them to the caller
2. **Actions take `&ServerDeps`** - NOT `&EffectContext`
3. **All business logic inside actions** - GraphQL mutations are thin wrappers
4. **Events contain the data you need** - e.g., `AuthEvent::OTPVerified { token, member_id, ... }`

---

### PLATINUM RULE: Events Are Facts Only

**Events must represent facts about what happened. Never emit events for failures, errors, or hypotheticals.**

- ✅ `PostsRegenerated` - fact: posts were regenerated
- ✅ `WebsiteCrawled` - fact: a website was crawled
- ✅ `UserCreated` - fact: a user was created
- ❌ `CrawlFailed` - not a fact, it's an error (use `Result::Err`)
- ❌ `WebsiteIngested` - misleading if you're regenerating, not ingesting
- ❌ `ProcessingStarted` - not a fact about what happened, it's a status

**Errors go in `Result::Err`, not in events.** If an operation fails, return an error. Events are for successful state changes that other parts of the system may need to react to.

**Be precise about what happened.** Don't emit `PostsSynced` when posts were regenerated. Don't emit `WebsiteIngested` when you extracted posts from already-ingested pages. The event name must accurately describe the fact.

### HARD RULE: Effects Must Be Ultra-Thin

**Effect handlers should only: (1) check authorization, (2) call an action, (3) return an event.**

Handlers must be <50 lines. All business logic lives in actions modules.

#### ✅ Correct Pattern (Ultra-Thin Handler):

```rust
// Effect dispatches to thin handler
async fn handle_regenerate_posts(
    page_snapshot_id: Uuid,
    job_id: JobId,
    requested_by: MemberId,
    is_admin: bool,
    ctx: &EffectContext<ServerDeps>,
) -> Result<()> {
    // 1. Auth check (using reusable action) - returns Result, not event
    actions::check_crawl_authorization(
        requested_by, is_admin, "RegeneratePosts", ctx.deps()
    ).await?;

    // 2. Delegate to action (all business logic lives there)
    let posts_count = actions::regenerate_posts_for_page(
        page_snapshot_id, job_id, ctx.deps()
    ).await;

    // 3. Emit fact event
    ctx.emit(CrawlEvent::PagePostsRegenerated { page_snapshot_id, job_id, posts_count });
    Ok(())
}
```

#### ❌ Incorrect Pattern (Thick Handler):

```rust
// BAD: All this logic should be in an action!
async fn handle_regenerate_posts(...) -> Result<CrawlEvent> {
    // Auth check inline (should be reusable action)
    if let Err(auth_err) = Actor::new(requested_by, is_admin)
        .can(AdminCapability::TriggerScraping)
        .check(ctx.deps())
        .await
    { ... }

    // Multiple circuit breakers inline (should be in action)
    let page_snapshot = PageSnapshot::find_by_id(...).await?;
    let website_snapshot = WebsiteSnapshot::find_by_page_snapshot_id(...).await?;
    let website = Website::find_by_id(...).await?;

    // ... 100 more lines ...
}
```

---

### HARD RULE: Business Logic Lives in Actions

**Create reusable actions in `domains/*/actions/` modules.**

Actions are pure business logic functions that:
- Take dependencies explicitly (no EffectContext)
- Return simple values (count, bool, struct) NOT events
- Can be composed and reused across handlers
- Are easy to unit test

#### Actions Module Structure:

```
domains/crawling/actions/
├── mod.rs                 # Re-exports
├── authorization.rs       # check_crawl_authorization()
├── crawl_website.rs       # crawl_website_pages(), store_crawled_pages()
├── build_pages.rs         # build_pages_to_summarize(), fetch_single_page_context()
├── extract_posts.rs       # extract_posts_from_pages()
├── sync_posts.rs          # sync_and_deduplicate_posts()
└── website_context.rs     # fetch_approved_website()
```

#### ✅ Correct Action Pattern:

```rust
// actions/regenerate_page.rs

/// Workflow action that consolidates multiple operations.
/// Returns simple value (count), NOT an event.
pub async fn regenerate_posts_for_page(
    page_snapshot_id: Uuid,
    job_id: JobId,
    deps: &ServerDeps,  // Takes deps, not EffectContext
) -> usize {
    // Early return on failure - no event needed
    let Some(ctx) = fetch_single_page_context(page_snapshot_id, &deps.db_pool).await else {
        return 0;
    };

    let page = build_page_to_summarize_from_snapshot(&ctx.page_snapshot, ctx.page_snapshot.url.clone());

    let result = match extract_posts_from_pages(&ctx.website, vec![page], job_id, deps.ai.as_ref(), deps).await {
        Ok(r) if !r.posts.is_empty() => r,
        _ => return 0,
    };

    let count = result.posts.len();
    let _ = sync_and_deduplicate_posts(ctx.website_id, result.posts, deps).await;
    count
}
```

#### ✅ Correct Reusable Auth Action:

```rust
// actions/authorization.rs

/// Reusable authorization check - replaces 5+ identical blocks.
/// Returns Result<()>, NOT Result<(), Event> - auth failures are errors, not events.
pub async fn check_crawl_authorization<D: HasAuthContext>(
    requested_by: MemberId,
    is_admin: bool,
    action_name: &str,
    deps: &D,
) -> Result<()> {
    Actor::new(requested_by, is_admin)
        .can(AdminCapability::TriggerScraping)
        .check(deps)
        .await
        .map_err(|auth_err| {
            anyhow::anyhow!("Authorization denied for {}: {}", action_name, auth_err)
        })
}
```

---

### Edges Can Run Business Logic

In Seesaw 0.4.0+, edges can execute business logic during event-to-command transitions.

```rust
// Edge that runs logic before emitting command
edge!(
    CrawlEvent::PagesReadyForExtraction { website_id, job_id, pages } =>
    async |event, deps| {
        // Can run business logic here
        let priorities = actions::get_crawl_priorities(website_id, &deps.db_pool).await;

        CrawlCommand::ExtractFromPages {
            website_id,
            job_id,
            pages,
            priorities,
        }
    }
);
```

---

### Key Principles

| Component | Responsibility | Size Limit |
|-----------|---------------|------------|
| Effect `execute()` | Route command to handler | ~5 lines |
| Handler | Auth check → action → event | <50 lines |
| Action | All business logic | No limit (but keep focused) |
| Edge | Event → Command mapping, can run logic | Keep simple |

### Benefits of This Architecture:

- **Testability**: Actions are pure functions, easy to unit test
- **Reusability**: Actions called from multiple handlers (DRY)
- **Clarity**: Clear separation of routing vs logic
- **Maintainability**: Find business logic in actions/, not scattered in effects

### Common Patterns:

1. **Workflow actions** return simple values (`usize`, `bool`, `Option<T>`)
2. **Context helpers** like `fetch_single_page_context()` consolidate repeated lookups
3. **Auth actions** like `check_crawl_authorization()` return `Result<()>` and use `?` operator
4. **Auth failures are errors**, not events - use `anyhow::anyhow!` or custom error types

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
