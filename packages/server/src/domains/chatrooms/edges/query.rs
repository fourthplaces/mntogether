//! GraphQL queries for the chatrooms domain.

use juniper::FieldResult;

use crate::common::ContainerId;
use crate::domains::chatrooms::data::{ContainerData, MessageData};
use crate::domains::chatrooms::models::{Container, Message};
use crate::server::graphql::context::GraphQLContext;

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
