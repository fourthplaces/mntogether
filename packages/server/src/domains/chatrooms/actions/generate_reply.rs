//! Generate reply action - generates AI reply and creates assistant message

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::{error, info};

use crate::common::{ContainerId, MemberId, MessageId};
use crate::domains::chatrooms::events::ChatEvent;
use crate::domains::chatrooms::models::{Container, Message};
use crate::domains::chatrooms::ChatRequestState;
use crate::domains::posts::effects::ServerDeps;

/// Generate an AI reply to a message and create the assistant message.
///
/// This action:
/// 1. Loads the original message
/// 2. Skips if author is already assistant (prevents loops)
/// 3. Loads conversation context
/// 4. Generates AI reply
/// 5. Creates the assistant message
///
/// Returns:
/// - `MessageCreated` on success (the new assistant message)
/// - `ReplyGenerationFailed` on failure
pub async fn generate_reply(
    message_id: MessageId,
    container_id: ContainerId,
    ctx: &EffectContext<ServerDeps, ChatRequestState>,
) -> Result<ChatEvent> {
    info!(message_id = %message_id, container_id = %container_id, "Generating agent reply");

    // Load the original message
    let original_message = match Message::find_by_id(message_id, &ctx.deps().db_pool).await {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to load message: {}", e);
            return Ok(ChatEvent::ReplyGenerationFailed {
                message_id,
                container_id,
                reason: format!("Failed to load message: {}", e),
            });
        }
    };

    // Skip if the author is already an assistant (prevent loops)
    if original_message.role == "assistant" {
        info!("Skipping reply - original message is from assistant");
        // Return a "no-op" - we don't want to generate a reply to our own messages
        // This is an edge case that shouldn't normally happen due to internal edge filtering
        return Ok(ChatEvent::ReplyGenerationFailed {
            message_id,
            container_id,
            reason: "Original message is from assistant".to_string(),
        });
    }

    // Load conversation context (all messages in container)
    let messages = match Message::find_by_container(container_id, &ctx.deps().db_pool).await {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to load conversation: {}", e);
            return Ok(ChatEvent::ReplyGenerationFailed {
                message_id,
                container_id,
                reason: format!("Failed to load conversation: {}", e),
            });
        }
    };

    let conversation = build_conversation_messages(&messages);

    // Get system prompt based on auth context
    let system_prompt = get_system_prompt(false); // Default to non-admin for now

    // Build full prompt with system context and conversation
    let full_prompt = build_chat_prompt(system_prompt, &conversation);

    // Generate reply using AI service
    let reply_text = match ctx.deps().ai.complete(&full_prompt).await {
        Ok(text) => text,
        Err(e) => {
            error!("Failed to generate AI reply: {}", e);
            return Ok(ChatEvent::ReplyGenerationFailed {
                message_id,
                container_id,
                reason: format!("AI generation failed: {}", e),
            });
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
    let sequence_number = Message::next_sequence_number(container_id, &ctx.deps().db_pool).await?;

    let new_message = Message::create(
        container_id,
        "assistant".to_string(),
        reply_text.clone(),
        Some(agent_member_id),
        Some("approved".to_string()),
        Some(message_id), // Parent is the message we're replying to
        sequence_number,
        &ctx.deps().db_pool,
    )
    .await?;

    // Update container activity
    Container::touch_activity(container_id, &ctx.deps().db_pool).await?;

    info!(
        new_message_id = %new_message.id,
        "Assistant message created"
    );

    Ok(ChatEvent::MessageCreated { message: new_message })
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
