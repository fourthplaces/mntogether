//! AI response generation actions for agents domain.
//!
//! Moved from chatrooms/actions/ai_responses.rs.
//! Contains greeting generation, reply generation (blocking + streaming),
//! and prompt building helpers.
//!
//! Agent identity and preamble come from the `agents` table.

use anyhow::Result;
use futures::StreamExt;
use tracing::{error, info};

use crate::common::{ContainerId, MessageId};
use crate::domains::agents::events::ChatStreamEvent;
use crate::domains::agents::models::Agent;
use crate::domains::chatrooms::models::{Container, Message};
use crate::domains::tag::Tag;
use crate::kernel::{CompletionExt, ServerDeps};
use openai_client::{ChatRequest, Message as OaiMessage};

// ============================================================================
// Action: Generate Greeting
// ============================================================================

/// Generate a greeting message for a new container with an agent.
///
/// Creates an assistant message with the greeting. Returns the created message.
pub async fn generate_greeting(
    container_id: ContainerId,
    _agent_config: &str,
    deps: &ServerDeps,
) -> Result<Message> {
    info!(container_id = %container_id, "Generating agent greeting");

    let agent = Agent::get_or_create_default(&deps.db_pool).await?;

    let greeting_prompt = build_greeting_prompt(&agent.preamble);

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

    let agent = Agent::get_or_create_default(&deps.db_pool).await?;

    let messages = Message::find_by_container(container_id, &deps.db_pool)
        .await
        .map_err(|e| {
            error!("Failed to load conversation: {}", e);
            anyhow::anyhow!("Failed to load conversation: {}", e)
        })?;

    let conversation = build_conversation_messages(&messages);
    let full_prompt = build_chat_prompt(&agent.preamble, &conversation);

    let reply_text = deps.ai.complete(&full_prompt).await.map_err(|e| {
        error!("Failed to generate AI reply: {}", e);
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
// Action: Generate Reply (streaming)
// ============================================================================

/// Generate a streaming reply to a user message.
///
/// Publishes token deltas to StreamHub as they arrive from OpenAI,
/// then saves the complete message to DB.
pub async fn generate_reply_streaming(
    message_id: MessageId,
    container_id: ContainerId,
    deps: &ServerDeps,
) -> Result<Message> {
    info!(message_id = %message_id, container_id = %container_id, "Generating streaming agent reply");

    let original_message = Message::find_by_id(message_id, &deps.db_pool).await?;

    if original_message.role == "assistant" {
        return Err(anyhow::anyhow!(
            "Skipping reply - original message is from assistant"
        ));
    }

    let agent = Agent::get_or_create_default(&deps.db_pool).await?;

    let messages = Message::find_by_container(container_id, &deps.db_pool).await?;
    let topic = ChatStreamEvent::topic(&container_id.to_string());

    // Build proper OpenAI message array
    let request = build_streaming_chat_request(&agent.preamble, &messages);

    // Signal generation started
    deps.stream_hub
        .publish(
            &topic,
            serde_json::to_value(ChatStreamEvent::GenerationStarted {
                container_id: container_id.to_string(),
                in_reply_to: message_id.to_string(),
            })?,
        )
        .await;

    // Stream tokens from OpenAI
    let mut accumulated = String::new();
    let stream_result = deps.ai.chat_completion_stream(request).await;

    let mut stream = match stream_result {
        Ok(s) => s,
        Err(e) => {
            deps.stream_hub
                .publish(
                    &topic,
                    serde_json::to_value(ChatStreamEvent::GenerationError {
                        container_id: container_id.to_string(),
                        error: e.to_string(),
                    })?,
                )
                .await;
            return Err(anyhow::anyhow!("Failed to start streaming: {}", e));
        }
    };

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) if !chunk.delta.is_empty() => {
                accumulated.push_str(&chunk.delta);

                deps.stream_hub
                    .publish(
                        &topic,
                        serde_json::to_value(ChatStreamEvent::TokenDelta {
                            container_id: container_id.to_string(),
                            delta: chunk.delta,
                        })?,
                    )
                    .await;
            }
            Err(e) => {
                error!(error = %e, "Stream chunk error");
                deps.stream_hub
                    .publish(
                        &topic,
                        serde_json::to_value(ChatStreamEvent::GenerationError {
                            container_id: container_id.to_string(),
                            error: e.to_string(),
                        })?,
                    )
                    .await;
                return Err(e.into());
            }
            _ => {} // empty delta or done signal
        }
    }

    // Save complete message to DB
    let sequence_number = Message::next_sequence_number(container_id, &deps.db_pool).await?;

    let new_message = Message::create(
        container_id,
        "assistant".to_string(),
        accumulated.clone(),
        Some(agent.member_id()),
        Some("approved".to_string()),
        Some(message_id),
        sequence_number,
        &deps.db_pool,
    )
    .await?;

    // Signal completion with the persisted message
    deps.stream_hub
        .publish(
            &topic,
            serde_json::to_value(ChatStreamEvent::MessageComplete {
                container_id: container_id.to_string(),
                message_id: new_message.id.to_string(),
                content: accumulated,
                role: "assistant".to_string(),
                created_at: new_message.created_at.to_rfc3339(),
            })?,
        )
        .await;

    Container::touch_activity(container_id, &deps.db_pool).await?;

    info!(new_message_id = %new_message.id, "Streaming assistant message created");

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

fn build_chat_prompt(preamble: &str, conversation: &str) -> String {
    format!(
        r#"{preamble}

## Conversation History

{conversation}

## Instructions

Respond to the user's most recent message. Be helpful, concise, and friendly.
Do not include any role prefixes like "assistant:" in your response.

Your response:"#
    )
}

fn build_greeting_prompt(preamble: &str) -> String {
    format!(
        r#"{preamble}

Generate a brief, friendly greeting (1-2 sentences) welcoming the user.
Mention what you can help with based on your capabilities above.
Keep it concise and professional. Do not use asterisks for formatting.

Your greeting:"#
    )
}

/// Build a proper OpenAI ChatRequest with separate messages for streaming.
fn build_streaming_chat_request(preamble: &str, messages: &[Message]) -> ChatRequest {
    let mut request = ChatRequest::new("gpt-4o");
    request.messages.push(OaiMessage::system(preamble));

    for msg in messages {
        let oai_msg = match msg.role.as_str() {
            "user" => OaiMessage::user(&msg.content),
            "assistant" => OaiMessage::assistant(&msg.content),
            _ => continue,
        };
        request.messages.push(oai_msg);
    }

    request
}
