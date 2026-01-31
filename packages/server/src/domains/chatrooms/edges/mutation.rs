//! GraphQL mutations for the chatrooms domain.

use juniper::{FieldError, FieldResult};
use tracing::info;

use crate::common::ContainerId;
use crate::domains::chatrooms::data::{ContainerData, MessageData};
use crate::domains::chatrooms::events::ChatEvent;
use crate::domains::chatrooms::models::{Container, Message};
use crate::server::graphql::context::GraphQLContext;
use seesaw_core::dispatch_request;

/// Create a new AI chat container
pub async fn create_chat(
    ctx: &GraphQLContext,
    language: Option<String>,
    with_agent: Option<String>,
) -> FieldResult<ContainerData> {
    let member_id = ctx.auth_user.as_ref().map(|u| u.member_id);

    info!(?member_id, ?with_agent, "Creating new AI chat container");

    // Dispatch through seesaw for proper event flow
    let container_id = dispatch_request(
        ChatEvent::CreateContainerRequested {
            container_type: "ai_chat".to_string(),
            entity_id: None,
            language: language.unwrap_or_else(|| "en".to_string()),
            requested_by: member_id,
            with_agent,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &ChatEvent| match e {
                ChatEvent::ContainerCreated { container_id, .. } => Some(Ok(*container_id)),
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| FieldError::new(format!("Failed to create chat: {}", e), juniper::Value::null()))?;

    let container = Container::find_by_id(container_id, &ctx.db_pool).await?;
    Ok(ContainerData::from(container))
}

/// Send a message to a chat container
///
/// This triggers the agent reply flow:
/// 1. Message is created with role "user"
/// 2. AgentReplyMachine schedules GenerateChatReplyCommand
/// 3. GenerateChatReplyEffect generates AI response
/// 4. AgentMessagingMachine creates assistant message
pub async fn send_message(
    ctx: &GraphQLContext,
    container_id: String,
    content: String,
) -> FieldResult<MessageData> {
    let author_id = ctx.auth_user.as_ref().map(|u| u.member_id);
    let container_id = ContainerId::parse(&container_id)?;

    info!(%container_id, ?author_id, "Sending chat message");

    // Dispatch through seesaw for proper event flow
    let message_id = dispatch_request(
        ChatEvent::SendMessageRequested {
            container_id,
            content,
            author_id,
            parent_message_id: None,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &ChatEvent| match e {
                ChatEvent::MessageCreated { message_id, .. } => Some(Ok(*message_id)),
                ChatEvent::MessageFailed { reason, .. } => {
                    Some(Err(anyhow::anyhow!("Message failed: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| FieldError::new(format!("Failed to send message: {}", e), juniper::Value::null()))?;

    let message = Message::find_by_id(message_id, &ctx.db_pool).await?;
    Ok(MessageData::from(message))
}

/// Signal that the user is typing (for real-time indicators)
pub async fn signal_typing(ctx: &GraphQLContext, container_id: String) -> FieldResult<bool> {
    let member_id = ctx
        .auth_user
        .as_ref()
        .ok_or_else(|| {
            FieldError::new("Authentication required", juniper::Value::null())
        })?
        .member_id;

    let container_id = ContainerId::parse(&container_id)?;

    // Emit typing event directly to bus (ephemeral, not persisted)
    ctx.bus.emit(crate::domains::chatrooms::events::TypingEvent::Started {
        container_id,
        member_id,
    });

    Ok(true)
}
