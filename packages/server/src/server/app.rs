//! Application setup and server configuration.

use std::sync::Arc;

use axum::{
    extract::{Extension, Request},
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method,
    },
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Router,
};
use seesaw_core::Engine;
use sqlx::PgPool;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use twilio::{TwilioOptions, TwilioService};

use crate::domains::auth::JwtService;
use crate::domains::crawling::effects::register_crawling_jobs;
use crate::kernel::jobs::{JobRegistry, JobRunner, PostgresJobQueue};
use crate::kernel::{create_extraction_service, OpenAIClient, ServerDeps, TwilioAdapter};
use crate::server::graphql::context::AppEngine;
use crate::server::graphql::{create_schema, GraphQLContext};
use crate::server::middleware::{extract_client_ip, jwt_auth_middleware, AuthUser};
use crate::server::routes::{
    graphql_batch_handler, graphql_handler, graphql_playground, health_handler,
};
// Note: web-app static file serving removed - web-next runs as separate service

// Import effect builder functions from each domain
use crate::domains::auth::effects::auth_effect;
use crate::domains::chatrooms::effects::chat_effect;
use crate::domains::crawling::effects::mark_no_listings_effect;
use crate::domains::member::effects::member_effect;
use crate::domains::posts::effects::post_composite_effect;
use crate::domains::providers::effects::provider_effect;
use crate::domains::website::effects::website_effect;
use crate::domains::website_approval::effects::website_approval_effect;

/// Shared application state
#[derive(Clone)]
pub struct AxumAppState {
    pub db_pool: PgPool,
    pub engine: Arc<AppEngine>,
    pub server_deps: Arc<ServerDeps>,
    pub twilio: Arc<TwilioService>,
    pub jwt_service: Arc<JwtService>,
    pub openai_client: Arc<OpenAIClient>,
}

/// Middleware to create GraphQLContext per-request
async fn create_graphql_context(
    Extension(state): Extension<AxumAppState>,
    mut request: Request,
    next: Next,
) -> Response {
    // Extract auth user from request extensions (populated by jwt_auth_middleware)
    let auth_user = request.extensions().get::<AuthUser>().cloned();

    // Create GraphQL context with shared state + per-request auth
    let context = GraphQLContext::new(
        state.db_pool.clone(),
        state.engine.clone(),
        state.server_deps.clone(),
        auth_user,
        state.twilio.clone(),
        state.jwt_service.clone(),
        state.openai_client.clone(),
    );

    // Add context to request extensions
    request.extensions_mut().insert(context);

    next.run(request).await
}

/// Build the seesaw engine with all domain effects
///
/// In seesaw 0.7.6, effects are registered using:
/// - `effect::on::<E>().then()` for single variant handling
/// - `on!` macro for multi-variant matching with explicit event returns
/// - `.on_error()` for global error handling (errors no longer stop other effects)
fn build_engine(server_deps: ServerDeps) -> AppEngine {
    Engine::with_deps(server_deps)
        // Global error handler - logs all effect errors
        .on_error(|error, _type_id, _ctx| async move {
            tracing::error!(error = %error, "Effect failed");
        })
        // Auth domain
        .with_effect(auth_effect())
        // Member domain
        .with_effect(member_effect())
        // Chat domain
        .with_effect(chat_effect())
        // Website domain
        .with_effect(website_effect())
        // Crawling domain (job chaining handled by JobRunner + job_handlers)
        .with_effect(mark_no_listings_effect())
        // Posts domain (composite effect)
        .with_effect(post_composite_effect())
        // Website approval domain
        .with_effect(website_approval_effect())
        // Providers domain
        .with_effect(provider_effect())
}

