use anyhow::{Context, Result};
use server_core::config::Config;
use server_core::kernel::ai::OpenAIClient;
use server_core::kernel::ai_matching::AIMatchingService;
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<()> {
    // Load config
    let config = Config::from_env()?;

    // Connect to database
    let pool = PgPool::connect(&config.database_url)
        .await
        .context("Failed to connect to database")?;

    println!("✓ Connected to database");

    // Initialize OpenAI client
    let openai_client = OpenAIClient::new(config.openai_api_key.clone());
    let ai_matching = AIMatchingService::new(openai_client);

    println!("\n🚀 Starting embedding generation...\n");

    // Generate embeddings for organizations missing them
    let updated_count = ai_matching
        .update_missing_embeddings(&pool)
        .await
        .context("Failed to update embeddings")?;

    println!("\n✨ Embedding generation complete!");
    println!("   Updated: {} organizations", updated_count);

    Ok(())
}
