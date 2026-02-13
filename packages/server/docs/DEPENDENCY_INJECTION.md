# Dependency Injection Architecture

## Overview

The application uses two dependency containers that serve different purposes:

1. **ServerKernel** - Complete infrastructure layer
2. **ServerDeps** - Domain-specific dependencies

This document explains when and why to use each.

## ServerKernel (Infrastructure Layer)

**Location**: `src/kernel/server_kernel.rs`

**Purpose**: Complete server infrastructure with all low-level services

**Contains**:
- `db_pool: PgPool` - Database connection pool
- `web_scraper: Arc<dyn BaseWebScraper>` - Website scraping service
- `ai: Arc<dyn BaseAI>` - AI/LLM service
- `embedding_service: Arc<dyn BaseEmbeddingService>` - Vector embedding service
- `push_service: Arc<dyn BasePushNotificationService>` - Push notification service
- `search_service: Arc<dyn BaseSearchService>` - Web search service (Tavily)
- `pii_detector: Arc<dyn BasePiiDetector>` - PII detection service
- `bus: EventBus` - Event bus for domain event propagation
- `job_queue: Arc<dyn JobQueue>` - Background job queue

**Used By**:
- Test infrastructure (`TestHarness`)
- GraphQL schema construction
- Background job processing

**Usage Pattern**:
```rust
// In tests
let kernel = TestDependencies::new().into_kernel(db_pool);

// Access services
let scraper = kernel.web_scraper.clone();
```

## ServerDeps (Domain Dependencies)

**Location**: `src/domains/listings/effects/deps.rs`

**Purpose**: Dependencies required by domain effects (business logic layer)

**Contains**:
- `db_pool: PgPool` - Database connection pool
- `web_scraper: Arc<dyn BaseWebScraper>` - Website scraping service
- `ai: Arc<dyn BaseAI>` - AI/LLM service
- `embedding_service: Arc<dyn BaseEmbeddingService>` - Vector embedding service
- `push_service: Arc<dyn BasePushNotificationService>` - Push notification service
- `twilio: Arc<TwilioService>` - SMS service (domain-specific)
- `search_service: Arc<dyn BaseSearchService>` - Web search service
- `test_identifier_enabled: bool` - Auth config for testing
- `admin_identifiers: Vec<String>` - Auth config for admin access

**Used By**:
- Domain effects (ListingEffect, SyncEffect, etc.)
- Seesaw engines in production and tests
- Authorization checks (implements `HasAuthContext`)

**Usage Pattern**:
```rust
// In effects
async fn execute(
    &self,
    cmd: Command,
    ctx: EffectContext<ServerDeps>,
) -> Result<Event> {
    let deps = ctx.deps();
    let listings = Listing::find_pending(&deps.db_pool).await?;
    // ...
}
```

## Why Two Containers?

### Separation of Concerns

**ServerKernel**: Infrastructure Layer
- Owns event bus and job queue (framework-level concerns)
- Contains PII detection (cross-cutting infrastructure)
- Used for test harness setup
- Stable interface - rarely changes

**ServerDeps**: Domain Layer
- Contains only what domain effects need
- Includes domain-specific services (Twilio for SMS)
- Includes auth configuration
- Implements domain-specific traits (`HasAuthContext`)
- May evolve as domains add new dependencies

### Test Strategy

Tests create `ServerKernel` first, then derive `ServerDeps`:

```rust
// 1. Create infrastructure with mocks
let kernel = TestDependencies::new().into_kernel(db_pool);

// 2. Create domain deps from infrastructure
let server_deps = ServerDeps::new(
    kernel.db_pool.clone(),
    kernel.web_scraper.clone(),
    kernel.ai.clone(),
    kernel.embedding_service.clone(),
    kernel.push_service.clone(),
    test_twilio,
    kernel.search_service.clone(),
    true,  // test_identifier_enabled
    vec![], // admin_identifiers
);

// 3. Start engines with domain deps
let engines = start_domain_engines(server_deps, &kernel.bus);
```

This approach:
- Allows complete infrastructure mocking via `TestDependencies`
- Enables engine testing with real seesaw event propagation
- Keeps domain code independent of test infrastructure

## Production Usage

In production (`src/server/app.rs`):

```rust
// 1. Create real services
let web_scraper = Arc::new(FirecrawlClient::new(config.firecrawl_api_key));
let ai = Arc::new(OpenAIClient::new(config.openai_api_key));
// ... etc

// 2. Create ServerDeps directly (no ServerKernel)
let server_deps = ServerDeps::new(
    db_pool.clone(),
    web_scraper,
    ai,
    embedding_service,
    push_service,
    twilio,
    search_service,
    config.test_identifier_enabled,
    config.admin_identifiers,
);

// 3. Start engines
let bus = EventBus::new();
let engines = start_domain_engines(server_deps, &bus);
```

Production doesn't need `ServerKernel` because:
- Event bus is created directly
- No test mocking infrastructure needed
- Domain effects only need `ServerDeps`

## Adding New Dependencies

### Infrastructure Service (e.g., new API client)

Add to **both** containers if domain effects need it:

1. Add trait to `src/kernel/traits.rs`
2. Add field to `ServerKernel`
3. Add field to `ServerDeps`
4. Create mock in `TestDependencies`
5. Update production app.rs

### Domain-Specific Service (e.g., payment processor)

Add only to **ServerDeps**:

1. Add field to `ServerDeps::new()`
2. Update test harness to provide test version
3. Update production app.rs

## Common Pitfalls

### ❌ Using ServerKernel in Effects

```rust
// WRONG - Effects should not depend on ServerKernel
impl Effect<Cmd, ServerKernel> for MyEffect { }
```

### ✅ Using ServerDeps in Effects

```rust
// CORRECT - Effects depend on ServerDeps
impl Effect<Cmd, ServerDeps> for MyEffect { }
```

### ❌ Creating ServerDeps Without search_service

```rust
// WRONG - Missing search_service parameter
ServerDeps::new(
    db_pool,
    scraper,
    ai,
    embedding,
    push,
    twilio,
    true,  // Missing search_service here!
    vec![],
)
```

### ✅ Complete ServerDeps Construction

```rust
// CORRECT - All 9 parameters
ServerDeps::new(
    db_pool,
    scraper,
    ai,
    embedding,
    push,
    twilio,
    search_service,  // Don't forget this!
    true,
    vec![],
)
```

## Related Documentation

- `AUTHORIZATION.md` - How auth checks use `ServerDeps`
- `src/kernel/test_dependencies.rs` - Mock service implementations
- `tests/common/harness.rs` - Test infrastructure setup
- `src/server/app.rs` - Production dependency wiring
