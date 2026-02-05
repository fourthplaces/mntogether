//! Application setup and server configuration.

use std::sync::Arc;

use axum::{
    extract::{Extension, Request, State},
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method,
    },
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use seesaw_core::{effect, Engine};
use seesaw_viz::{MermaidRenderer, RenderOptions, SpanCollector, StateFormatter};
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

// =============================================================================
// Seesaw Visualizer Routes
// =============================================================================

/// No-op formatter for unit state (we don't track state, just events)
#[derive(Clone, Copy)]
struct NoOpFormatter;

impl StateFormatter<()> for NoOpFormatter {
    fn serialize(&self, _state: &()) -> Result<serde_json::Value, seesaw_viz::formatter::FormatterError> {
        Ok(serde_json::Value::Null)
    }

    fn diff(&self, _prev: &(), _next: &()) -> Result<Option<serde_json::Value>, seesaw_viz::formatter::FormatterError> {
        Ok(None)
    }
}

/// State for the seesaw visualizer routes (wrapped in Arc for Extension)
#[derive(Clone)]
struct VizState {
    collector: Arc<SpanCollector<()>>,
}

/// Create the visualizer router to be nested at /seesaw
fn viz_routes<S>(collector: Arc<SpanCollector<()>>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let state = VizState { collector };

    Router::new()
        .route("/", get(viz_index_handler))
        .route("/api/graph", get(viz_graph_handler))
        .route("/api/diagram", get(viz_diagram_handler))
        .layer(Extension(state))
}

