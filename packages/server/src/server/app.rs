use crate::server::graphql::{GraphQLContext, create_schema};
use crate::server::middleware::extract_client_ip;
use crate::server::routes::{graphql_batch_handler, graphql_handler, graphql_playground, health_handler};
use axum::{
    extract::Extension,
    middleware,
    routing::{get, post},
    Router,
};
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

/// Build the Axum application router
pub fn build_app(
    pool: PgPool,
    firecrawl_api_key: String,
    openai_api_key: String,
) -> Router {
    // Create GraphQL schema (singleton)
    let schema = Arc::new(create_schema());

    // Create GraphQL context factory
    let context = GraphQLContext::new(pool.clone(), firecrawl_api_key, openai_api_key);

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    Router::new()
        // GraphQL endpoints
        .route("/graphql", post(graphql_handler))
        .route("/graphql/batch", post(graphql_batch_handler))
        .route("/graphql", get(graphql_playground))
        // Health check
        .route("/health", get(health_handler))
        // State and extensions
        .with_state(schema)
        .with_state(pool)
        .layer(Extension(context))
        // Middleware
        .layer(middleware::from_fn(extract_client_ip))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}
