# Migration from OpenAI to Claude + Voyage AI

This document summarizes the changes made to migrate from OpenAI to Anthropic Claude for AI completions and Voyage AI for embeddings.

## Summary

- **AI Completions**: OpenAI GPT-4o → Anthropic Claude 3.5 Sonnet
- **Embeddings**: OpenAI text-embedding-3-small (1536 dims) → Voyage AI voyage-3-large (1024 dims)

## Changes Made

### 1. Database Migration
**File**: `packages/server/migrations/000022_change_embedding_dimensions.sql`
- Dropped existing vector columns and indexes
- Recreated with 1024 dimensions (down from 1536)
- Updated comments to reflect Voyage AI

### 2. Configuration Updates
**File**: `packages/server/src/config.rs`
- Replaced `openai_api_key: String` with:
  - `anthropic_api_key: String`
  - `voyage_api_key: String`

**File**: `packages/server/.env.example`
- Updated environment variable examples:
  ```bash
  # Before
  OPENAI_API_KEY=sk-...

  # After
  ANTHROPIC_API_KEY=sk-ant-...
  VOYAGE_API_KEY=pa-...
  ```

### 3. AI Client Changes
**File**: `packages/server/src/kernel/ai.rs`
- Renamed `OpenAIClient` → `ClaudeClient`
- Updated to use `anthropic::ClientBuilder::new(api_key).build()`
- Changed model from `gpt-4o` to `claude-3-5-sonnet-latest`
- Added required `max_tokens(4096)` parameter
- Updated test to use `ANTHROPIC_API_KEY`

**File**: `packages/server/src/kernel/mod.rs`
- Updated export from `OpenAIClient` to `ClaudeClient`

### 4. Embeddings Service Changes
**File**: `packages/server/src/common/utils/embeddings.rs`
- Changed model from `text-embedding-3-small` to `voyage-3-large`
- Updated API endpoint to `https://api.voyageai.com/v1/embeddings`
- Updated response parsing (OpenAI returns `data[].embedding`, Voyage returns `embeddings[]`)
- Changed expected dimensions from 1536 to 1024
- Updated test to use `VOYAGE_API_KEY`

### 5. Application Initialization
**File**: `packages/server/src/server/app.rs`
- Updated import from `OpenAIClient` to `ClaudeClient`
- Changed function signature of `build_app()` to accept `anthropic_api_key` and `voyage_api_key`
- Updated client instantiations:
  ```rust
  // Before
  Arc::new(OpenAIClient::new(openai_api_key.clone()))
  Arc::new(EmbeddingService::new(openai_api_key))

  // After
  Arc::new(ClaudeClient::new(anthropic_api_key))
  Arc::new(EmbeddingService::new(voyage_api_key))
  ```

**File**: `packages/server/src/server/main.rs`
- Updated `build_app()` call to pass new API keys from config

### 6. Domain Layer Updates
**File**: `packages/server/src/domains/organization/effects/need_extraction.rs`
- Updated test imports from `OpenAIClient` to `ClaudeClient`
- Changed test to use `ANTHROPIC_API_KEY`

**File**: `packages/server/src/bin/seed_organizations.rs`
- Changed variable name from `openai_api_key` to `anthropic_api_key`
- Updated `extract_tags_with_ai()` to use Claude API:
  - Endpoint: `https://api.anthropic.com/v1/messages`
  - Headers: `x-api-key` and `anthropic-version`
  - Model: `claude-3-5-haiku-latest`
  - Request format: Anthropic Messages API format
  - Response parsing: `content[0].text` instead of `choices[0].message.content`

### 7. Dev CLI Updates
**File**: `packages/dev-cli/src/main.rs`
- Updated environment variable wizard to include:
  - `ANTHROPIC_API_KEY` (required)
  - `VOYAGE_API_KEY` (required)
- Updated API key checker to validate new keys
- Updated example .env output
- Changed example in flyctl secret setter

## Environment Variables

### Required New Variables
```bash
ANTHROPIC_API_KEY=sk-ant-...  # Get from https://console.anthropic.com
VOYAGE_API_KEY=pa-...         # Get from https://www.voyageai.com
```

### Removed Variables
```bash
OPENAI_API_KEY  # No longer needed
```

## API Costs Comparison

### AI Completions
- **OpenAI GPT-4o**: $2.50 per 1M input tokens, $10.00 per 1M output tokens
- **Claude 3.5 Sonnet**: $3.00 per 1M input tokens, $15.00 per 1M output tokens
- _Claude is ~20-50% more expensive but offers superior reasoning and code quality_

### Embeddings
- **OpenAI text-embedding-3-small**: $0.02 per 1M tokens (1536 dimensions)
- **Voyage AI voyage-3-large**: $0.12 per 1M tokens (1024 dimensions)
- _Voyage is 6x more expensive but optimized for semantic search (first 200M tokens free)_

## Models Used

- **AI Completions**: `claude-3-5-sonnet-latest` via rig-core 0.7
- **Embeddings**: `voyage-3-large` (1024 dimensions)

## Next Steps

1. **Set Environment Variables**: Add `ANTHROPIC_API_KEY` and `VOYAGE_API_KEY` to your `.env` file
2. **Run Migration**: Execute the new database migration to update vector dimensions
   ```bash
   cargo run --bin server
   # or
   ./dev-cli  # then select "Run database migrations"
   ```
3. **Restart Services**: All embeddings need to be regenerated with the new model
4. **Test**: Run tests to ensure everything works:
   ```bash
   cargo test -- --ignored  # Runs integration tests with API keys
   ```

## Rollback Plan

If you need to rollback:
1. Revert all Rust code changes
2. Create a migration to change vector dimensions back to 1536
3. Regenerate all embeddings with OpenAI
4. Update environment variables back to `OPENAI_API_KEY`

## Additional Notes

- The vector dimension change (1536 → 1024) means **all existing embeddings will be lost** and need to be regenerated
- Voyage AI offers 200M free tokens for new accounts
- Claude 3.5 Sonnet has better reasoning capabilities than GPT-4o for complex tasks
- The API response formats are different between providers, so any direct API calls need updating
