pub mod auth;
pub mod error;
pub mod routes;
pub mod state;

use axum::routing::get;
use axum::Router;

use self::state::AppState;

/// Build the main Axum router with all API routes.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .merge(routes::router())
        .with_state(state)
}
