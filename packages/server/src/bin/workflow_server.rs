//! Restate Workflow Server
//!
//! This binary runs the Restate workflow HTTP server that handles
//! durable workflow executions.

use anyhow::{Context, Result};
use restate_sdk::prelude::*;
use server_core::domains::crawling::workflows::CrawlWebsiteWorkflowImpl;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

    // Get port from environment or use default
    let port = std::env::var("WORKFLOW_SERVER_PORT")
        .unwrap_or_else(|_| "9080".to_string())
        .parse::<u16>()
        .context("Invalid WORKFLOW_SERVER_PORT")?;

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Workflow server listening on {}", addr);

    // Build Restate endpoint with all domain workflows
    let endpoint = Endpoint::builder()
        // Crawling domain workflows
        .bind(CrawlWebsiteWorkflowImpl.serve())
        // TODO: Add other domain workflows as they're migrated
        .build();

    // Start HTTP server
    HttpServer::new(endpoint)
        .listen_and_serve(addr.parse()?)
        .await;

    Ok(())
}
