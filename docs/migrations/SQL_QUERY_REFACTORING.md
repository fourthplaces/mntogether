# SQL Query Refactoring Summary

All SQL queries have been moved from effect handlers to their corresponding model files, following the established pattern of separation of concerns.

## Pattern

### Before (queries in handlers)

```rust
// BAD: SQL query directly in handler
async fn handle_event(ctx: &Context) -> Result<()> {
    let posts = sqlx::query_as::<_, Post>(
        "SELECT * FROM posts WHERE status = $1"
    )
    .bind("active")
    .fetch_all(&ctx.deps().db_pool)
    .await?;
    // ...
}
```

### After (queries in models)

```rust
// GOOD: Query method on model
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

// Handler calls model method
let posts = Post::find_active(&deps.db_pool).await?;
```

## Benefits

1. **Separation of Concerns**: Handlers orchestrate business logic, models handle data access
2. **Reusability**: Model methods can be used from anywhere (handlers, GraphQL resolvers, etc.)
3. **Testability**: Models can be tested independently
4. **Maintainability**: SQL queries are centralized and easier to update
5. **Type Safety**: Model methods provide clear interfaces for data operations

## Key Rule

**All database queries must live in `domains/*/models/` modules.** This is enforced in CLAUDE.md and documented in [INSTITUTIONAL_LEARNINGS.md](../INSTITUTIONAL_LEARNINGS.md).
