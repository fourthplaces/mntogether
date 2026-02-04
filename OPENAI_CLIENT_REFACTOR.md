# OpenAI Client Refactoring

## Summary

Extracted the OpenAI API client code from the `extraction` library into a pure, reusable `openai-client` package with no domain-specific logic.

## New Package Structure

```
packages/
├── openai-client/           ← NEW: Pure OpenAI REST API client
│   ├── src/
│   │   ├── lib.rs          # Main client implementation
│   │   ├── types.rs        # Request/response types
│   │   └── error.rs        # Error types
│   ├── examples/
│   │   ├── basic.rs        # Chat & embeddings example
│   │   └── structured.rs   # Structured output example
│   ├── Cargo.toml
│   └── README.md
│
├── extraction/              ← UPDATED: Uses openai-client
│   └── src/ai/openai.rs    # Implements AI trait using pure client
│
└── server/                  ← UPDATED: Can use openai-client directly
    └── Cargo.toml          # Added openai-client dependency
```

## What's in `openai-client`

### Pure API Methods
- `chat_completion()` - Chat with GPT models
- `create_embedding()` - Generate embeddings
- `create_embeddings_batch()` - Batch embeddings
- `structured_output()` - JSON schema responses
- `function_calling()` - Tool use support

### Clean Types
- `ChatRequest`, `ChatResponse`
- `Message` (system/user/assistant)
- `StructuredRequest` with JSON schema
- `FunctionRequest`, `FunctionResponse`
- `OpenAIError` for error handling

### Utilities
- `truncate_to_char_boundary()` - UTF-8 safe truncation
- `strip_code_blocks()` - Clean JSON from markdown
- Model detection for token params

## What Stayed in `extraction`

The `extraction` library's `ai/openai.rs` now:
- Uses `openai-client` for HTTP calls
- Implements extraction-specific logic:
  - `summarize()` - Page summarization with recall signals
  - `classify_query()` - Strategy detection
  - `recall_and_partition()` - Page grouping
  - `extract()` - Evidence-grounded extraction
- Parses extraction-specific response types

## Benefits

✅ **Clean separation** - OpenAI client has no extraction concepts
✅ **Reusable** - Server can use client directly for any AI task
✅ **Testable** - Pure client is easy to mock
✅ **Maintainable** - Changes to extraction don't affect client
✅ **Clear dependencies**: `server` → `openai-client` → OpenAI API
                           `server` → `extraction` → `openai-client` → OpenAI API

## Usage Examples

### Direct Client Usage

```rust
use openai_client::{OpenAIClient, ChatRequest, Message};

let client = OpenAIClient::from_env()?;

// Simple chat
let response = client.chat_completion(
    ChatRequest::new("gpt-4o")
        .message(Message::user("Hello!"))
).await?;

// Embeddings
let embedding = client.create_embedding(
    "text to embed",
    "text-embedding-3-small"
).await?;
```

### Extraction Library Usage

```rust
use extraction::ai::OpenAI;
use extraction::{Index, MemoryStore};

let ai = OpenAI::from_env()?;
let store = MemoryStore::new();
let index = Index::new(store, ai);

// Uses client internally for extraction logic
let results = index.extract("volunteer opportunities", None).await?;
```

## Testing

Both packages compile successfully:
```bash
✅ cargo check -p openai-client
✅ cargo check -p extraction --features openai
```

## Files Changed

**New files:**
- `packages/openai-client/Cargo.toml`
- `packages/openai-client/src/lib.rs`
- `packages/openai-client/src/types.rs`
- `packages/openai-client/src/error.rs`
- `packages/openai-client/README.md`
- `packages/openai-client/examples/basic.rs`
- `packages/openai-client/examples/structured.rs`

**Modified files:**
- `packages/extraction/Cargo.toml` (added openai-client dependency)
- `packages/extraction/src/ai/openai.rs` (refactored to use client)
- `packages/server/Cargo.toml` (added openai-client dependency)

## Next Steps

Potential improvements:
1. Add rate limiting to client
2. Add retry logic with exponential backoff
3. Support streaming responses
4. Add Azure OpenAI support
5. Add batch API support
6. Add image generation (DALL-E)
