# Integration Tests

This directory contains integration tests following patterns from Shay's api-core.

## Architecture

### Test Harness Pattern

Tests use the **test harness pattern** with shared containers for fast execution:

```rust
#[test_context(TestHarness)]
#[tokio::test]
async fn my_test(ctx: &TestHarness) {
    let client = ctx.graphql();
    // ... test code
}
```

**Key benefits:**
- ✅ Containers started once, reused across all tests
- ✅ Migrations run once on first test
- ✅ Each test gets fresh context with clean state
- ✅ Tests run in parallel safely

### Testing at the Edges

Following Shay's approach, we **test at the edges** (GraphQL layer) rather than testing individual functions:

```rust
// ❌ BAD: Testing internal functions directly
#[test]
fn test_generate_content_hash() { ... }

// ✅ GOOD: Testing through GraphQL API
#[test_context(TestHarness)]
#[tokio::test]
async fn approve_need_changes_status(ctx: &TestHarness) {
    let client = ctx.graphql();
    let result = client.query(mutation).await;
    // Test actual behavior users experience
}
```

**Why test at the edges?**
- Tests actual user-facing behavior
- Catches integration issues
- More resilient to refactoring (tests don't break when internal implementation changes)
- Validates the full stack (GraphQL → business logic → database)

### Dependency Injection

The test harness uses **dependency injection** to provide mock services:

```rust
pub struct TestHarness {
    pub db_pool: PgPool,
    pub firecrawl_api_key: String,  // Can be mocked
    pub openai_api_key: String,     // Can be mocked
}
```

This allows:
- Testing without real API calls
- Faster test execution
- Deterministic test results

## Running Tests

```bash
# Run all tests
cargo test

# Run with logging
RUST_LOG=debug cargo test -- --nocapture

# Run specific test file
cargo test --test organization_needs_tests

# Run single test
cargo test approve_need_changes_status

# Run tests in series (helpful for debugging)
cargo test -- --test-threads=1
```

## Test Structure

```
tests/
├── common/
│   ├── harness.rs      # TestHarness with shared containers
│   ├── graphql.rs      # GraphQL client for tests
│   ├── fixtures.rs     # Helper functions for creating test data
│   └── mod.rs          # Exports and vars! macro
│
├── organization_needs_tests.rs  # Need queries, mutations, approval workflow
├── content_hash_tests.rs        # Content hash unit tests
└── README.md                     # This file
```

## Writing Tests

### 1. Query Test

```rust
#[test_context(TestHarness)]
#[tokio::test]
async fn query_needs_returns_active(ctx: &TestHarness) {
    let client = ctx.graphql();

    // Setup: Create test data
    create_test_need_active(&ctx.db_pool, "Title", "Description")
        .await
        .unwrap();

    // Execute: Run GraphQL query
    let query = r#"
        query GetNeeds {
            needs { nodes { id title } }
        }
    "#;
    let result = client.query(query).await;

    // Assert: Check results
    assert_eq!(result["needs"]["nodes"][0]["title"].as_str().unwrap(), "Title");
}
```

### 2. Mutation Test

```rust
#[test_context(TestHarness)]
#[tokio::test]
async fn approve_need_works(ctx: &TestHarness) {
    let client = ctx.graphql();

    // Setup
    let need_id = create_test_need_pending(&ctx.db_pool, None, "Test", "Desc")
        .await
        .unwrap();

    // Execute
    let mutation = r#"
        mutation ApproveNeed($needId: ID!) {
            approveNeed(needId: $needId) { status }
        }
    "#;
    let result = client
        .query_with_vars(mutation, vars!("needId" => need_id.to_string()))
        .await;

    // Assert
    assert_eq!(result["approveNeed"]["status"].as_str().unwrap(), "ACTIVE");
}
```

### 3. Integration Test with Database Verification

```rust
#[test_context(TestHarness)]
#[tokio::test]
async fn reject_need_updates_database(ctx: &TestHarness) {
    let client = ctx.graphql();

    let need_id = create_test_need_pending(&ctx.db_pool, None, "Test", "Desc")
        .await
        .unwrap();

    // Execute mutation
    client
        .query_with_vars(
            mutation,
            vars!("needId" => need_id.to_string(), "reason" => "spam")
        )
        .await;

    // Verify in database
    let row = sqlx::query!("SELECT status FROM organization_needs WHERE id = $1", need_id)
        .fetch_one(&ctx.db_pool)
        .await
        .unwrap();

    assert_eq!(row.status.as_deref().unwrap(), "rejected");
}
```

## Test Coverage

### Current Coverage

- ✅ Need queries (active, pending_approval, pagination)
- ✅ Need details by ID
- ✅ Approve need (human-in-the-loop)
- ✅ Edit and approve need (fix AI mistakes)
- ✅ Reject need (hide spam)
- ✅ Content hash generation (deduplication)

### TODO: Add Tests

- [ ] Scrape organization source
- [ ] Sync needs (new/changed/disappeared)
- [ ] Volunteer registration
- [ ] Push notification sending

## Fixtures

Use fixture helpers from `tests/common/fixtures.rs`:

```rust
// Create active need
let need_id = create_test_need_active(&ctx.db_pool, "Title", "Description")
    .await
    .unwrap();

// Create pending need
let need_id = create_test_need_pending(&ctx.db_pool, Some(source_id), "Title", "Description")
    .await
    .unwrap();

// Clean database between tests
clean_needs(&ctx.db_pool).await.unwrap();
clean_sources(&ctx.db_pool).await.unwrap();
```

## Debugging Tests

### Enable Logging

```bash
RUST_LOG=debug,sqlx=debug cargo test -- --nocapture
```

### Run Single Test

```bash
cargo test approve_need_changes_status -- --nocapture
```

### Inspect Database After Test Failure

Tests use shared containers that persist during the test run. If a test fails, you can inspect the database:

```bash
# Get container ID
docker ps | grep postgres

# Connect to database
docker exec -it <container_id> psql -U postgres postgres
```

## Performance

**First test run:** ~2-3 seconds (containers start + migrations)
**Subsequent tests:** ~100-200ms per test (containers reused)

The shared container approach makes tests **10-20x faster** than starting fresh containers per test.
