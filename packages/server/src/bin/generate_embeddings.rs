use anyhow::{Context, Result};
use ai_client::OpenAi;
use server_core::config::Config;
use server_core::domains::website::models::WebsiteAssessment;
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<()> {
    // Load config
    let config = Config::from_env()?;

    // Connect to database
    let pool = PgPool::connect(&config.database_url)
        .await
        .context("Failed to connect to database")?;

    println!("Connected to database");

    // Initialize OpenAI client
    let openai_client = OpenAi::new(config.openai_api_key.clone(), "gpt-4o");

    println!("\nStarting embedding generation...\n");

    // Generate embeddings for website assessments missing them
    println!("Generating website assessment embeddings...");
    let assessments = WebsiteAssessment::find_without_embeddings(&pool)
        .await
        .context("Failed to find assessments without embeddings")?;

    println!("Found {} assessments without embeddings", assessments.len());

    let mut assessment_updated = 0;
    for assessment in assessments {
        match openai_client
            .create_embedding(&assessment.assessment_markdown, "text-embedding-3-small")
            .await
        {
            Ok(embedding) => {
                if let Err(e) =
                    WebsiteAssessment::update_embedding(assessment.id, &embedding, &pool).await
                {
                    eprintln!(
                        "Failed to store embedding for assessment {}: {}",
                        assessment.id, e
                    );
                } else {
                    assessment_updated += 1;
                    println!(
                        "  Updated embedding for assessment {} (website: {})",
                        assessment.id, assessment.website_id
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "Failed to generate embedding for assessment {}: {}",
                    assessment.id, e
                );
            }
        }
    }

    println!("\nEmbedding generation complete!");
    println!("  Assessments: {} updated", assessment_updated);

    Ok(())
}
