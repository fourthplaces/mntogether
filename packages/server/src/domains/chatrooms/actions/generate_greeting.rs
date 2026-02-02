//! Generate greeting action - generates AI greeting and creates assistant message

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::{error, info};

use crate::common::{ContainerId, MemberId};
use crate::domains::chatrooms::events::ChatEvent;
use crate::domains::chatrooms::models::{Container, Message};
use crate::domains::posts::effects::ServerDeps;

/// Generate an AI greeting for a new container with an agent.
///
/// This action:
/// 1. Generates a greeting message using AI
/// 2. Creates the assistant message
///
/// Returns:
/// - `MessageCreated` on success (the greeting message)
/// - `GreetingGenerationFailed` on failure
pub async fn generate_greeting(
    container_id: ContainerId,
    agent_config: String,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ChatEvent> {
    info!(container_id = %container_id, agent_config = %agent_config, "Generating agent greeting");

    // Get greeting prompt based on agent config
    let greeting_prompt = get_greeting_prompt(&agent_config);

    // Generate greeting using AI service
    let greeting_text = match ctx.deps().ai.complete(greeting_prompt).await {
        Ok(text) => text,
        Err(e) => {
            error!("Failed to generate AI greeting: {}", e);
            return Ok(ChatEvent::GreetingGenerationFailed {
                container_id,
                reason: format!("AI generation failed: {}", e),
            });
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
    let sequence_number = Message::next_sequence_number(container_id, &ctx.deps().db_pool).await?;

    let message = Message::create(
        container_id,
        "assistant".to_string(),
        greeting_text.clone(),
        Some(agent_member_id),
        Some("approved".to_string()),
        None, // No parent for greeting
        sequence_number,
        &ctx.deps().db_pool,
    )
    .await?;

    // Update container activity
    Container::touch_activity(container_id, &ctx.deps().db_pool).await?;

    info!(
        message_id = %message.id,
        "Greeting message created"
    );

    Ok(ChatEvent::MessageCreated {
        message_id: message.id,
        container_id,
        role: "assistant".to_string(),
        content: greeting_text,
        author_id: Some(agent_member_id),
    })
}

/// Get greeting prompt based on agent config.
fn get_greeting_prompt(agent_config: &str) -> &'static str {
    match agent_config {
        "admin" => r#"You are an admin assistant for MN Together. Generate a brief, friendly greeting (1-2 sentences) welcoming the admin.

Mention that you can help with:
- Managing websites and listings
- Running scrapers
- Generating assessments
- Answering questions about the data

Keep it concise and professional. Do not use asterisks for formatting.

Your greeting:"#,
        _ => r#"You are a helpful assistant for MN Together. Generate a brief, friendly greeting (1-2 sentences) welcoming the user.

Mention that you can help them find resources and services in their community.

Keep it concise and welcoming. Do not use asterisks for formatting.

Your greeting:"#,
    }
}
