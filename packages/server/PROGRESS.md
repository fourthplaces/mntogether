# Refactoring Progress

## ✅ Completed

### 1. Kernel is Infrastructure-Only ✅
- Created `BaseAI` trait with generic methods (`complete`, `complete_json`)
- Created `OpenAIClient` implementing `BaseAI`
- Removed business logic (`NeedExtractor`) from kernel
- Updated `ServerDeps` to use `BaseAI` instead of `BaseNeedExtractor`
- Updated `ServerKernel` to use `BaseAI`

### 2. Domain Functions for Business Logic ✅
- Created `need_extraction.rs` with:
  - `extract_needs(ai, ...)` - AI-powered need extraction
  - `generate_summary(ai, description)` - TLDR generation
  - `generate_outreach_copy(ai, ...)` - Email generation
- Created `scraping.rs` with:
  - `scrape_source(source_id, web_scraper, db_pool)` - Web scraping logic

### 3. Thin Effect Orchestrators ✅
- Updated `AIEffect` to delegate to `need_extraction::extract_needs()`
- Updated `ScraperEffect` to delegate to `scraping::scrape_source()`
- Both effects are now thin orchestrators (< 20 lines)

### 4. Mock Infrastructure ✅
- Implemented `MockAI` with:
  - `with_response()` - Queue text responses
  - `with_json_response()` - Queue JSON responses
  - Implements `complete()` and `complete_json()`
- Updated `TestDependencies` with `mock_ai()` method

### 5. Build System Updated ✅
- Updated `build_app()` to use `OpenAIClient` instead of `NeedExtractor`
- Updated imports to use infrastructure traits

## ⚠️ Remaining Work

### 1. Update TLDR Generation in NeedEffect
**File**: `src/domains/organization/effects/need.rs`
**Line**: 42-45

**Current** (hardcoded):
```rust
let tldr = if description.len() > 100 {
    format!("{}...", &description[..97])
} else {
    description.clone()
};
```

**Should be**:
```rust
let tldr = need_extraction::generate_summary(
    ctx.deps().ai.as_ref(),
    &description
).await?;
```

### 2. Extract Need Operations Logic (Optional)
Create `need_operations.rs` with functions:
- `create_need(...)` - Need creation logic
- `update_need_status(...)` - Status update logic
- `update_and_approve_need(...)` - Edit + approve logic

Then update `NeedEffect` to delegate to these functions.

### 3. Remove Old Files (Cleanup)
- Delete `src/domains/organization/effects/utils/need_extractor.rs` (replaced by `need_extraction.rs`)
- Update `utils/mod.rs` to remove `need_extractor`

## File Structure (After Refactoring)

```
src/
├── kernel/                          # INFRASTRUCTURE ONLY
│   ├── ai.rs                       # OpenAIClient (BaseAI impl)
│   ├── traits.rs                   # BaseAI, BaseWebScraper, etc.
│   ├── server_kernel.rs            # Dependency container
│   └── test_dependencies.rs        # MockAI, MockWebScraper, etc.
├── domains/organization/
│   ├── effects/
│   │   ├── ai.rs                   # Thin orchestrator → need_extraction
│   │   ├── scraper.rs              # Thin orchestrator → scraping
│   │   ├── sync.rs                 # Thin orchestrator → sync_utils
│   │   ├── need.rs                 # TODO: Make thin → need_operations
│   │   ├── need_extraction.rs      # BUSINESS LOGIC (AI extraction)
│   │   ├── scraping.rs             # BUSINESS LOGIC (web scraping)
│   │   └── utils/
│   │       ├── sync_utils.rs       # BUSINESS LOGIC (sync)
│   │       └── firecrawl.rs        # Infrastructure impl
│   ├── commands/
│   ├── events/
│   └── models/
```

## Key Architectural Principles

1. **Kernel = Infrastructure** - Generic capabilities, no business logic
2. **Domain Functions = Business Logic** - Use infrastructure to solve domain problems
3. **Effects = Thin Orchestrators** - Match commands to domain functions
4. **Testability** - Mock infrastructure, test business logic directly

## Example Usage

### In Production
```rust
let ai = OpenAIClient::new(api_key);
let needs = need_extraction::extract_needs(&ai, org, content, url).await?;
```

### In Tests
```rust
let mock_ai = MockAI::new()
    .with_json_response(&vec![
        ExtractedNeed { title: "Test", ... }
    ]);
let needs = need_extraction::extract_needs(&mock_ai, org, content, url).await?;
```

## Benefits Achieved

1. ✅ Clear separation between infrastructure and business logic
2. ✅ Generic AI capability usable across all domains
3. ✅ Easy to test business logic with mocks
4. ✅ Thin effects are easy to understand and maintain
5. ✅ Domain functions are reusable (can be called from edges, other effects, etc.)

## Next Steps

1. Update TLDR generation to use `generate_summary()`
2. (Optional) Extract need operations to separate function file
3. (Optional) Clean up old need_extractor files
4. Run tests to verify everything works
