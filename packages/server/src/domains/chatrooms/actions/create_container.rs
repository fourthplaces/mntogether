//! Create container action - creates a new chat container

use anyhow::Result;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::common::MemberId;
use crate::domains::chatrooms::events::ChatEvent;
use crate::domains::chatrooms::models::Container;
use crate::domains::tag::{Tag, Taggable};

/// Create a new chat container.
///
/// Returns (Container, ChatEvent::ContainerCreated).
pub async fn create_container(
    container_type: String,
    entity_id: Option<Uuid>,
    language: String,
    _requested_by: Option<MemberId>,
    with_agent: Option<String>,
    pool: &PgPool,
) -> Result<(Container, ChatEvent)> {
    info!(container_type = %container_type, ?with_agent, "Creating chat container");

    let container = Container::create(
        container_type.clone(),
        entity_id,
        language,
        pool,
    )
    .await?;

    // Tag container with agent config if provided
    if let Some(ref agent_config) = with_agent {
        info!(container_id = %container.id, agent_config = %agent_config, "Tagging container with agent");
        let tag = Tag::find_or_create("with_agent", agent_config, None, pool).await?;
        Taggable::create_container_tag(container.id, tag.id, pool).await?;
    }

    let event = ChatEvent::ContainerCreated {
        container: container.clone(),
        with_agent,
    };

    Ok((container, event))
}
