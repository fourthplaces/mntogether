use anyhow::{Context, Result};
use dotenvy::dotenv;
use std::env;

/// Application configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub port: u16,
    pub openai_api_key: String,
    pub firecrawl_api_key: String,
    pub tavily_api_key: Option<String>,
    pub clerk_secret_key: Option<String>,
    pub expo_access_token: Option<String>,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        // Load .env file if present (development)
        let _ = dotenv();

        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .context("DATABASE_URL must be set")?,
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .context("PORT must be a valid number")?,
            openai_api_key: env::var("OPENAI_API_KEY")
                .context("OPENAI_API_KEY must be set")?,
            firecrawl_api_key: env::var("FIRECRAWL_API_KEY")
                .context("FIRECRAWL_API_KEY must be set")?,
            tavily_api_key: env::var("TAVILY_API_KEY").ok(),
            clerk_secret_key: env::var("CLERK_SECRET_KEY").ok(),
            expo_access_token: env::var("EXPO_ACCESS_TOKEN").ok(),
        })
    }
}