async fn viz_index_handler() -> Html<&'static str> {
    // Inline HTML viewer with Mermaid.js
    Html(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Seesaw Visualizer</title>
    <script src="https://cdn.jsdelivr.net/npm/mermaid/dist/mermaid.min.js"></script>
    <style>
        body { font-family: system-ui, sans-serif; margin: 2rem; background: #1a1a2e; color: #eee; }
        h1 { color: #7c3aed; }
        .controls { margin: 1rem 0; }
        button { background: #7c3aed; color: white; border: none; padding: 0.5rem 1rem; border-radius: 4px; cursor: pointer; margin-right: 0.5rem; }
        button:hover { background: #6d28d9; }
        #diagram { background: #16213e; padding: 1rem; border-radius: 8px; margin-top: 1rem; }
        .stats { font-size: 0.875rem; color: #888; margin-top: 1rem; }
        pre { background: #0f0f23; padding: 1rem; border-radius: 4px; overflow-x: auto; }
    </style>
</head>
<body>
    <h1>🎯 Seesaw Event Visualizer</h1>
    <div class="controls">
        <button onclick="refresh()">Refresh</button>
        <button onclick="toggleRaw()">Toggle Raw</button>
    </div>
    <div id="diagram"></div>
    <div class="stats" id="stats"></div>
    <pre id="raw" style="display:none"></pre>

    <script>
        mermaid.initialize({ startOnLoad: false, theme: 'dark' });

        async function refresh() {
            const res = await fetch('/seesaw/api/diagram');
            const data = await res.json();

            document.getElementById('raw').textContent = data.diagram;

            const container = document.getElementById('diagram');
            container.innerHTML = '';

            if (data.diagram && data.diagram.trim()) {
                const { svg } = await mermaid.render('mermaid-svg', data.diagram);
                container.innerHTML = svg;
            } else {
                container.innerHTML = '<p style="color:#888">No events recorded yet. Trigger some actions!</p>';
            }

            document.getElementById('stats').textContent =
                `Total spans: ${data.stats?.total_spans || 0} | Root spans: ${data.stats?.root_spans || 0}`;
        }

        function toggleRaw() {
            const raw = document.getElementById('raw');
            raw.style.display = raw.style.display === 'none' ? 'block' : 'none';
        }

        refresh();
        setInterval(refresh, 5000);
    </script>
</body>
</html>"#,
    )
}

async fn viz_graph_handler(Extension(state): Extension<VizState>) -> impl IntoResponse {
    let graph = state.collector.graph().await;
    axum::Json(graph)
}

async fn viz_diagram_handler(Extension(state): Extension<VizState>) -> impl IntoResponse {
    let graph = state.collector.graph().await;
    let renderer = MermaidRenderer::new(RenderOptions {
        group_by_component: true,
        show_timings: true,
        ..Default::default()
    });
    let diagram = renderer.render(&graph);
    let stats = state.collector.stats().await;

    axum::Json(serde_json::json!({
        "diagram": diagram,
        "stats": stats,
    }))
}

// =============================================================================
// Application State & Middleware
// =============================================================================

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
/// In seesaw 0.7.8, effects are registered using:
/// - `effect::on::<E>().then()` for single variant handling
/// - `on!` macro for multi-variant matching with explicit event returns
/// - `.on_error()` for global error handling (errors no longer stop other effects)
/// - `effect::on_any().then()` for observability (seesaw-viz integration)
fn build_engine(server_deps: ServerDeps, collector: Arc<SpanCollector<()>>) -> AppEngine {
    // Create observer for span collection (no state tracking, just events)
    let observer = collector.create_observer(NoOpFormatter);

    Engine::with_deps(server_deps)
        // Global error handler - logs all effect errors
        .on_error(|error, _type_id, _ctx| async move {
            tracing::error!(error = %error, "Effect failed");
        })
        // Seesaw-viz observer - records all events for visualization
        .with_effect(effect::on_any().then(move |event, _ctx| {
            let observer = observer.clone();
            async move {
                // Get human-readable type name from the event value
                let type_name = std::any::type_name_of_val(&*event.value);
                // Clean up the type name (remove crate paths for readability)
                let short_name = type_name
                    .rsplit("::")
                    .next()
                    .unwrap_or(type_name)
                    .to_string();

                // Record span (non-blocking via async channel)
                let _ = observer
                    .record(
                        uuid::Uuid::new_v4(), // Generate event ID
                        short_name,
                        event.type_id,
                        None, // No parent tracking in basic AnyEvent
                        None, // Module path not available
                        None, // Effect name not available
                        None, // No state tracking
                        None,
                    )
                    .await;

                Ok(())
            }
        }))
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

    // Create seesaw-viz span collector for event visualization
    let viz_collector = Arc::new(SpanCollector::<()>::new(1000));

    // Build seesaw engine with effects (0.7.8 pattern)
    let engine = Arc::new(build_engine(server_deps, viz_collector.clone()));

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

    // Rate limiting configuration (production only)
    // GraphQL: 100 requests per minute per IP (10/sec with burst of 20)
    // Prevents API abuse, DoS attacks, and resource exhaustion
    // Disabled in development where all requests share localhost IP
    #[cfg(not(debug_assertions))]
    let rate_limit_layer = {
        let rate_limit_config = std::sync::Arc::new(
            GovernorConfigBuilder::default()
                .per_second(10) // Base rate: 10 requests per second
                .burst_size(20) // Allow bursts up to 20
                .use_headers() // Extract IP from X-Forwarded-For header
                .finish()
                .expect("Rate limiter configuration is valid and should never fail"),
        );
        GovernorLayer {
            config: rate_limit_config,
        }
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

    // Add seesaw visualizer routes (debug builds only)
    #[cfg(debug_assertions)]
    let router = router.nest("/seesaw", viz_routes(viz_collector));

    let router = router
        // Health check (no rate limit)
        .route("/health", get(health_handler));

    // Middleware layers (applied in reverse order - last added runs first)
    // Rate limiting only in production
    #[cfg(not(debug_assertions))]
    let router = router
        .layer(middleware::from_fn(create_graphql_context))
        .layer(middleware::from_fn(move |req, next| {
            jwt_auth_middleware(jwt_service_for_middleware.clone(), req, next)
        }))
        .layer(rate_limit_layer)
        .layer(middleware::from_fn(extract_client_ip));

    #[cfg(debug_assertions)]
    let router = router
        .layer(middleware::from_fn(create_graphql_context))
        .layer(middleware::from_fn(move |req, next| {
            jwt_auth_middleware(jwt_service_for_middleware.clone(), req, next)
        }))
        .layer(middleware::from_fn(extract_client_ip));

    let app = router
        .layer(Extension(app_state)) // Add shared state (must be after middlewares that need it)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        // State (schema for GraphQL handlers)
        .with_state(schema);

    (app, engine, server_deps_arc)
}
