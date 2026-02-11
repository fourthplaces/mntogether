//! AI response generation activities for agents domain.
//!
//! Contains greeting generation, reply generation, and prompt building helpers.
//!
//! Agent identity comes from the `agents` table; preamble from `agent_assistant_configs`.

use anyhow::Result;
use tracing::{error, info};

use crate::common::{ContainerId, MessageId};
use crate::domains::agents::models::{Agent, AgentAssistantConfig};
use crate::domains::chatrooms::models::{Container, Message};
use crate::domains::tag::Tag;
use ai_client::{Agent as AiAgent, Message as AiMessage, PromptBuilder};
use crate::kernel::{SearchPostsTool, ServerDeps};

// ============================================================================
// Action: Generate Greeting
// ============================================================================

/// Generate a greeting message for a new container with an agent.
///
/// Creates an assistant message with the greeting. Returns the created message.
pub async fn generate_greeting(
    container_id: ContainerId,
    agent_config: &str,
    deps: &ServerDeps,
) -> Result<Message> {
    info!(container_id = %container_id, agent_config = %agent_config, "Generating agent greeting");

    let (agent, config) = get_or_create_assistant(agent_config, &deps.db_pool).await?;

    let greeting_prompt = build_greeting_prompt(&config.preamble);

    let greeting_text = deps.ai.complete(&greeting_prompt).await.map_err(|e| {
        error!("Failed to generate AI greeting: {}", e);
        anyhow::anyhow!("AI greeting generation failed: {}", e)
    })?;

    info!(
        container_id = %container_id,
        greeting_length = greeting_text.len(),
        "Agent greeting generated, creating message"
    );

    let sequence_number = Message::next_sequence_number(container_id, &deps.db_pool).await?;

    let message = Message::create(
        container_id,
        "assistant".to_string(),
        greeting_text,
        Some(agent.member_id()),
        Some("approved".to_string()),
        None,
        sequence_number,
        &deps.db_pool,
    )
    .await?;

    Container::touch_activity(container_id, &deps.db_pool).await?;

    info!(message_id = %message.id, "Greeting message created");

    Ok(message)
}

// ============================================================================
// Action: Generate Reply (blocking, non-streaming)
// ============================================================================

/// Generate a reply to a user message in a container with an agent.
///
/// Uses the agent tool-calling loop so the model can search posts if needed.
/// Creates an assistant reply message. Returns the created message.
pub async fn generate_reply(
    message_id: MessageId,
    container_id: ContainerId,
    deps: &ServerDeps,
) -> Result<Message> {
    info!(message_id = %message_id, container_id = %container_id, "Generating agent reply");

    let original_message = Message::find_by_id(message_id, &deps.db_pool)
        .await
        .map_err(|e| {
            error!("Failed to load message: {}", e);
            anyhow::anyhow!("Failed to load message: {}", e)
        })?;

    // Skip if the author is already an assistant (prevent loops)
    if original_message.role == "assistant" {
        info!("Skipping reply - original message is from assistant");
        return Err(anyhow::anyhow!(
            "Skipping reply - original message is from assistant"
        ));
    }

    let agent_config = get_container_agent_config(container_id, &deps.db_pool)
        .await
        .unwrap_or_else(|| "admin".to_string());
    let (agent, config) = get_or_create_assistant(&agent_config, &deps.db_pool).await?;

    let messages = Message::find_by_container(container_id, &deps.db_pool)
        .await
        .map_err(|e| {
            error!("Failed to load conversation: {}", e);
            anyhow::anyhow!("Failed to load conversation: {}", e)
        })?;

    let ai_messages = build_ai_messages(&messages);

    let ai_agent = (*deps.ai).clone().tool(SearchPostsTool::new(
        deps.db_pool.clone(),
        deps.embedding_service.clone(),
    ));

    let reply_text = ai_agent
        .prompt("")
        .preamble(&config.preamble)
        .messages(ai_messages)
        .multi_turn(3)
        .send()
        .await
        .map_err(|e| {
            error!("Agent reply failed: {}", e);
            anyhow::anyhow!("AI reply generation failed: {}", e)
        })?;

    info!(
        message_id = %message_id,
        reply_length = reply_text.len(),
        "Agent reply generated, creating message"
    );

    let sequence_number = Message::next_sequence_number(container_id, &deps.db_pool).await?;

    let new_message = Message::create(
        container_id,
        "assistant".to_string(),
        reply_text,
        Some(agent.member_id()),
        Some("approved".to_string()),
        Some(message_id),
        sequence_number,
        &deps.db_pool,
    )
    .await?;

    Container::touch_activity(container_id, &deps.db_pool).await?;

    info!(new_message_id = %new_message.id, "Assistant message created");

    Ok(new_message)
}

// ============================================================================
// Helper: Check if container has agent
// ============================================================================

/// Check if a container has an agent enabled and return the agent config.
pub async fn get_container_agent_config(
    container_id: ContainerId,
    pool: &sqlx::PgPool,
) -> Option<String> {
    let tags: Vec<Tag> = Tag::find_for_container(container_id, pool).await.ok()?;
    tags.into_iter()
        .find(|t| t.kind == "with_agent")
        .map(|t| t.value)
}

// ============================================================================
// Helper: Get or create assistant agent by config name
// ============================================================================

use crate::domains::agents::models::{ADMIN_AGENT_PREAMBLE, PUBLIC_AGENT_PREAMBLE};

/// Look up an assistant agent by config_name, creating it if it doesn't exist.
async fn get_or_create_assistant(
    config_name: &str,
    pool: &sqlx::PgPool,
) -> Result<(Agent, AgentAssistantConfig)> {
    if let Some(config) = AgentAssistantConfig::find_by_config_name(config_name, pool).await? {
        let agent = Agent::find_by_id(config.agent_id, pool).await?;
        return Ok((agent, config));
    }

    let display_name = match config_name {
        "admin" => "Admin Assistant",
        "public" => "MN Together Guide",
        _ => config_name,
    };
    let preamble = match config_name {
        "admin" => ADMIN_AGENT_PREAMBLE,
        "public" => PUBLIC_AGENT_PREAMBLE,
        _ => "You are a helpful assistant.",
    };

    let agent = Agent::create(display_name, "assistant", pool).await?;
    Agent::set_status(agent.id, "active", pool).await?;
    let config = AgentAssistantConfig::create(agent.id, preamble, config_name, pool).await?;

    Ok((agent, config))
}

// ============================================================================
// Private Helpers
// ============================================================================

fn build_greeting_prompt(preamble: &str) -> String {
    format!(
        r#"{preamble}

Generate a brief, friendly greeting (1-2 sentences) welcoming the user.
Mention what you can help with based on your capabilities above.
Keep it concise and professional. Do not use asterisks for formatting.

Your greeting:"#
    )
}

/// Build conversation history as ai_client::Message types.
fn build_ai_messages(messages: &[Message]) -> Vec<AiMessage> {
    messages
        .iter()
        .filter_map(|msg| match msg.role.as_str() {
            "user" => Some(AiMessage::user(&msg.content)),
            "assistant" => Some(AiMessage::assistant(&msg.content)),
            _ => None,
        })
        .collect()
}
