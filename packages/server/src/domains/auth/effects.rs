//! Auth domain effect - handles cascading reactions to fact events
//!
//! Auth has no cascading effects - all events are terminal.
//! Since auth events don't trigger cascading effects, actions are called
//! directly without going through the engine (see schema.rs mutations).

use seesaw_core::effect;

use super::events::AuthEvent;
use crate::common::AppState;
use crate::kernel::ServerDeps;

/// Build the auth effect handler.
///
/// Auth has no cascading effects - all events are terminal.
/// This effect is registered for completeness but does nothing.
pub fn auth_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<AuthEvent>().id("auth_terminal").then(|_event, _ctx| async move {
        // All auth events are terminal - no cascading actions needed
        Ok(())
    })
}
