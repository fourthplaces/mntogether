//! Basic OpenAI client usage example

use openai_client::{ChatRequest, Message, OpenAIClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize from environment
    let client = OpenAIClient::from_env()?;

    // Simple chat completion
    println!("=== Chat Completion ===");
    let response = client
        .chat_completion(
            ChatRequest::new("gpt-4o")
                .message(Message::system("You are a helpful assistant."))
                .message(Message::user("What is Rust in one sentence?"))
                .temperature(0.7)
                .max_tokens(100),
        )
        .await?;

    println!("Response: {}", response.content);

    // Embeddings
    println!("\n=== Embeddings ===");
    let embedding = client
        .create_embedding("Hello, world!", "text-embedding-3-small")
        .await?;

    println!("Embedding dimensions: {}", embedding.len());
    println!("First 5 values: {:?}", &embedding[..5]);

    // Batch embeddings
    println!("\n=== Batch Embeddings ===");
    let texts = &["First text", "Second text", "Third text"];
    let embeddings = client
        .create_embeddings_batch(texts, "text-embedding-3-small")
        .await?;

    println!("Created {} embeddings", embeddings.len());

    Ok(())
}
