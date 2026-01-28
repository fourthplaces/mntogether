# Integration Test Setup

This document describes the integration test infrastructure following patterns from the shay project.

## Overview

The integration test system uses:
- **Trait-based dependency injection** for testability
- **Shared testcontainers** for fast test execution
- **ServerKernel** for managing all dependencies
- **TestDependencies** for easy mock injection
- **Seesaw event bus** integration for testing event-driven flows

## Architecture

### Trait-Based Dependency Injection

All external services are abstracted behind traits in `src/kernel/traits.rs`:

- `BaseWebScraper` - Web scraping (Firecrawl)
- `BaseNeedExtractor` - AI need extraction (OpenAI)
- `BaseEmbeddingService` - Text embeddings (OpenAI)
- `BasePushNotificationService` - Push notifications (Expo)

**Naming convention**: Base* for trait names (e.g., `BaseWebScraper`, `BaseNeedExtractor`)

### ServerKernel

`src/kernel/server_kernel.rs` holds all server dependencies:

```rust
pub struct ServerKernel {
    pub db_pool: PgPool,
    pub web_scraper: Arc<dyn BaseWebScraper>,
    pub need_extractor: Arc<dyn BaseNeedExtractor>,
    pub embedding_service: Arc<dyn BaseEmbeddingService>,
    pub push_service: Arc<dyn BasePushNotificationService>,
    pub bus: EventBus,
    pub job_queue: Arc<dyn JobQueue>,
}
```

### TestDependencies

`src/kernel/test_dependencies.rs` provides mock implementations:

```rust
let deps = TestDependencies::new()
    .mock_scraper(MockWebScraper::new().with_response("# Test Content"))
    .mock_extractor(MockNeedExtractor::new().with_single_need("Title", "Description"));
```

Available mocks:
- `MockWebScraper` - Returns pre-configured scrape results
- `MockNeedExtractor` - Returns pre-configured needs
- `MockEmbeddingService` - Returns fixed embedding vectors
- `MockPushNotificationService` - Records sent notifications

### TestHarness

`tests/common/harness.rs` manages test infrastructure:

```rust
#[test_context(TestHarness)]
#[tokio::test]
async fn my_test(ctx: &TestHarness) {
    let client = ctx.graphql();
    // ... test code
}
```

Features:
- Shared containers (Postgres, Redis) across all tests
- Per-test kernel with test dependencies
- Domain engines running in background
- `settle()` method to wait for event processing
- `wait_for()` method for condition polling

## Writing Integration Tests

### Basic Test

```rust
use crate::common::TestHarness;
use test_context::test_context;

#[test_context(TestHarness)]
#[tokio::test]
async fn test_query_needs(ctx: &TestHarness) {
    let client = ctx.graphql();

    let query = r#"
        query {
            needs {
                nodes {
                    id
                    title
                }
            }
        }
    "#;

    let result = client.query(query).await;
    assert!(result["needs"]["nodes"].is_array());
}
```

### Test with Custom Mocks

```rust
use server_core::kernel::{MockNeedExtractor, TestDependencies};

#[tokio::test]
async fn test_with_mock_extractor() {
    let deps = TestDependencies::new()
        .mock_extractor(
            MockNeedExtractor::new()
                .with_single_need("Volunteers Needed", "Help us!")
        );

    let harness = TestHarness::with_deps(deps).await.unwrap();
    let client = harness.graphql();

    // ... test with mocked need extractor
}
```

### Test with Event Bus

```rust
#[test_context(TestHarness)]
#[tokio::test]
async fn test_event_driven_flow(ctx: &TestHarness) {
    // Trigger command
    let bus = ctx.bus();
    bus.emit(OrganizationCommand::ScrapeSource { ... }).await;

    // Wait for events to process
    ctx.settle().await;

    // Verify results
    let needs = sqlx::query!("SELECT * FROM organization_needs")
        .fetch_all(&ctx.db_pool)
        .await
        .unwrap();

    assert_eq!(needs.len(), 1);
}
```

## Running Tests

```bash
# Run all tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo test -- --nocapture

# Run specific test
cargo test test_query_needs

# Run integration tests only
cargo test --test '*'
```

## Key Differences from Shay

1. **Simpler structure** - No NATS, no complex job processing (yet)
2. **GraphQL-first** - Tests primarily use GraphQL client
3. **Trait naming** - Uses Base* convention (BaseWebScraper vs WebScraper)
4. **Fewer engines** - Only 3 domains (Organization, Member, Matching)

## File Structure

```
tests/
├── common/
│   ├── mod.rs              # Re-exports
│   ├── harness.rs          # TestHarness implementation
│   ├── graphql.rs          # GraphQL test client
│   └── fixtures.rs         # Test data helpers
├── content_hash_tests.rs   # Existing tests
├── organization_needs_tests.rs # Existing tests
└── integration/            # NEW: Domain integration tests
    ├── organization_tests.rs
    ├── matching_tests.rs
    └── member_tests.rs
```

## Next Steps

1. Create `tests/integration/` directory with domain-specific tests
2. Add more fixture helpers in `tests/common/fixtures.rs`
3. Add job queue integration when ready
4. Add performance benchmarks for event processing
