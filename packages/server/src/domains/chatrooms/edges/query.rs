//! GraphQL queries and mutations for the chatrooms domain.

use juniper::{FieldError, FieldResult};

use crate::common::ContainerId;
use crate::domains::chatrooms::data::{ContainerData, MessageData};
use crate::domains::chatrooms::edges::{CreateChat, SendMessage};
use crate::domains::chatrooms::models::{Container, Message};
use crate::domains::chatrooms::state::ChatRequestState;
use crate::server::graphql::context::GraphQLContext;

// =============================================================================
// Queries
// =============================================================================

/// Get a container by ID
pub async fn get_container(ctx: &GraphQLContext, id: String) -> FieldResult<Option<ContainerData>> {
    let container_id = ContainerId::parse(&id)?;

    match Container::find_by_id(container_id, &ctx.db_pool).await {
        Ok(container) => Ok(Some(ContainerData::from(container))),
        Err(_) => Ok(None),
    }
}

/// Get messages for a container
pub async fn get_messages(
    ctx: &GraphQLContext,
    container_id: String,
) -> FieldResult<Vec<MessageData>> {
    let container_id = ContainerId::parse(&container_id)?;

    let messages = Message::find_by_container(container_id, &ctx.db_pool).await?;

    Ok(messages.into_iter().map(MessageData::from).collect())
}

/// Get recent chat containers for the current user
pub async fn get_recent_chats(
    ctx: &GraphQLContext,
    limit: Option<i32>,
) -> FieldResult<Vec<ContainerData>> {
    let limit = limit.unwrap_or(20) as i64;

    let containers = Container::find_recent_by_type("ai_chat", limit, &ctx.db_pool).await?;

    Ok(containers.into_iter().map(ContainerData::from).collect())
}

// =============================================================================
// Mutations (via engine.run)
// =============================================================================

/// Create a new AI chat container
pub async fn create_chat(
    ctx: &GraphQLContext,
    language: Option<String>,
    with_agent: Option<String>,
) -> FieldResult<ContainerData> {
    let member_id = ctx.auth_user.as_ref().map(|u| u.member_id);

    let edge = CreateChat {
        language: language.unwrap_or_else(|| "en".to_string()),
        with_agent,
        requested_by: member_id,
    };

    let mut engine = ctx.engine.lock().await;
    let container = engine
        .run(edge, ChatRequestState::default())
        .await
        .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))?
        .ok_or_else(|| FieldError::new("Failed to create chat", juniper::Value::null()))?;

    Ok(container)
}

/// Send a message to a chat container
pub async fn send_message(
    ctx: &GraphQLContext,
    container_id: String,
    content: String,
) -> FieldResult<MessageData> {
    let member_id = ctx.auth_user.as_ref().map(|u| u.member_id);
    let container_id = ContainerId::parse(&container_id)?;

    let edge = SendMessage {
        container_id,
        content,
        author_id: member_id,
    };

    let mut engine = ctx.engine.lock().await;
    let message = engine
        .run(edge, ChatRequestState::default())
        .await
        .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))?
        .ok_or_else(|| FieldError::new("Failed to send message", juniper::Value::null()))?;

    Ok(message)
}
