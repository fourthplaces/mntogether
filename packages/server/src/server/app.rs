use crate::domains::matching::{
    commands::MatchingCommand,
    effects::MatchingEffect,
    machines::{MatchingCoordinatorMachine, MatchingMachine},
};
use crate::domains::member::{
    commands::MemberCommand, effects::RegistrationEffect, machines::MemberMachine,
};
use crate::domains::organization::{
    commands::OrganizationCommand,
    effects::{
        AIEffect, FirecrawlClient, NeedEffect, NeedExtractor, ScraperEffect, ServerDeps, SyncEffect,
    },
    machines::OrganizationMachine,
};
use crate::server::auth::SessionStore;
use crate::server::graphql::{create_schema, GraphQLContext};
use crate::server::middleware::{extract_client_ip, session_auth_middleware, AuthUser};
use crate::server::routes::{
    graphql_batch_handler, graphql_handler, graphql_playground, health_handler,
};
use crate::server::static_files::serve_admin;
use axum::{
    extract::{Extension, Request},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Router,
};
use seesaw::{EngineBuilder, EngineHandle, EventBus};
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use twilio::{TwilioOptions, TwilioService};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub bus: EventBus,
    pub twilio: Arc<TwilioService>,
    pub session_store: Arc<SessionStore>,
}

/// Middleware to create GraphQLContext per-request
async fn create_graphql_context(
    Extension(state): Extension<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    // Extract auth user from request extensions (populated by session_auth_middleware)
    let auth_user = request.extensions().get::<AuthUser>().cloned();

    // Create GraphQL context with shared state + per-request auth
    let context = GraphQLContext::new(
        state.db_pool.clone(),
        state.bus.clone(),
        auth_user,
        state.twilio.clone(),
        state.session_store.clone(),
    );

    // Add context to request extensions
    request.extensions_mut().insert(context);

    next.run(request).await
}

/// Build the Axum application router and engine handle
pub fn build_app(
    pool: PgPool,
    firecrawl_api_key: String,
    openai_api_key: String,
    expo_access_token: Option<String>,
    twilio_account_sid: String,
    twilio_auth_token: String,
    twilio_verify_service_sid: String,
) -> (Router, EngineHandle) {
    // Create GraphQL schema (singleton)
    let schema = Arc::new(create_schema());

    // Create server dependencies for effects
    let server_deps = ServerDeps {
        db_pool: pool.clone(),
        firecrawl_client: FirecrawlClient::new(firecrawl_api_key),
        need_extractor: NeedExtractor::new(openai_api_key.clone()),
        embedding_service: crate::common::utils::EmbeddingService::new(openai_api_key),
        expo_client: crate::common::utils::ExpoClient::new(expo_access_token),
    };

    // Build and start seesaw engine
    let engine = EngineBuilder::new(server_deps)
        // Organization domain
        .with_machine(OrganizationMachine::new())
        .with_effect::<OrganizationCommand, _>(ScraperEffect)
        .with_effect::<OrganizationCommand, _>(AIEffect)
        .with_effect::<OrganizationCommand, _>(SyncEffect)
        .with_effect::<OrganizationCommand, _>(NeedEffect)
        // Member domain
        .with_machine(MemberMachine::new())
        .with_effect::<MemberCommand, _>(RegistrationEffect)
        // Matching domain
        .with_machine(MatchingMachine::new())
        .with_effect::<MatchingCommand, _>(MatchingEffect)
        // Cross-domain coordinator (OrganizationEvent → MatchingCommand)
        .with_machine(MatchingCoordinatorMachine::new())
        // TODO: Integrate job queue
        // .with_job_queue(job_manager)
        .build();

    let handle = engine.start();
    let bus = handle.bus().clone();

    // Create Twilio service
    let twilio_options = TwilioOptions {
        account_sid: twilio_account_sid,
        auth_token: twilio_auth_token,
        service_id: twilio_verify_service_sid,
    };
    let twilio = Arc::new(TwilioService::new(twilio_options));

    // Create session store
    let session_store = Arc::new(SessionStore::new());

    // Create shared app state
    let app_state = AppState {
        db_pool: pool.clone(),
        bus,
        twilio: twilio.clone(),
        session_store: session_store.clone(),
    };

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    let router = Router::new()
        // GraphQL endpoints
        .route("/graphql", post(graphql_handler))
        .route("/graphql/batch", post(graphql_batch_handler))
        .route("/graphql", get(graphql_playground))
        // Health check
        .route("/health", get(health_handler))
        // Static file serving for admin SPA
        .route("/admin", get(serve_admin))
        .route("/admin/*path", get(serve_admin))
        // State (schema for GraphQL handlers)
        .with_state(schema)
        // Middleware layers (applied in reverse order)
        .layer(middleware::from_fn(create_graphql_context)) // Last: Create GraphQL context
        .layer(middleware::from_fn_with_state(
            session_store.clone(),
            session_auth_middleware,
        )) // Second: Extract session auth
        .layer(Extension(app_state)) // First: Add shared state
        .layer(middleware::from_fn(extract_client_ip))
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    (router, handle)
}
