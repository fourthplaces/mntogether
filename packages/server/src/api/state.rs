use std::sync::Arc;

use crate::kernel::ServerDeps;

/// Shared application state for Axum handlers.
///
/// Wraps `Arc<ServerDeps>` so it can be cheaply cloned by Axum's state layer.
#[derive(Clone)]
pub struct AppState {
    pub deps: Arc<ServerDeps>,
}
