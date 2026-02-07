//! Application setup and server configuration.

use std::sync::Arc;

use axum::{
    extract::{Extension, Request},
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE},
        Method,
    },
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Router,
};
use sqlx::PgPool;
#[cfg(not(debug_assertions))]
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use twilio::{TwilioOptions, TwilioService};

use crate::domains::auth::JwtService;
use crate::kernel::{
    create_extraction_service, OpenAIClient, ServerDeps, StreamHub, TwilioAdapter,
};
use crate::server::graphql::{create_schema, DataLoaders, GraphQLContext};
use crate::WorkflowClient;
use crate::server::middleware::{extract_client_ip, jwt_auth_middleware, AuthUser};
use crate::server::routes::{
    graphql_batch_handler, graphql_handler, graphql_playground, health_handler,
    stream::stream_handler,
};

// =============================================================================
// Application State & Middleware
// =============================================================================

/// Shared application state
#[derive(Clone)]
pub struct AxumAppState {
    pub db_pool: PgPool,
    pub server_deps: Arc<ServerDeps>,
    pub twilio: Arc<TwilioService>,
    pub jwt_service: Arc<JwtService>,
    pub openai_client: Arc<OpenAIClient>,
    pub stream_hub: StreamHub,
    pub workflow_client: Arc<WorkflowClient>,
}

/// Middleware to create GraphQLContext per-request
async fn create_graphql_context(
    Extension(state): Extension<AxumAppState>,
    mut request: Request,
    next: Next,
) -> Response {
    // Extract auth user from request extensions (populated by jwt_auth_middleware)
    let auth_user = request.extensions().get::<AuthUser>().cloned();

    // Create per-request DataLoaders for batching N+1 queries
    let loaders = Arc::new(DataLoaders::new(Arc::new(state.db_pool.clone())));

    // Create GraphQL context with shared state + per-request auth
    let context = GraphQLContext::new(
        state.db_pool.clone(),
        state.server_deps.clone(),
        auth_user,
        state.twilio.clone(),
        state.jwt_service.clone(),
        state.openai_client.clone(),
        loaders,
        state.workflow_client.clone(),
    );

    // Add context to request extensions
    request.extensions_mut().insert(context);

    next.run(request).await
}

/// Build the Axum application router
///
/// Returns (Router, Arc<ServerDeps>) - deps needed for scheduled tasks.
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
    _allowed_origins: Vec<String>,
    test_identifier_enabled: bool,
    admin_identifiers: Vec<String>,
    pii_scrubbing_enabled: bool,
    pii_use_gpt_detection: bool,
) -> (Router, Arc<ServerDeps>) {
    // Create GraphQL schema (singleton)
    let schema = Arc::new(create_schema());

    // Create Twilio service (needed by ServerDeps)
    let twilio_options = TwilioOptions {
        account_sid: twilio_account_sid,
        auth_token: twilio_auth_token,
        service_id: twilio_verify_service_sid,
    };
    let twilio = Arc::new(TwilioService::new(twilio_options));

    // Create OpenAI client (shared across workflows and GraphQL)
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

    // Create JWT service
    let jwt_service = Arc::new(JwtService::new(&jwt_secret, jwt_issuer.clone()));

    // Create StreamHub for real-time SSE streaming
    let stream_hub = StreamHub::new();

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
        jwt_service.clone(),
        stream_hub.clone(),
        test_identifier_enabled,
        admin_identifiers,
    );

    let server_deps_arc = Arc::new(server_deps);

    // Create Restate workflow client
    let workflow_url = std::env::var("RESTATE_URL")
        .unwrap_or_else(|_| "http://localhost:9070".to_string());
    let workflow_client = Arc::new(WorkflowClient::new(workflow_url));

    // Create shared app state
    let app_state = AxumAppState {
        db_pool: pool.clone(),
        server_deps: server_deps_arc.clone(),
        twilio: twilio.clone(),
        jwt_service: jwt_service.clone(),
        openai_client: openai_client.clone(),
        stream_hub,
        workflow_client,
    };

    // CORS configuration - allow any origin for development
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE]);

    // Clone jwt_service for middleware closure
    let jwt_service_for_middleware = jwt_service.clone();

    // Rate limiting configuration (production only)
    #[cfg(not(debug_assertions))]
    let rate_limit_layer = {
        let rate_limit_config = std::sync::Arc::new(
            GovernorConfigBuilder::default()
                .per_second(10)
                .burst_size(20)
                .use_headers()
                .finish()
                .expect("Rate limiter configuration is valid and should never fail"),
        );
        GovernorLayer {
            config: rate_limit_config,
        }
    };

    // SSE routes — separate router, no GraphQL middleware
    // Has its own auth via query param / Authorization header
    let sse_routes = Router::new().route("/api/streams/:topic", get(stream_handler));

    // GraphQL routes — get JWT + GraphQL context middleware
    let mut graphql_routes = Router::new()
        .route("/graphql", post(graphql_handler))
        .route("/graphql/batch", post(graphql_batch_handler));

    // GraphQL playground only in debug builds (development)
    #[cfg(debug_assertions)]
    {
        graphql_routes = graphql_routes.route("/graphql", get(graphql_playground));
    }

    // Apply GraphQL-specific middleware
    #[cfg(not(debug_assertions))]
    let graphql_routes = graphql_routes
        .layer(middleware::from_fn(create_graphql_context))
        .layer(middleware::from_fn(move |req, next| {
            jwt_auth_middleware(jwt_service_for_middleware.clone(), req, next)
        }))
        .layer(rate_limit_layer)
        .layer(middleware::from_fn(extract_client_ip));

    #[cfg(debug_assertions)]
    let graphql_routes = graphql_routes
        .layer(middleware::from_fn(create_graphql_context))
        .layer(middleware::from_fn(move |req, next| {
            jwt_auth_middleware(jwt_service_for_middleware.clone(), req, next)
        }))
        .layer(middleware::from_fn(extract_client_ip));

    // Merge all routes — shared layers applied to everything
    let app = graphql_routes
        .merge(sse_routes)
        .route("/health", get(health_handler))
        .layer(Extension(app_state))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(schema);

    (app, server_deps_arc)
}
