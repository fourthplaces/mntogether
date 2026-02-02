//! Website domain internal edges - event-to-event reactions
//!
//! Internal edges observe fact events and emit new request events.
//! This replaces the machine's decide() logic in seesaw 0.3.0.
//!
//! Currently the website domain has no event chains that require internal edges.
//! All workflows are simple request → effect → fact patterns.

use crate::domains::website::events::WebsiteEvent;

/// List of all website domain internal edges.
///
/// Currently empty as the website domain doesn't have event chains.
pub fn all_edges() -> Vec<fn(&WebsiteEvent) -> Option<WebsiteEvent>> {
    vec![]
}
