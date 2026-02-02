//! Auth domain effect - handles cascading reactions to fact events
//!
//! Auth has no cascading effects - all events are terminal.

use seesaw_core::effect;
use std::sync::Arc;

use super::events::AuthEvent;
use crate::common::AppState;
use crate::kernel::ServerDeps;

/// Build the auth effect handler.
///
/// Auth has no cascading effects - all events are terminal.
pub fn auth_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<AuthEvent>().run(|_event: Arc<AuthEvent>, _ctx| async move {
        // All auth events are terminal - no cascading actions needed
        Ok(())
    })
}
