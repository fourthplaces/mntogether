//! AI response generation actions for chatrooms
//!
//! These actions handle AI-generated greetings and replies.
//! Business logic moved from effects/handlers.rs to follow
//! the "actions contain business logic" pattern.

use anyhow::Result;
use tracing::{error, info};

use crate::common::{ContainerId, MemberId, MessageId};
use crate::domains::chatrooms::models::{Container, Message};
use crate::domains::tag::Tag;
use crate::kernel::{CompletionExt, ServerDeps};

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

    let greeting_prompt = get_greeting_prompt(agent_config);

    let greeting_text = deps.ai.complete(greeting_prompt).await.map_err(|e| {
        error!("Failed to generate AI greeting: {}", e);
        anyhow::anyhow!("AI greeting generation failed: {}", e)
    })?;

    info!(
        container_id = %container_id,
        greeting_length = greeting_text.len(),
        "Agent greeting generated, creating message"
    );

    let agent_member_id = MemberId::new();
    let sequence_number = Message::next_sequence_number(container_id, &deps.db_pool).await?;

    let message = Message::create(
        container_id,
        "assistant".to_string(),
        greeting_text,
        Some(agent_member_id),
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
// Action: Generate Reply
// ============================================================================

/// Generate a reply to a user message in a container with an agent.
///
/// Creates an assistant reply message. Returns the created message.
pub async fn generate_reply(
    message_id: MessageId,
    container_id: ContainerId,
    deps: &ServerDeps,
) -> Result<Message> {
    info!(message_id = %message_id, container_id = %container_id, "Generating agent reply");

    let original_message = Message::find_by_id(message_id, &deps.db_pool).await.map_err(|e| {
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

    let messages = Message::find_by_container(container_id, &deps.db_pool)
        .await
        .map_err(|e| {
            error!("Failed to load conversation: {}", e);
            anyhow::anyhow!("Failed to load conversation: {}", e)
        })?;

    let conversation = build_conversation_messages(&messages);
    let system_prompt = get_system_prompt(false);
    let full_prompt = build_chat_prompt(system_prompt, &conversation);

    let reply_text = deps.ai.complete(&full_prompt).await.map_err(|e| {
        error!("Failed to generate AI reply: {}", e);
        anyhow::anyhow!("AI reply generation failed: {}", e)
    })?;

    info!(
        message_id = %message_id,
        reply_length = reply_text.len(),
        "Agent reply generated, creating message"
    );

    let agent_member_id = MemberId::new();
    let sequence_number = Message::next_sequence_number(container_id, &deps.db_pool).await?;

    let new_message = Message::create(
        container_id,
        "assistant".to_string(),
        reply_text,
        Some(agent_member_id),
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
// Private Helpers
// ============================================================================

fn build_conversation_messages(messages: &[Message]) -> String {
    messages
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn build_chat_prompt(system_prompt: &str, conversation: &str) -> String {
    format!(
        r#"{system_prompt}

## Conversation History

{conversation}

## Instructions

Respond to the user's most recent message. Be helpful, concise, and friendly.
Do not include any role prefixes like "assistant:" in your response.

Your response:"#
    )
}

fn get_system_prompt(is_admin: bool) -> &'static str {
    if is_admin {
        r#"You are an admin assistant for MN Together, a resource-sharing platform.
You can help administrators:
- Approve or reject listings
- Scrape websites for new resources
- Generate website assessments
- Search and filter listings
- Manage organizations

Be helpful and proactive. If an admin asks to do something, use the appropriate tool."#
    } else {
        r#"You are a helpful assistant for MN Together, a resource-sharing platform.
You can help users find resources and services in their community.
You have access to publicly available listings and can search for relevant information.
Be friendly and helpful in your responses."#
    }
}

fn get_greeting_prompt(agent_config: &str) -> &'static str {
    match agent_config {
        "admin" => {
            r#"You are an admin assistant for MN Together. Generate a brief, friendly greeting (1-2 sentences) welcoming the admin.

Mention that you can help with:
- Managing websites and listings
- Running scrapers
- Generating assessments
- Answering questions about the data

Keep it concise and professional. Do not use asterisks for formatting.

Your greeting:"#
        }
        _ => {
            r#"You are a helpful assistant for MN Together. Generate a brief, friendly greeting (1-2 sentences) welcoming the user.

Mention that you can help them find resources and services in their community.

Keep it concise and welcoming. Do not use asterisks for formatting.

Your greeting:"#
        }
    }
}