/// Build the Axum application router
///
/// The Engine is created with effects for the seesaw 0.7.2 architecture.
/// GraphQL mutations use engine.activate(state).process() to execute workflows.
///
/// Returns (Router, Arc<AppEngine>, Arc<ServerDeps>) - engine and deps are needed for scheduled tasks.
pub async fn build_app(
    pool: PgPool,
    openai_api_key: String,
    tavily_api_key: String,
    firecrawl_api_key: Option<String>,
    expo_access_token: Option<String>,
    twilio_account_sid: String,
    twilio_auth_token: String,
    twilio_verify_service_sid: String,
    jwt_secret: String,
    jwt_issuer: String,
    allowed_origins: Vec<String>,
    test_identifier_enabled: bool,
    admin_identifiers: Vec<String>,
    pii_scrubbing_enabled: bool,
    pii_use_gpt_detection: bool,
) -> (Router, Arc<AppEngine>, Arc<ServerDeps>) {
    // Create GraphQL schema (singleton)
    let schema = Arc::new(create_schema());

    // Create Twilio service (needed by ServerDeps)
    let twilio_options = TwilioOptions {
        account_sid: twilio_account_sid,
        auth_token: twilio_auth_token,
        service_id: twilio_verify_service_sid,
    };
    let twilio = Arc::new(TwilioService::new(twilio_options));

    // Create OpenAI client (shared across effects and GraphQL)
    let openai_client = Arc::new(OpenAIClient::new(openai_api_key.clone()));

    // Clone OpenAI API key for embedding service before it's consumed
    let embedding_api_key = openai_api_key.clone();

    // Create PII detector based on configuration
    let pii_detector = crate::kernel::pii::create_pii_detector(
        pii_scrubbing_enabled,
        pii_use_gpt_detection,
        Some(openai_api_key),
    );

    // Create ingestor with SSRF protection
    // Use Firecrawl if API key is provided, otherwise basic HTTP
    let ingestor: Arc<dyn extraction::Ingestor> = if let Some(key) = firecrawl_api_key.clone() {
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

    // Create web searcher (Tavily)
    let web_searcher: Arc<dyn extraction::WebSearcher> =
        Arc::new(extraction::TavilyWebSearcher::new(tavily_api_key));

    // Create extraction service (required for all crawling operations)
    let extraction_service = create_extraction_service(pool.clone())
        .await
        .expect("Failed to create extraction service - this is required for server operation");

    // Create job queue with PostgreSQL backend
    let job_queue: Arc<dyn crate::kernel::jobs::JobQueue> =
        Arc::new(PostgresJobQueue::new(pool.clone()));

    // Clone for job runner before moving into ServerDeps
    let job_queue_for_runner = job_queue.clone();

    // Create JWT service
    let jwt_service = Arc::new(JwtService::new(&jwt_secret, jwt_issuer.clone()));

    let server_deps = ServerDeps::new(
        pool.clone(),
        ingestor,
        openai_client.clone(),
        Arc::new(crate::common::utils::EmbeddingService::new(
            embedding_api_key,
        )),
        Arc::new(crate::common::utils::ExpoClient::new(expo_access_token)),
        Arc::new(TwilioAdapter::new(twilio.clone())),
        web_searcher,
        pii_detector,
        Some(extraction_service),
        job_queue,
        jwt_service.clone(),
        test_identifier_enabled,
        admin_identifiers,
    );

    // Clone server_deps before moving into engine
    let server_deps_arc = Arc::new(server_deps.clone());

    // Build seesaw engine with effects (0.7.2 pattern)
    let engine = Arc::new(build_engine(server_deps));

    // Create job registry and register all job handlers
    let mut job_registry = JobRegistry::new();
    register_crawling_jobs(&mut job_registry);
    let job_registry = Arc::new(job_registry);

    // Create and spawn the job runner as a background task
    let runner = JobRunner::new(
        job_queue_for_runner,
        job_registry,
        server_deps_arc.clone(),
    );
    tokio::spawn(async move {
        if let Err(e) = runner.run().await {
            tracing::error!(error = %e, "Job runner exited with error");
        }
    });

    // Create shared app state
    let app_state = AxumAppState {
        db_pool: pool.clone(),
        engine: engine.clone(),
        server_deps: server_deps_arc.clone(),
        twilio: twilio.clone(),
        jwt_service: jwt_service.clone(),
        openai_client: openai_client.clone(),
    };

    // CORS configuration - allow any origin for development
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE]);

    // Clone jwt_service for middleware closure
    let jwt_service_for_middleware = jwt_service.clone();

    // Rate limiting configuration
    // GraphQL: 100 requests per minute per IP (10/sec with burst of 20)
    // Prevents API abuse, DoS attacks, and resource exhaustion
    let rate_limit_config = std::sync::Arc::new(
        GovernorConfigBuilder::default()
            .per_second(10) // Base rate: 10 requests per second
            .burst_size(20) // Allow bursts up to 20
            .use_headers() // Extract IP from X-Forwarded-For header
            .finish()
            .expect("Rate limiter configuration is valid and should never fail"),
    );

    let rate_limit_layer = GovernorLayer {
        config: rate_limit_config,
    };

    // Build router
    let mut router = Router::new()
        // GraphQL endpoints with rate limiting
        .route("/graphql", post(graphql_handler))
        .route("/graphql/batch", post(graphql_batch_handler));

    // GraphQL playground only in debug builds (development)
    #[cfg(debug_assertions)]
    {
        router = router.route("/graphql", get(graphql_playground));
    }

    let app = router
        // Health check (no rate limit)
        .route("/health", get(health_handler))
        // Note: web-app static file routes removed - web-next runs as separate service
        // Middleware layers (applied in reverse order - last added runs first)
        .layer(middleware::from_fn(create_graphql_context)) // Create GraphQL context
        .layer(middleware::from_fn(move |req, next| {
            jwt_auth_middleware(jwt_service_for_middleware.clone(), req, next)
        })) // JWT authentication
        .layer(rate_limit_layer) // Rate limit: 100 req/min per IP
        .layer(middleware::from_fn(extract_client_ip))
        .layer(Extension(app_state)) // Add shared state (must be after middlewares that need it)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        // State (schema for GraphQL handlers)
        .with_state(schema);

    (app, engine, server_deps_arc)
}
