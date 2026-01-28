# Refactoring Summary

## Changes Made

### 1. Kernel is Now Infrastructure-Only

**Before**: Kernel contained business logic (NeedExtractor with business-specific methods)
**After**: Kernel contains only infrastructure traits (BaseAI with generic methods)

The kernel now provides:
- `BaseAI` - Generic LLM completion (complete, complete_json)
- `BaseWebScraper` - Web scraping
- `BaseEmbeddingService` - Text embeddings
- `BasePushNotificationService` - Push notifications

### 2. Business Logic Moved to Domain Layer

**New file**: `src/domains/organization/effects/need_extraction.rs`

Contains domain functions:
```rust
// Domain function using infrastructure
pub async fn extract_needs(
    ai: &dyn BaseAI,
    organization_name: &str,
    website_content: &str,
    source_url: &str,
) -> Result<Vec<ExtractedNeed>>

pub async fn generate_outreach_copy(...) -> Result<String>
```

### 3. Effects Use Domain Functions

Effects now call domain functions instead of calling infrastructure directly:

```rust
// In AIEffect
let needs = need_extraction::extract_needs(
    ctx.deps().ai.as_ref(),
    &organization_name,
    &content,
    &source.source_url,
).await?;
```

### 4. TLDR Generation (TODO)

As requested, tldr should be generated with AI:
```rust
pub async fn generate_summary(
    ai: &dyn BaseAI,
    description: &str
) -> Result<String>
```

## Remaining Work

### 1. Complete MockAI Implementation

Update `src/kernel/test_dependencies.rs`:
```rust
pub struct MockAI {
    responses: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl BaseAI for MockAI {
    async fn complete(&self, _prompt: &str) -> Result<String> {
        // Return pre-configured response
    }

    async fn complete_json<T: DeserializeOwned>(&self, _prompt: &str) -> Result<T> {
        // Parse and return mock JSON
    }
}
```

### 2. Move Effect Logic to Individual Functions

As requested: "move effect handling code to individual functions instead of jamming everything in Effects"

For each effect, create separate function files:

#### Organization Domain
- `src/domains/organization/effects/scraping.rs` - Web scraping logic
- `src/domains/organization/effects/need_extraction.rs` - AI extraction logic (DONE)
- `src/domains/organization/effects/syncing.rs` - Sync logic
- `src/domains/organization/effects/need_operations.rs` - Need CRUD operations

#### Example Structure
```rust
// scraping.rs
pub async fn scrape_source(
    source_id: Uuid,
    web_scraper: &dyn BaseWebScraper,
    db_pool: &PgPool,
) -> Result<ScrapeResult> {
    // All scraping logic here
}

// Then in ScraperEffect:
impl Effect for ScraperEffect {
    async fn execute(&self, cmd: Command, ctx: Context) -> Result<Event> {
        match cmd {
            Command::ScrapeSource { source_id } => {
                let result = scraping::scrape_source(
                    source_id,
                    ctx.deps().web_scraper.as_ref(),
                    &ctx.deps().db_pool,
                ).await?;

                Ok(Event::SourceScraped { ... })
            }
        }
    }
}
```

### 3. Add generate_summary Function

```rust
// need_extraction.rs
pub async fn generate_summary(
    ai: &dyn BaseAI,
    description: &str,
) -> Result<String> {
    let prompt = format!(
        "Summarize this volunteer need in 1-2 sentences:\n\n{}",
        description
    );

    ai.complete(&prompt).await
}
```

### 4. Update All Effects

Apply the "individual functions" pattern to all effects:
- Organization effects (scraper, ai, sync, need)
- Member effects (registration)
- Matching effects (vector search)

## Benefits

1. **Separation of concerns**: Infrastructure (kernel) vs business logic (domain)
2. **Testability**: Can mock AI without business-specific methods
3. **Reusability**: Domain functions can be called from anywhere
4. **Clarity**: Effect handlers are thin orchestrators
5. **Single responsibility**: Each function file has one clear purpose

## File Structure After Refactoring

```
src/
├── kernel/                      # Infrastructure only
│   ├── ai.rs                   # OpenAIClient (BaseAI impl)
│   ├── traits.rs               # Base* traits
│   ├── server_kernel.rs        # Dependency container
│   └── test_dependencies.rs    # Mock implementations
├── domains/
│   └── organization/
│       ├── effects/
│       │   ├── scraping.rs     # Scraping functions
│       │   ├── need_extraction.rs  # AI extraction functions
│       │   ├── syncing.rs      # Sync functions
│       │   ├── need_operations.rs  # CRUD functions
│       │   ├── deps.rs         # ServerDeps (thin)
│       │   └── mod.rs          # Effect handlers (thin)
│       ├── commands/
│       ├── events/
│       └── machines/
```
