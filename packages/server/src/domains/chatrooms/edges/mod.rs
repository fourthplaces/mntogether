//! Chat domain edges
//!
//! Edge structs implement seesaw_core::Edge and are executed via engine.run().
//! Query functions are simple database reads.
//! Internal edges react to fact events and emit request events for chains.

mod create_chat;
mod send_message;

pub mod internal;
pub mod query;

// Edge structs for mutations
pub use create_chat::CreateChat;
pub use send_message::SendMessage;

// Query functions
pub use query::*;

// Internal edge functions
pub use internal::*;

// Re-export signal_typing (ephemeral, doesn't use Edge pattern)
use crate::common::ContainerId;
use crate::server::graphql::context::GraphQLContext;
use juniper::{FieldError, FieldResult};

/// Signal that the user is typing (ephemeral event, no response needed)
pub async fn signal_typing(ctx: &GraphQLContext, container_id: String) -> FieldResult<bool> {
    let member_id = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?
        .member_id;

    let container_id = ContainerId::parse(&container_id)?;

    // Emit typing event directly to bus (ephemeral, not persisted)
    ctx.bus.emit(crate::domains::chatrooms::events::TypingEvent::Started {
        container_id,
        member_id,
    });

    Ok(true)
}
