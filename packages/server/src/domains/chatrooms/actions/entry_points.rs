//! Entry-point actions for chatrooms domain
//!
//! These actions are called directly from GraphQL mutations via `process()`.
//! They do the work and return fact events for the effect system to dispatch.

use anyhow::Result;
use tracing::info;
use uuid::Uuid;

use crate::common::{ContainerId, MemberId, MessageId};
use crate::domains::chatrooms::events::ChatEvent;
use crate::domains::chatrooms::models::{Container, Message};
use crate::domains::tag::{Tag, Taggable};
use crate::kernel::ServerDeps;

// ============================================================================
// Entry Point: Create Container
// ============================================================================

/// Create a new chat container.
///
/// Entry point for GraphQL mutation. Does the actual work, returns fact event.
pub async fn create_container(
    container_type: String,
    entity_id: Option<Uuid>,
    language: String,
    _requested_by: Option<MemberId>,
    with_agent: Option<String>,
    deps: &ServerDeps,
) -> Result<ChatEvent> {
    info!(container_type = %container_type, ?with_agent, "Creating chat container");

    let container =
        Container::create(container_type.clone(), entity_id, language, &deps.db_pool).await?;

    // Tag container with agent config if provided
    if let Some(ref agent_config) = with_agent {
        info!(container_id = %container.id, agent_config = %agent_config, "Tagging container with agent");
        let tag = Tag::find_or_create("with_agent", agent_config, None, &deps.db_pool).await?;
        Taggable::create_container_tag(container.id, tag.id, &deps.db_pool).await?;
    }

    Ok(ChatEvent::ContainerCreated {
        container,
        with_agent,
    })
}

// ============================================================================
// Entry Point: Send Message
// ============================================================================

/// Send a user message to a container.
///
/// Entry point for GraphQL mutation. Does the actual work, returns fact event.
pub async fn send_message(
    container_id: ContainerId,
    content: String,
    author_id: Option<MemberId>,
    parent_message_id: Option<MessageId>,
    deps: &ServerDeps,
) -> Result<ChatEvent> {
    info!(container_id = %container_id, "Creating user message");

    // Rate limit anonymous senders: max 10 messages per minute
    if author_id.is_none() {
        let one_minute_ago = chrono::Utc::now() - chrono::Duration::minutes(1);
        let recent_count =
            Message::count_since(container_id, one_minute_ago, &deps.db_pool).await?;
        if recent_count >= 10 {
            anyhow::bail!("Rate limit exceeded: too many messages. Please wait a moment.");
        }
    }

    // Get next sequence number
    let sequence_number = Message::next_sequence_number(container_id, &deps.db_pool).await?;

    // Create message
    let message = Message::create(
        container_id,
        "user".to_string(),
        content,
        author_id,
        Some("approved".to_string()),
        parent_message_id,
        sequence_number,
        &deps.db_pool,
    )
    .await?;

    // Update container activity
    Container::touch_activity(container_id, &deps.db_pool).await?;

    Ok(ChatEvent::MessageCreated { message })
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
    deps: &ServerDeps,
) -> Result<ChatEvent> {
    info!(container_id = %container_id, role = %role, "Creating message");

    // Get next sequence number
    let sequence_number = Message::next_sequence_number(container_id, &deps.db_pool).await?;

    // Create message
    let message = Message::create(
        container_id,
        role,
        content,
        author_id,
        Some("approved".to_string()),
        parent_message_id,
        sequence_number,
        &deps.db_pool,
    )
    .await?;

    // Update container activity
    Container::touch_activity(container_id, &deps.db_pool).await?;

    Ok(ChatEvent::MessageCreated { message })
}
