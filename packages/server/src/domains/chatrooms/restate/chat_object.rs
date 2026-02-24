//! Chat virtual object
//!
//! Keyed by container_id. Serialized writes prevent message ordering issues.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::common::auth::restate_auth::optional_auth;
use crate::common::EmptyRequest;
use crate::common::ContainerId;
use crate::domains::agents::activities as agent_activities;
use crate::domains::chatrooms::activities as chatroom_activities;
use crate::domains::chatrooms::models::{Container, Message};
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChatRequest {
    pub language: Option<String>,
    pub with_agent: Option<String>,
}

impl_restate_serde!(CreateChatRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
}

impl_restate_serde!(SendMessageRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateReplyRequest {
    pub message_id: Uuid,
}

impl_restate_serde!(GenerateReplyRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedRequest {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl_restate_serde!(PaginatedRequest);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResult {
    pub id: Uuid,
    pub language: String,
    pub created_at: String,
}

impl_restate_serde!(ChatResult);

impl From<Container> for ChatResult {
    fn from(c: Container) -> Self {
        Self {
            id: c.id.into_uuid(),
            language: c.language,
            created_at: c.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResult {
    pub id: Uuid,
    pub container_id: Uuid,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

impl_restate_serde!(MessageResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageListResult {
    pub messages: Vec<MessageResult>,
}

impl_restate_serde!(MessageListResult);

impl From<Message> for MessageResult {
    fn from(m: Message) -> Self {
        Self {
            id: m.id.into_uuid(),
            container_id: m.container_id.into_uuid(),
            role: m.role,
            content: m.content,
            created_at: m.created_at.to_rfc3339(),
        }
    }
}

// =============================================================================
// Virtual object definition
// =============================================================================

#[restate_sdk::object]
#[name = "Chat"]
pub trait ChatObject {
    async fn create(req: CreateChatRequest) -> Result<ChatResult, HandlerError>;
    async fn send_message(req: SendMessageRequest) -> Result<MessageResult, HandlerError>;
    async fn generate_reply(req: GenerateReplyRequest) -> Result<MessageResult, HandlerError>;

    #[shared]
    async fn get(req: EmptyRequest) -> Result<ChatResult, HandlerError>;

    #[shared]
    async fn get_messages(req: PaginatedRequest) -> Result<MessageListResult, HandlerError>;
}

pub struct ChatObjectImpl {
    deps: Arc<ServerDeps>,
}

impl ChatObjectImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl ChatObject for ChatObjectImpl {
    async fn create(
        &self,
        ctx: ObjectContext<'_>,
        req: CreateChatRequest,
    ) -> Result<ChatResult, HandlerError> {
        let user = optional_auth(ctx.headers(), &self.deps.jwt_service);
        let with_agent = req.with_agent.clone();

        let (container, _greeting) = chatroom_activities::create_container(
            req.language.unwrap_or_else(|| "en".to_string()),
            user.as_ref().map(|u| u.member_id),
            with_agent.clone(),
            &self.deps,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        // Generate agent greeting if agent is configured
        if let Some(agent_config) = with_agent {
            let container_id = container.id;
            let deps = self.deps.clone();
            ctx.run(|| async {
                match agent_activities::generate_greeting(container_id, &agent_config, &deps).await
                {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to generate agent greeting");
                        Ok(())
                    }
                }
            })
            .await?;
        }

        Ok(ChatResult::from(container))
    }

    async fn send_message(
        &self,
        ctx: ObjectContext<'_>,
        req: SendMessageRequest,
    ) -> Result<MessageResult, HandlerError> {
        let user = optional_auth(ctx.headers(), &self.deps.jwt_service);
        let container_id = Uuid::parse_str(ctx.key())
            .map_err(|e| TerminalError::new(format!("Invalid container ID: {}", e)))?;
        let cid = ContainerId::from_uuid(container_id);

        let message = ctx
            .run(|| async {
                chatroom_activities::send_message(
                    cid,
                    req.content.clone(),
                    user.as_ref().map(|u| u.member_id),
                    None, // parent_message_id
                    &self.deps,
                )
                .await
                .map_err(Into::into)
            })
            .await?;

        // Check if container has an agent configured
        let has_agent = ctx
            .run(|| async {
                let result = agent_activities::get_container_agent_config(cid, &self.deps.db_pool)
                    .await
                    .is_some();
                Ok(result)
            })
            .await?;

        // Fire-and-forget: trigger reply generation as a separate durable invocation
        if has_agent {
            ctx.object_client::<ChatObjectClient>(ctx.key())
                .generate_reply(GenerateReplyRequest {
                    message_id: message.id.into_uuid(),
                })
                .send();
        }

        Ok(MessageResult::from(message))
    }

    async fn generate_reply(
        &self,
        ctx: ObjectContext<'_>,
        req: GenerateReplyRequest,
    ) -> Result<MessageResult, HandlerError> {
        let container_id = Uuid::parse_str(ctx.key())
            .map_err(|e| TerminalError::new(format!("Invalid container ID: {}", e)))?;
        let cid = ContainerId::from_uuid(container_id);
        let message_id = crate::common::MessageId::from_uuid(req.message_id);

        // Journaled: load context + call LLM + save reply
        let new_message = ctx
            .run(|| async {
                agent_activities::generate_reply(message_id, cid, &self.deps)
                    .await
                    .map_err(Into::into)
            })
            .await?;

        // Ephemeral: notify subscribers via SSE
        let topic = crate::domains::agents::ChatStreamEvent::topic(&container_id.to_string());
        if let Ok(event) =
            serde_json::to_value(crate::domains::agents::ChatStreamEvent::MessageComplete {
                container_id: container_id.to_string(),
                message_id: new_message.id.to_string(),
                content: new_message.content.clone(),
                role: "assistant".to_string(),
                created_at: new_message.created_at.to_rfc3339(),
            })
        {
            self.deps.stream_hub.publish(&topic, event).await;
        }

        Ok(MessageResult::from(new_message))
    }

    async fn get(
        &self,
        ctx: SharedObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<ChatResult, HandlerError> {
        let container_id = Uuid::parse_str(ctx.key())
            .map_err(|e| TerminalError::new(format!("Invalid container ID: {}", e)))?;

        let container =
            Container::find_by_id(ContainerId::from_uuid(container_id), &self.deps.db_pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(ChatResult::from(container))
    }

    async fn get_messages(
        &self,
        ctx: SharedObjectContext<'_>,
        _req: PaginatedRequest,
    ) -> Result<MessageListResult, HandlerError> {
        let container_id = Uuid::parse_str(ctx.key())
            .map_err(|e| TerminalError::new(format!("Invalid container ID: {}", e)))?;

        let messages =
            Message::find_by_container(ContainerId::from_uuid(container_id), &self.deps.db_pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(MessageListResult {
            messages: messages.into_iter().map(MessageResult::from).collect(),
        })
    }
}
