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
    pub voyage_api_key: String,
    pub firecrawl_api_key: String,
    pub tavily_api_key: Option<String>,
    pub expo_access_token: Option<String>,
    pub twilio_account_sid: String,
    pub twilio_auth_token: String,
    pub twilio_verify_service_sid: String,
    pub jwt_secret: String,
    pub jwt_issuer: String,
    pub allowed_origins: Vec<String>,
    pub test_identifier_enabled: bool,
    pub admin_identifiers: Vec<String>,
    pub pii_scrubbing_enabled: bool,
    pub pii_use_gpt_detection: bool,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        // Load .env file if present (development)
        let _ = dotenv();

        Ok(Self {
            database_url: env::var("DATABASE_URL").context("DATABASE_URL must be set")?,
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .context("PORT must be a valid number")?,
            openai_api_key: env::var("OPENAI_API_KEY")
                .context("OPENAI_API_KEY must be set")?,
            voyage_api_key: env::var("VOYAGE_API_KEY").context("VOYAGE_API_KEY must be set")?,
            firecrawl_api_key: env::var("FIRECRAWL_API_KEY")
                .context("FIRECRAWL_API_KEY must be set")?,
            tavily_api_key: env::var("TAVILY_API_KEY").ok(),
            expo_access_token: env::var("EXPO_ACCESS_TOKEN").ok(),
            twilio_account_sid: env::var("TWILIO_ACCOUNT_SID")
                .context("TWILIO_ACCOUNT_SID must be set")?,
            twilio_auth_token: env::var("TWILIO_AUTH_TOKEN")
                .context("TWILIO_AUTH_TOKEN must be set")?,
            twilio_verify_service_sid: env::var("TWILIO_VERIFY_SERVICE_SID")
                .context("TWILIO_VERIFY_SERVICE_SID must be set")?,
            jwt_secret: env::var("JWT_SECRET").context("JWT_SECRET must be set")?,
            jwt_issuer: env::var("JWT_ISSUER").unwrap_or_else(|_| "mndigitalaid".to_string()),
            allowed_origins: env::var("ALLOWED_ORIGINS")
                .unwrap_or_else(|_| {
                    if cfg!(debug_assertions) {
                        // Development: Allow localhost and Expo
                        "http://localhost:3000,http://localhost:19006,http://localhost:8081"
                            .to_string()
                    } else {
                        // Production: Must be explicitly set
                        "".to_string()
                    }
                })
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            test_identifier_enabled: env::var("TEST_IDENTIFIER_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            admin_identifiers: env::var("ADMIN_IDENTIFIERS")
                .unwrap_or_else(|_| "".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            pii_scrubbing_enabled: env::var("PII_SCRUBBING_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            pii_use_gpt_detection: env::var("PII_USE_GPT_DETECTION")
                .unwrap_or_else(|_| "true".to_string()) // AI detection ON by default
                .parse()
                .unwrap_or(true),
        })
    }
}
