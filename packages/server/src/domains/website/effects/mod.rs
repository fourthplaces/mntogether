//! Website domain effect - handles cascading reactions to fact events
//!
//! Following the direct-call pattern, all website events are terminal.
//! GraphQL calls actions directly, actions emit fact events.
//! No cascading effects are needed for the website domain.

use seesaw_core::effect;
use std::sync::Arc;

use crate::common::AppState;
use crate::domains::website::events::WebsiteEvent;
use crate::kernel::ServerDeps;

/// Build the website effect handler.
///
/// Website has no cascading effects - all events are terminal.
pub fn website_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<WebsiteEvent>().run(|_event: Arc<WebsiteEvent>, _ctx| async move {
        // All website events are terminal - no cascading actions needed
        // Events: WebsiteApproved, WebsiteRejected, WebsiteSuspended, CrawlSettingsUpdated
        Ok(())
    })
}
