//! Agent messaging effect - generates AI replies.
//!
//! This effect implements the pure effect pattern:
//! - Effect generates text → returns ChatMessagingEvent (fact about what was generated)
//! - Machine observes fact → emits ChatCommand::CreateMessage (orchestration)
//! - Chat effect creates message → returns ChatEvent::MessageCreated
//!
//! This maintains causality: every transition is visible in the event stream.

use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};
use tracing::info;

use crate::common::{ContainerId, MemberId, MessageId};
use crate::domains::chatrooms::commands::{GenerateAgentGreetingCommand, GenerateChatReplyCommand};
use crate::domains::chatrooms::events::ChatMessagingEvent;
use crate::domains::chatrooms::models::Message;
use crate::kernel::ServerDeps;

/// Effect that generates agent replies.
///
/// Returns ChatMessagingEvent::ReplyGenerated (a fact about AI output).
/// The AgentMessagingMachine observes this and emits ChatCommand::CreateMessage.
pub struct GenerateChatReplyEffect;

#[async_trait]
impl Effect<GenerateChatReplyCommand, ServerDeps> for GenerateChatReplyEffect {
    type Event = ChatMessagingEvent;

    async fn execute(
        &self,
        cmd: GenerateChatReplyCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<ChatMessagingEvent> {
        handle_generate_reply(cmd, &ctx).await
    }
}

// =============================================================================
// Handler Functions (Business Logic)
// =============================================================================

async fn handle_generate_reply(
    cmd: GenerateChatReplyCommand,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ChatMessagingEvent> {
    let message_id = MessageId::from_uuid(cmd.message_id);
    let container_id = ContainerId::from_uuid(cmd.container_id);

    info!(message_id = %message_id, container_id = %container_id, "Generating agent reply");

    // Load the original message
    let original_message = Message::find_by_id(message_id, &ctx.deps().db_pool).await?;

    // Skip if the author is already an agent (prevent loops)
    if original_message.role == "assistant" {
        return Ok(ChatMessagingEvent::Skipped {
            reason: "Author is already an assistant",
        });
    }

    // Load conversation context (all messages in container)
    let messages = Message::find_by_container(container_id, &ctx.deps().db_pool).await?;
    let conversation = build_conversation_messages(&messages);

    // Get system prompt based on auth context
    // TODO: Check if author is admin and use different tools
    let system_prompt = get_system_prompt(false); // Default to non-admin for now

    // Build full prompt with system context and conversation
    let full_prompt = build_chat_prompt(system_prompt, &conversation);

    // Generate reply using AI service
    let ai = &ctx.deps().ai;
    let reply_text = ai.complete(&full_prompt).await?;

    // For now, use a placeholder agent member ID
    // TODO: Look up or create agent member for this container
    let agent_member_id = MemberId::new();

    info!(
        message_id = %message_id,
        reply_length = reply_text.len(),
        "Agent reply generated"
    );

    Ok(ChatMessagingEvent::ReplyGenerated {
        container_id,
        response_to_id: message_id,
        author_id: agent_member_id,
        text: reply_text,
    })
}

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
///
/// Admin users get access to admin tools.
/// Public users get read-only access.
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

// =============================================================================
// Agent Greeting Effect
// =============================================================================

/// Effect that generates an agent greeting when a container is created with an agent.
///
/// Returns ChatMessagingEvent::ReplyGenerated (a fact about AI output).
/// The AgentMessagingMachine observes this and emits ChatCommand::CreateMessage.
pub struct GenerateAgentGreetingEffect;

#[async_trait]
impl Effect<GenerateAgentGreetingCommand, ServerDeps> for GenerateAgentGreetingEffect {
    type Event = ChatMessagingEvent;

    async fn execute(
        &self,
        cmd: GenerateAgentGreetingCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<ChatMessagingEvent> {
        handle_generate_greeting(cmd, &ctx).await
    }
}

async fn handle_generate_greeting(
    cmd: GenerateAgentGreetingCommand,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ChatMessagingEvent> {
    let container_id = ContainerId::from_uuid(cmd.container_id);
    let agent_config = &cmd.agent_config;

    info!(container_id = %container_id, agent_config = %agent_config, "Generating agent greeting");

    // Get greeting prompt based on agent config
    let greeting_prompt = get_greeting_prompt(agent_config);

    // Generate greeting using AI service
    let ai = &ctx.deps().ai;
    let greeting_text = ai.complete(greeting_prompt).await?;

    // Create a placeholder message ID for the greeting (since there's no user message)
    let placeholder_id = MessageId::new();

    // For now, use a placeholder agent member ID
    let agent_member_id = MemberId::new();

    info!(
        container_id = %container_id,
        greeting_length = greeting_text.len(),
        "Agent greeting generated"
    );

    Ok(ChatMessagingEvent::ReplyGenerated {
        container_id,
        response_to_id: placeholder_id,
        author_id: agent_member_id,
        text: greeting_text,
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
