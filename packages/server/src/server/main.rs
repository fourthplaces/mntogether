// Main entry point for server

use anyhow::{Context, Result};
use server_core::{kernel::scheduled_tasks, server::build_app, Config};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "info,server_core=debug,sqlx=warn,seesaw=debug,tower_http=debug".into()
            }),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_line_number(true),
        )
        .init();

    tracing::info!("Starting Emergency Resource Aggregator Server");

    // Load configuration
    let config = Config::from_env().context("Failed to load configuration")?;
    tracing::info!("Configuration loaded");

    // Connect to database
    tracing::info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .context("Failed to connect to database")?;
    tracing::info!("Database connected");

    // Run migrations
    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("Failed to run migrations")?;
    tracing::info!("Migrations complete");

    // Build application
    let (app, handle) = build_app(
        pool.clone(),
        config.firecrawl_api_key,
        config.openai_api_key,
        config.expo_access_token,
        config.twilio_account_sid,
        config.twilio_auth_token,
        config.twilio_verify_service_sid,
    );

    // Start scheduled tasks (periodic scraping)
    let bus = handle.bus().clone();
    let _scheduler = scheduled_tasks::start_scheduler(pool.clone(), bus)
        .await
        .context("Failed to start scheduler")?;

    // Start server
    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Starting server on {}", addr);
    tracing::info!(
        "GraphQL playground: http://localhost:{}/graphql",
        config.port
    );
    tracing::info!("Health check: http://localhost:{}/health", config.port);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .context("Failed to bind to address")?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .context("Server error")?;

    Ok(())
}
