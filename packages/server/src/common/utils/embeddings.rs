use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::kernel::BaseEmbeddingService;

/// Embedding service using Voyage AI's voyage-3-large
pub struct EmbeddingService {
    client: Client,
    api_key: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    input: String,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    embeddings: Vec<Vec<f32>>,
}

impl EmbeddingService {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: "voyage-3-large".to_string(),
        }
    }
}

#[async_trait]
impl BaseEmbeddingService for EmbeddingService {
    async fn generate(&self, text: &str) -> Result<Vec<f32>> {
        let response = self
            .client
            .post("https://api.voyageai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&EmbeddingRequest {
                model: self.model.clone(),
                input: text.to_string(),
            })
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await?;
            anyhow::bail!("Voyage AI API error {}: {}", status, body);
        }

        let embedding_response: EmbeddingResponse = response.json().await?;

        let embedding = embedding_response
            .embeddings
            .first()
            .ok_or_else(|| anyhow::anyhow!("No embedding returned"))?
            .clone();

        Ok(embedding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_generate_embedding() {
        let api_key = std::env::var("VOYAGE_API_KEY").expect("VOYAGE_API_KEY not set");
        let service = EmbeddingService::new(api_key);

        let embedding = service
            .generate("I can help with food distribution and speak Spanish")
            .await
            .expect("Failed to generate embedding");

        assert_eq!(embedding.len(), 1024);
        println!("Generated embedding with {} dimensions", embedding.len());
    }
}
