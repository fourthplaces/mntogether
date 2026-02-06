//! Auth domain effect - handles cascading reactions to fact events
//!
//! Auth has no cascading effects - all events are terminal.
//! Since auth events don't trigger cascading effects, actions are called
//! directly without going through the engine (see schema.rs mutations).

use anyhow::Result;
use seesaw_core::{effect, effects, EffectContext};

use super::events::AuthEvent;
use crate::common::AppState;
use crate::kernel::ServerDeps;

#[effects]
pub mod handlers {
    use super::*;

    #[effect(on = AuthEvent, id = "auth_terminal")]
    async fn auth_terminal(
        _event: AuthEvent,
        _ctx: EffectContext<AppState, ServerDeps>,
    ) -> Result<()> {
        // All auth events are terminal - no cascading actions needed
        Ok(())
    }
}
