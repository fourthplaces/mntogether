//! Entry-point actions for chatrooms domain
//!
//! These actions are called directly from GraphQL mutations via `process()`.
//! They do the work, emit fact events, and return ReadResult for deferred reads.

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::info;
use uuid::Uuid;

use crate::common::{AppState, ContainerId, MemberId, MessageId, ReadResult};
use crate::domains::chatrooms::events::ChatEvent;
use crate::domains::chatrooms::models::{Container, Message};
use crate::domains::tag::{Tag, Taggable};
use crate::kernel::ServerDeps;

// ============================================================================
// Entry Point: Create Container
// ============================================================================

/// Create a new chat container.
///
/// Entry point for GraphQL mutation. Does the actual work, emits fact event.
pub async fn create_container(
    container_type: String,
    entity_id: Option<Uuid>,
    language: String,
    _requested_by: Option<MemberId>,
    with_agent: Option<String>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<ReadResult<Container>> {
    info!(container_type = %container_type, ?with_agent, "Creating chat container");

    let container = Container::create(
        container_type.clone(),
        entity_id,
        language,
        &ctx.deps().db_pool,
    )
    .await?;

    // Tag container with agent config if provided
    if let Some(ref agent_config) = with_agent {
        info!(container_id = %container.id, agent_config = %agent_config, "Tagging container with agent");
        let tag = Tag::find_or_create("with_agent", agent_config, None, &ctx.deps().db_pool).await?;
        Taggable::create_container_tag(container.id, tag.id, &ctx.deps().db_pool).await?;
    }

    ctx.emit(ChatEvent::ContainerCreated {
        container: container.clone(),
        with_agent,
    });

    Ok(ReadResult::new(container.id, ctx.deps().db_pool.clone()))
}

// ============================================================================
// Entry Point: Send Message
// ============================================================================

/// Send a user message to a container.
///
/// Entry point for GraphQL mutation. Does the actual work, emits fact event.
pub async fn send_message(
    container_id: ContainerId,
    content: String,
    author_id: Option<MemberId>,
    parent_message_id: Option<MessageId>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<ReadResult<Message>> {
    info!(container_id = %container_id, "Creating user message");

    // Get next sequence number
    let sequence_number = Message::next_sequence_number(container_id, &ctx.deps().db_pool).await?;

    // Create message
    let message = Message::create(
        container_id,
        "user".to_string(),
        content,
        author_id,
        Some("approved".to_string()),
        parent_message_id,
        sequence_number,
        &ctx.deps().db_pool,
    )
    .await?;

    // Update container activity
    Container::touch_activity(container_id, &ctx.deps().db_pool).await?;

    ctx.emit(ChatEvent::MessageCreated {
        message: message.clone(),
    });

    Ok(ReadResult::new(message.id, ctx.deps().db_pool.clone()))
}

// ============================================================================
// Entry Point: Create Message (internal, any role)
// ============================================================================

/// Create a message with specific role.
///
/// Used internally for assistant messages, etc.
pub async fn create_message(
    container_id: ContainerId,
    role: String,
    content: String,
    author_id: Option<MemberId>,
    parent_message_id: Option<MessageId>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<ReadResult<Message>> {
    info!(container_id = %container_id, role = %role, "Creating message");

    // Get next sequence number
    let sequence_number = Message::next_sequence_number(container_id, &ctx.deps().db_pool).await?;

    // Create message
    let message = Message::create(
        container_id,
        role,
        content,
        author_id,
        Some("approved".to_string()),
        parent_message_id,
        sequence_number,
        &ctx.deps().db_pool,
    )
    .await?;

    // Update container activity
    Container::touch_activity(container_id, &ctx.deps().db_pool).await?;

    ctx.emit(ChatEvent::MessageCreated {
        message: message.clone(),
    });

    Ok(ReadResult::new(message.id, ctx.deps().db_pool.clone()))
}
