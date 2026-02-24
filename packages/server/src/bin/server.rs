//! Restate Workflow Server — Root Editorial CMS

use std::sync::Arc;

use anyhow::{Context, Result};
use restate_sdk::prelude::*;
use server_core::domains::auth::restate::{AuthService, AuthServiceImpl};
use server_core::domains::auth::JwtService;
use server_core::domains::heat_map::restate::{HeatMapService, HeatMapServiceImpl};
use server_core::domains::jobs::restate::{JobsService, JobsServiceImpl};
use server_core::domains::member::restate::{
    MemberObject, MemberObjectImpl, MembersService, MembersServiceImpl, RegisterMemberWorkflow,
    RegisterMemberWorkflowImpl,
};
use server_core::domains::notes::restate::{NotesService, NotesServiceImpl};
use server_core::domains::organization::restate::{
    OrganizationsService, OrganizationsServiceImpl,
};
use server_core::domains::posts::restate::{
    PostObject, PostObjectImpl, PostsService, PostsServiceImpl,
};
use server_core::domains::tag::restate::{TagsService, TagsServiceImpl};
use server_core::kernel::{ServerDeps};
use server_core::common::utils::EmbeddingService;
use server_core::kernel::{OpenAi, Claude, TwilioAdapter, StreamHub};
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

    tracing::info!("Starting Root Editorial Server");

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

    // Create AI clients
    let openai_client = Arc::new(OpenAi::new(openai_api_key.clone(), "gpt-4o"));
    let claude_client = std::env::var("ANTHROPIC_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
        .map(|key| Arc::new(Claude::new(key, "claude-sonnet-4-5-20250929")));
    let embedding_api_key = openai_api_key.clone();

    // Create PII detector
    let pii_detector = server_core::kernel::pii::create_pii_detector(
        pii_scrubbing_enabled,
        pii_use_gpt_detection,
        Some(openai_client.clone()),
    );

    // Create JWT service
    let jwt_service = Arc::new(JwtService::new(&jwt_secret, jwt_issuer));

    // Create StreamHub
    let stream_hub = StreamHub::new();

    // Build ServerDeps
    let server_deps = Arc::new(ServerDeps::new(
        pool.clone(),
        openai_client,
        claude_client,
        Arc::new(EmbeddingService::new(embedding_api_key)),
        Arc::new(TwilioAdapter::new(twilio)),
        pii_detector,
        jwt_service,
        stream_hub,
        test_identifier_enabled,
        admin_identifiers,
    ));

    // Get port from environment or use default
    let port = std::env::var("SERVER_PORT")
        .unwrap_or_else(|_| "9080".to_string())
        .parse::<u16>()
        .context("Invalid SERVER_PORT")?;

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Server listening on {}", addr);

    // Build Restate endpoint with all domain services
    let mut builder = Endpoint::builder();

    // Configure Restate request identity verification
    if let Ok(identity_key) = std::env::var("RESTATE_IDENTITY_KEY") {
        tracing::info!("Restate identity key configured");
        builder = builder
            .identity_key(&identity_key)
            .context("Invalid Restate identity key")?;
    }

    let endpoint = builder
        // Auth
        .bind(AuthServiceImpl::with_deps(server_deps.clone()).serve())
        // Heat map
        .bind(HeatMapServiceImpl::with_deps(server_deps.clone()).serve())
        // Jobs
        .bind(JobsServiceImpl::with_deps(server_deps.clone()).serve())
        // Notes
        .bind(NotesServiceImpl::with_deps(server_deps.clone()).serve())
        // Organizations
        .bind(OrganizationsServiceImpl::with_deps(server_deps.clone()).serve())
        // Members
        .bind(MemberObjectImpl::with_deps(server_deps.clone()).serve())
        .bind(MembersServiceImpl::with_deps(server_deps.clone()).serve())
        .bind(RegisterMemberWorkflowImpl::with_deps(server_deps.clone()).serve())
        // Posts
        .bind(PostObjectImpl::with_deps(server_deps.clone()).serve())
        .bind(PostsServiceImpl::with_deps(server_deps.clone()).serve())
        // Tags
        .bind(TagsServiceImpl::with_deps(server_deps.clone()).serve())
        .build();

    // Auto-register with Restate runtime if RESTATE_ADMIN_URL is set
    if let Ok(admin_url) = std::env::var("RESTATE_ADMIN_URL") {
        let self_url = std::env::var("RESTATE_SELF_URL")
            .unwrap_or_else(|_| format!("http://localhost:{}", port));
        let auth_token = std::env::var("RESTATE_AUTH_TOKEN").ok();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            tracing::info!(
                admin_url = %admin_url,
                self_url = %self_url,
                "Auto-registering with Restate"
            );
            let client = reqwest::Client::new();
            let mut request =
                client
                    .post(format!("{}/deployments", admin_url))
                    .json(&serde_json::json!({
                        "uri": self_url,
                        "force": true
                    }));
            if let Some(token) = &auth_token {
                request = request.bearer_auth(token);
            }
            match request.send().await {
                Ok(resp) if resp.status().is_success() => {
                    tracing::info!("Restate registration successful");
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    tracing::warn!(
                        status = %status,
                        body = %body,
                        "Restate registration failed"
                    );
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to connect to Restate admin");
                }
            }
        });
    }

    // Start HTTP server
    HttpServer::new(endpoint)
        .listen_and_serve(addr.parse()?)
        .await;

    Ok(())
}
