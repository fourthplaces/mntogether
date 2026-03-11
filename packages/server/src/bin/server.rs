//! Root Editorial CMS — Axum HTTP API Server

use std::sync::Arc;

use anyhow::{Context, Result};
use server_core::domains::auth::JwtService;
use server_core::kernel::ServerDeps;
use server_core::kernel::{TwilioAdapter, StreamHub};
use server_core::kernel::sse::SseState;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use twilio::{TwilioOptions, TwilioService};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,server_core=debug".into()),
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
    let twilio_account_sid =
        std::env::var("TWILIO_ACCOUNT_SID").context("TWILIO_ACCOUNT_SID must be set")?;
    let twilio_auth_token =
        std::env::var("TWILIO_AUTH_TOKEN").context("TWILIO_AUTH_TOKEN must be set")?;
    let twilio_verify_service_sid = std::env::var("TWILIO_VERIFY_SERVICE_SID")
        .context("TWILIO_VERIFY_SERVICE_SID must be set")?;
    let jwt_secret = std::env::var("JWT_SECRET").context("JWT_SECRET must be set")?;
    let jwt_issuer = std::env::var("JWT_ISSUER").unwrap_or_else(|_| "rooteditorial".to_string());
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
    // Create Twilio service
    let twilio_options = TwilioOptions {
        account_sid: twilio_account_sid,
        auth_token: twilio_auth_token,
        service_id: twilio_verify_service_sid,
    };
    let twilio = Arc::new(TwilioService::new(twilio_options));

    // Create PII detector
    let pii_detector = server_core::kernel::pii::create_pii_detector(pii_scrubbing_enabled);

    // Create JWT service
    let jwt_service = Arc::new(JwtService::new(&jwt_secret, jwt_issuer));

    // Create StreamHub
    let stream_hub = StreamHub::new();

    // Create S3-compatible storage adapter (optional — only if S3_BUCKET is set)
    let storage: Option<Arc<dyn server_core::kernel::BaseStorageService>> =
        if let Ok(bucket) = std::env::var("S3_BUCKET") {
            let endpoint = std::env::var("S3_ENDPOINT").ok();
            let presign_endpoint = std::env::var("S3_PRESIGN_ENDPOINT").ok();
            let region = std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".into());
            let public_url = std::env::var("S3_PUBLIC_URL").unwrap_or_else(|_| {
                format!(
                    "{}/{}",
                    endpoint.as_deref().unwrap_or("http://localhost:9000"),
                    bucket
                )
            });
            let adapter = server_core::kernel::storage::S3StorageAdapter::new(
                endpoint.as_deref(),
                presign_endpoint.as_deref(),
                &region,
                &bucket,
                &public_url,
            )
            .await;
            tracing::info!(bucket = %bucket, "S3 storage adapter initialized");
            Some(Arc::new(adapter))
        } else {
            tracing::info!("No S3_BUCKET set — media uploads disabled");
            None
        };

    // Build ServerDeps
    let server_deps = Arc::new(ServerDeps::new(
        pool.clone(),
        Arc::new(TwilioAdapter::new(twilio)),
        pii_detector,
        storage,
        jwt_service.clone(),
        stream_hub.clone(),
        test_identifier_enabled,
        admin_identifiers,
    ));

    // Get port from environment or use default
    let port = std::env::var("SERVER_PORT")
        .unwrap_or_else(|_| "9080".to_string())
        .parse::<u16>()
        .context("Invalid SERVER_PORT")?;

    // Build Axum router with API routes + SSE streams
    let app_state = server_core::api::state::AppState {
        deps: server_deps.clone(),
    };
    let sse_state = SseState {
        stream_hub: stream_hub,
        jwt_service: jwt_service,
    };

    let app = server_core::api::router(app_state)
        .merge(server_core::kernel::sse::router(sse_state));

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .context("Failed to bind listener")?;

    axum::serve(listener, app).await.context("Server error")?;

    Ok(())
}
