//! Application state for the seesaw engine.

use uuid::Uuid;

/// Application state passed to engine.activate().
///
/// This is the same for all domains - just tracks request-scoped data
/// like visitor ID. Domain-specific results come from action return values.
#[derive(Clone, Default)]
pub struct AppState {
    pub visitor_id: Option<Uuid>,
}
