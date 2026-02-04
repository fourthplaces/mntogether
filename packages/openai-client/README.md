# openai-client

Pure OpenAI REST API client with no domain-specific logic.

## Features

- **Chat completions** - GPT-4o, GPT-4, etc.
- **Embeddings** - text-embedding-3-small/large
- **Structured outputs** - JSON schema validation
- **Function calling** - Tool use support

## Usage

```rust
use openai_client::{OpenAIClient, ChatRequest, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OpenAIClient::from_env()?;

    // Chat completion
    let response = client.chat_completion(
        ChatRequest::new("gpt-4o")
            .message(Message::user("What is Rust?"))
            .temperature(0.7)
    ).await?;

    println!("{}", response.content);

    // Embeddings
    let embedding = client.create_embedding(
        "Hello world",
        "text-embedding-3-small"
    ).await?;

    println!("Embedding dimensions: {}", embedding.len());

    Ok(())
}
```

## Configuration

Set your API key:

```bash
export OPENAI_API_KEY=sk-...
```

Or pass it directly:

```rust
let client = OpenAIClient::new("sk-...");
```

Custom base URL (for Azure, proxies):

```rust
let client = OpenAIClient::new("sk-...")
    .with_base_url("https://your-proxy.com/v1");
```

## Design Philosophy

This crate provides a **pure** OpenAI API client:
- ✅ Raw API access
- ✅ Clean types
- ✅ No domain logic
- ✅ Minimal dependencies

For higher-level extraction/RAG functionality, see the `extraction` crate which builds on top of this client.
