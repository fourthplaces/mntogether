//! Restate Workflow Server
//!
//! This binary runs the Restate workflow HTTP server that handles
//! durable workflow executions.

use std::sync::Arc;

use anyhow::{Context, Result};
use restate_sdk::prelude::*;
use server_core::common::utils::{EmbeddingService, ExpoClient};
use server_core::domains::auth::workflows::{
    SendOtpWorkflow, SendOtpWorkflowImpl, VerifyOtpWorkflow, VerifyOtpWorkflowImpl,
};
use server_core::domains::auth::JwtService;
use server_core::domains::crawling::workflows::{CrawlWebsiteWorkflow, CrawlWebsiteWorkflowImpl};
use server_core::kernel::{
    create_extraction_service, OpenAIClient, ServerDeps, StreamHub, TwilioAdapter,
};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use twilio::{TwilioOptions, TwilioService};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,server_core=debug,restate_sdk=debug".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_line_number(true),
        )
        .init();

    tracing::info!("Starting Restate Workflow Server");

    // Load environment variables
    dotenvy::dotenv().ok();

    // Database setup
    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .context("Failed to connect to database")?;

    // Load configuration from environment
    let openai_api_key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY must be set")?;
    let tavily_api_key = std::env::var("TAVILY_API_KEY").context("TAVILY_API_KEY must be set")?;
    let firecrawl_api_key = std::env::var("FIRECRAWL_API_KEY").ok();
    let expo_access_token = std::env::var("EXPO_ACCESS_TOKEN").ok();
    let twilio_account_sid =
        std::env::var("TWILIO_ACCOUNT_SID").context("TWILIO_ACCOUNT_SID must be set")?;
    let twilio_auth_token =
        std::env::var("TWILIO_AUTH_TOKEN").context("TWILIO_AUTH_TOKEN must be set")?;
    let twilio_verify_service_sid = std::env::var("TWILIO_VERIFY_SERVICE_SID")
        .context("TWILIO_VERIFY_SERVICE_SID must be set")?;
    let jwt_secret = std::env::var("JWT_SECRET").context("JWT_SECRET must be set")?;
    let jwt_issuer = std::env::var("JWT_ISSUER").unwrap_or_else(|_| "mndigitalaid".to_string());
    let test_identifier_enabled = std::env::var("TEST_IDENTIFIER_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);
    let admin_identifiers = std::env::var("ADMIN_IDENTIFIERS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    let pii_scrubbing_enabled = std::env::var("PII_SCRUBBING_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);
    let pii_use_gpt_detection = std::env::var("PII_USE_GPT_DETECTION")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    // Create Twilio service
    let twilio_options = TwilioOptions {
        account_sid: twilio_account_sid,
        auth_token: twilio_auth_token,
        service_id: twilio_verify_service_sid,
    };
    let twilio = Arc::new(TwilioService::new(twilio_options));

    // Create OpenAI client
    let openai_client = Arc::new(OpenAIClient::new(openai_api_key.clone()));
    let embedding_api_key = openai_api_key.clone();

    // Create PII detector
    let pii_detector = server_core::kernel::pii::create_pii_detector(
        pii_scrubbing_enabled,
        pii_use_gpt_detection,
        Some(openai_api_key),
    );

    // Create ingestor with SSRF protection
    let ingestor: Arc<dyn extraction::Ingestor> = if let Some(key) = firecrawl_api_key {
        match extraction::FirecrawlIngestor::new(key) {
            Ok(firecrawl) => Arc::new(extraction::ValidatedIngestor::new(firecrawl)),
            Err(e) => {
                tracing::warn!(
                    "Failed to create Firecrawl ingestor: {}, falling back to HTTP",
                    e
                );
                Arc::new(extraction::ValidatedIngestor::new(
                    extraction::HttpIngestor::new(),
                ))
            }
        }
    } else {
        Arc::new(extraction::ValidatedIngestor::new(
            extraction::HttpIngestor::new(),
        ))
    };

    // Create web searcher
    let web_searcher: Arc<dyn extraction::WebSearcher> =
        Arc::new(extraction::TavilyWebSearcher::new(tavily_api_key));

    // Create extraction service
    let extraction_service = create_extraction_service(pool.clone())
        .await
        .context("Failed to create extraction service")?;

    // Create JWT service
    let jwt_service = Arc::new(JwtService::new(&jwt_secret, jwt_issuer));

    // Create StreamHub
    let stream_hub = StreamHub::new();

    // Build ServerDeps and wrap in Arc for sharing across workflows
    let server_deps = Arc::new(ServerDeps::new(
        pool.clone(),
        ingestor,
        openai_client,
        Arc::new(EmbeddingService::new(embedding_api_key)),
        Arc::new(ExpoClient::new(expo_access_token)),
        Arc::new(TwilioAdapter::new(twilio)),
        web_searcher,
        pii_detector,
        Some(extraction_service),
        jwt_service,
        stream_hub,
        test_identifier_enabled,
        admin_identifiers,
    ));

    // Get port from environment or use default
    let port = std::env::var("WORKFLOW_SERVER_PORT")
        .unwrap_or_else(|_| "9080".to_string())
        .parse::<u16>()
        .context("Invalid WORKFLOW_SERVER_PORT")?;

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Workflow server listening on {}", addr);

    // Build Restate endpoint with all domain workflows
    let endpoint = Endpoint::builder()
        // Auth domain workflows
        .bind(SendOtpWorkflowImpl::with_deps(server_deps.clone()).serve())
        .bind(VerifyOtpWorkflowImpl::with_deps(server_deps.clone()).serve())
        // Crawling domain workflows
        .bind(CrawlWebsiteWorkflowImpl::with_deps(server_deps.clone()).serve())
        .build();

    // Start HTTP server
    HttpServer::new(endpoint)
        .listen_and_serve(addr.parse()?)
        .await;

    Ok(())
}
