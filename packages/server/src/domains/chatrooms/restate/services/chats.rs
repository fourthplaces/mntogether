//! Chats service (stateless)
//!
//! Cross-container operations: list recent chats.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::domains::chatrooms::models::Container;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

use crate::domains::chatrooms::restate::virtual_objects::chat::ChatResult;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRecentChatsRequest {
    pub limit: Option<i32>,
}

impl_restate_serde!(ListRecentChatsRequest);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatListResult {
    pub chats: Vec<ChatResult>,
}

impl_restate_serde!(ChatListResult);

// =============================================================================
// Service definition
// =============================================================================

#[restate_sdk::service]
#[name = "Chats"]
pub trait ChatsService {
    async fn list_recent(req: ListRecentChatsRequest) -> Result<ChatListResult, HandlerError>;
}

pub struct ChatsServiceImpl {
    deps: Arc<ServerDeps>,
}

impl ChatsServiceImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl ChatsService for ChatsServiceImpl {
    async fn list_recent(
        &self,
        _ctx: Context<'_>,
        req: ListRecentChatsRequest,
    ) -> Result<ChatListResult, HandlerError> {
        let limit = req.limit.unwrap_or(20) as i64;

        let containers = Container::find_recent(limit, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(ChatListResult {
            chats: containers.into_iter().map(ChatResult::from).collect(),
        })
    }
}
