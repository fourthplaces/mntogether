# Institutional Learnings & Historical Context

This document captures documented learnings, gotchas, and architectural decisions from the codebase. It serves as a knowledge base for avoiding repeated mistakes and understanding why certain patterns exist.

---

## 1. SQL Query Rules (CRITICAL)

### HARD RULE: Never use `sqlx::query_as!` macro
**Always use function version: `sqlx::query_as::<_, Type>`**

**Why:**
- Macro requires compile-time database access (fragile)
- Type inference issues with nullable fields (Option<Option<T>>)
- Slower compilation
- Less flexible for complex queries

**Correct Pattern:**
```rust
#[derive(FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub metadata: Option<serde_json::Value>,
}

impl User {
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM users WHERE id = $1"
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }
}
```

### HARD RULE: NEVER Modify Existing Migration Scripts
**ALWAYS create a new migration file instead.**

**Why:**
- SQLx checksums migrations
- Modifying applied migrations breaks checksum
- Causes: "migration was previously applied but has been modified" errors
- Recovery requires manual database intervention
- **Can cause data loss in production**

**Example:**
```sql
-- WRONG: Edit 000057_add_users.sql to add missing column

-- CORRECT: Create 000058_add_user_email.sql
ALTER TABLE users ADD COLUMN email TEXT;
```

---

## 2. Database Schema Design (CRITICAL)

### HARD RULE: Avoid JSONB Columns
**Normalize data into proper relational tables instead.**

**Why JSONB is bad:**
- Harder to query: `WHERE search_results->>'title' = ...` (awkward syntax)
- No foreign key constraints
- No type safety in Rust models
- Can't add proper indexes on nested fields
- Requires JSON parsing in application code

**Anti-Pattern (DO NOT USE):**
```sql
CREATE TABLE domains (
    id UUID PRIMARY KEY,
    search_results JSONB  -- AVOID!
);
```

**Correct Pattern (Normalized):**
```sql
CREATE TABLE search_queries (
    id UUID PRIMARY KEY,
    domain_id UUID REFERENCES domains(id),
    query TEXT NOT NULL
);

CREATE TABLE search_results (
    id UUID PRIMARY KEY,
    query_id UUID REFERENCES search_queries(id),
    title TEXT NOT NULL,
    url TEXT NOT NULL,
    score DECIMAL(3,2)
);
```

**When JSONB is Acceptable (Rare):**
1. Truly unstructured data with unknown schema (arbitrary metadata)
2. External API responses you don't control (audit trail only)
3. Configuration objects with frequent schema changes

---

## 3. Domain Architecture Patterns

### Activities Must Be Pure Functions

Activities take `&ServerDeps` explicitly and return simple values. All business logic lives in activities, not in Restate handlers.

```rust
pub async fn approve_post(
    post_id: PostId,
    reviewer: MemberId,
    is_admin: bool,
    deps: &ServerDeps,
) -> Result<PostId> {
    // Auth check, business logic, return data
}
```

### Database Queries Live in Models

**All database queries must live in `domains/*/models/` modules, not in activities or handlers.**

**Why:**
- **Single source of truth**: All queries for a table are in one place
- **Reusability**: Multiple handlers can use the same query methods
- **Testability**: Models can be unit tested without workflow machinery
- **Discoverability**: Easy to find all queries for a given table

**Correct Pattern:**
```rust
// domains/posts/models/post.rs
impl Post {
    pub async fn find_active(pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM posts WHERE status = 'active' ORDER BY created_at DESC"
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}
```

Then in the activity:
```rust
let posts = Post::find_active(&deps.db_pool).await?;
```

### ServerDeps Pattern

`ServerDeps` is the central dependency container, stored as `Arc<ServerDeps>`. It holds:
- `db_pool` — PostgreSQL connection pool
- `redis` — Redis client (caching)
- `twilio` — Twilio service (OTP)
- `jwt_service` — JWT token management
- `ai_client` — LLM client
- Admin identifiers list

All workflow implementations receive `Arc<ServerDeps>` via their constructor.

---

## 4. Specific Known Issues & Gotchas

### TwilioService Abstraction Underutilized
**Issue**: `ServerDeps` uses concrete `TwilioService` instead of `BaseTwilioService` trait
- Makes testing harder (can't easily mock)
- Breaks dependency inversion principle

**Solution**: Change field type from concrete to trait object
```rust
// BEFORE
pub twilio: Arc<TwilioService>,

// AFTER
pub twilio: Arc<dyn BaseTwilioService>,
```

---

## 5. Best Practices Summary

### Do's
- Use `sqlx::query_as::<_, Type>` (function version)
- Keep activities as pure functions (take deps, return data)
- Put all SQL queries in model modules
- Use trait objects for abstraction (Arc<dyn Trait>)
- Normalize database schema (relational tables)
- Create new migrations (never modify old ones)
- Use Option<T> for nullable fields (not special annotations)
- Define structured errors (not string errors)

### Don'ts
- Use `sqlx::query_as!` macro
- Modify existing migration files
- Put SQL queries in activities or handlers
- JSONB for structured data
- Concrete types where traits should be used
- String-based error messages

---

## 6. How to Use This Document

### Before Starting Work
1. **Read the relevant section** for your feature type
2. **Follow the patterns** documented here
3. **Reference the CLAUDE.md** rules for confirmed patterns

### When You Discover Something New
1. **Document it** in the appropriate section
2. **Add to CLAUDE.md** if it's a HARD RULE for the future
3. **Create a plan document** if it's a refactor

---

## References

### Key Documentation
- **CLAUDE.md** - Hard rules and patterns (MUST READ)
- **docs/architecture/ROOT_EDITORIAL_PIVOT.md** - Pivot bible: what survives, what's dead
- **docs/architecture/DOMAIN_ARCHITECTURE.md** - Domain layer structure

### Code Examples
- `domains/auth/activities/` - Simple activities
- `domains/member/models/` - Model-based queries
- `domains/posts/models/` - Post lifecycle queries

---

**Last Updated**: February 24, 2026
**Document Status**: Active — scrubbed for Root Editorial pivot
