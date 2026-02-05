//! Cascade event handlers for chatrooms domain
//!
//! These handlers respond to FACT events in the multi-step workflow.
//! They are called by the effect dispatcher when cascading is needed.
//!
//! Cascade flow:
//!   ContainerCreated (with_agent) → handle_generate_greeting → MessageCreated
//!   MessageCreated (user role) → handle_generate_reply → MessageCreated

use anyhow::Result;
use tracing::{error, info};

use crate::common::{ContainerId, MemberId, MessageId};
use crate::domains::chatrooms::models::{Container, Message};
use crate::domains::tag::Tag;
use crate::kernel::{CompletionExt, ServerDeps};

// ============================================================================
// Handler: Generate Greeting (cascade from ContainerCreated with agent)
// ============================================================================

/// Handle generate greeting for new container with agent.
///
/// Creates an assistant message with the greeting. Returns the created message.
/// The effect handles any further cascading internally.
pub async fn handle_generate_greeting(
    container_id: ContainerId,
    agent_config: String,
    deps: &ServerDeps,
) -> Result<Message> {
    info!(container_id = %container_id, agent_config = %agent_config, "Generating agent greeting");

    // Get greeting prompt based on agent config
    let greeting_prompt = get_greeting_prompt(&agent_config);

    // Generate greeting using AI service
    let greeting_text = match deps.ai.complete(greeting_prompt).await {
        Ok(text) => text,
        Err(e) => {
            error!("Failed to generate AI greeting: {}", e);
            return Err(anyhow::anyhow!("AI greeting generation failed: {}", e));
        }
    };

    info!(
        container_id = %container_id,
        greeting_length = greeting_text.len(),
        "Agent greeting generated, creating message"
    );

    // For now, use a placeholder agent member ID
    let agent_member_id = MemberId::new();

    // Create the greeting message directly
    let sequence_number = Message::next_sequence_number(container_id, &deps.db_pool).await?;

    let message = Message::create(
        container_id,
        "assistant".to_string(),
        greeting_text,
        Some(agent_member_id),
        Some("approved".to_string()),
        None, // No parent for greeting
        sequence_number,
        &deps.db_pool,
    )
    .await?;

    // Update container activity
    Container::touch_activity(container_id, &deps.db_pool).await?;

    info!(
        message_id = %message.id,
        "Greeting message created"
    );

    Ok(message)
}

// ============================================================================
// Handler: Generate Reply (cascade from MessageCreated when user message)
// ============================================================================

/// Handle generate reply for user message in container with agent.
///
/// Creates an assistant reply message. Returns the created message.
/// The effect handles any further cascading internally.
pub async fn handle_generate_reply(
    message_id: MessageId,
    container_id: ContainerId,
    deps: &ServerDeps,
) -> Result<Message> {
    info!(message_id = %message_id, container_id = %container_id, "Generating agent reply");

    // Load the original message
    let original_message = match Message::find_by_id(message_id, &deps.db_pool).await {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to load message: {}", e);
            return Err(anyhow::anyhow!("Failed to load message: {}", e));
        }
    };

    // Skip if the author is already an assistant (prevent loops)
    if original_message.role == "assistant" {
        info!("Skipping reply - original message is from assistant");
        return Err(anyhow::anyhow!("Skipping reply - original message is from assistant"));
    }

    // Load conversation context (all messages in container)
    let messages = match Message::find_by_container(container_id, &deps.db_pool).await {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to load conversation: {}", e);
            return Err(anyhow::anyhow!("Failed to load conversation: {}", e));
        }
    };

    let conversation = build_conversation_messages(&messages);

    // Get system prompt based on auth context
    let system_prompt = get_system_prompt(false); // Default to non-admin for now

    // Build full prompt with system context and conversation
    let full_prompt = build_chat_prompt(system_prompt, &conversation);

    // Generate reply using AI service
    let reply_text = match deps.ai.complete(&full_prompt).await {
        Ok(text) => text,
        Err(e) => {
            error!("Failed to generate AI reply: {}", e);
            return Err(anyhow::anyhow!("AI reply generation failed: {}", e));
        }
    };

    info!(
        message_id = %message_id,
        reply_length = reply_text.len(),
        "Agent reply generated, creating message"
    );

    // For now, use a placeholder agent member ID
    let agent_member_id = MemberId::new();

    // Create the assistant message directly
    let sequence_number = Message::next_sequence_number(container_id, &deps.db_pool).await?;

    let new_message = Message::create(
        container_id,
        "assistant".to_string(),
        reply_text,
        Some(agent_member_id),
        Some("approved".to_string()),
        Some(message_id), // Parent is the message we're replying to
        sequence_number,
        &deps.db_pool,
    )
    .await?;

    // Update container activity
    Container::touch_activity(container_id, &deps.db_pool).await?;

    info!(
        new_message_id = %new_message.id,
        "Assistant message created"
    );

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
    // Look for with_agent tag on this container
    let tags: Vec<Tag> = Tag::find_for_container(container_id, pool).await.ok()?;
    tags.into_iter()
        .find(|t| t.kind == "with_agent")
        .map(|t| t.value)
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Build conversation context for AI from message history.
fn build_conversation_messages(messages: &[Message]) -> String {
    messages
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Build full chat prompt with system context and conversation.
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

/// Get system prompt based on auth context.
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

/// Get greeting prompt based on agent config.
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
