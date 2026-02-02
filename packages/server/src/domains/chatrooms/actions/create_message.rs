//! Create message action - creates a message in a container

use anyhow::Result;
use sqlx::PgPool;
use tracing::info;

use crate::common::{ContainerId, MemberId, MessageId};
use crate::domains::chatrooms::events::ChatEvent;
use crate::domains::chatrooms::models::{Container, Message};

/// Create a message in a container.
///
/// Returns (Message, ChatEvent::MessageCreated).
pub async fn create_message(
    container_id: ContainerId,
    role: String,
    content: String,
    author_id: Option<MemberId>,
    parent_message_id: Option<MessageId>,
    pool: &PgPool,
) -> Result<(Message, ChatEvent)> {
    info!(container_id = %container_id, role = %role, "Creating message");

    // Get next sequence number
    let sequence_number = Message::next_sequence_number(container_id, pool).await?;

    // Create message
    let message = Message::create(
        container_id,
        role.clone(),
        content.clone(),
        author_id,
        Some("approved".to_string()), // AI chat messages auto-approved
        parent_message_id,
        sequence_number,
        pool,
    )
    .await?;

    // Update container activity
    Container::touch_activity(container_id, pool).await?;

    let event = ChatEvent::MessageCreated {
        message: message.clone(),
    };

    Ok((message, event))
}
