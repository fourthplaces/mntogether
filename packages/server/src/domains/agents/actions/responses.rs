//! AI response generation actions for agents domain.
//!
//! Moved from chatrooms/actions/ai_responses.rs.
//! Contains greeting generation, reply generation (blocking + streaming),
//! and prompt building helpers.
//!
//! Agent identity and preamble come from the `agents` table.

use anyhow::Result;
use tracing::{error, info};

use crate::common::{ContainerId, MessageId};
use crate::domains::agents::events::ChatStreamEvent;
use crate::domains::agents::models::Agent;
use crate::domains::chatrooms::models::{Container, Message};
use crate::domains::tag::Tag;
use crate::kernel::{CompletionExt, SearchPostsTool, ServerDeps};

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

    let agent = Agent::get_or_create_by_config(agent_config, &deps.db_pool).await?;

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
    let agent = Agent::get_or_create_by_config(&agent_config, &deps.db_pool).await?;

    let messages = Message::find_by_container(container_id, &deps.db_pool)
        .await
        .map_err(|e| {
            error!("Failed to load conversation: {}", e);
            anyhow::anyhow!("Failed to load conversation: {}", e)
        })?;

    let oai_messages = build_oai_messages(&agent.preamble, &messages);

    let response = deps
        .ai
        .agent("gpt-4o")
        .tool(SearchPostsTool::new(
            deps.db_pool.clone(),
            deps.embedding_service.clone(),
        ))
        .max_iterations(3)
        .build()
        .chat_with_history(oai_messages)
        .await
        .map_err(|e| {
            error!("Agent reply failed: {}", e);
            anyhow::anyhow!("AI reply generation failed: {}", e)
        })?;

    let reply_text = response.content;

    info!(
        message_id = %message_id,
        reply_length = reply_text.len(),
        tool_calls = ?response.tool_calls_made,
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
/// Phase 1: Runs a non-streaming agent loop with tools (model can call search_posts).
/// Phase 2: Simulates streaming by chunking the final response into token deltas.
/// Then saves the complete message to DB.
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

    // Resolve agent config from container tags
    let agent_config = get_container_agent_config(container_id, &deps.db_pool)
        .await
        .unwrap_or_else(|| "admin".to_string());
    let agent = Agent::get_or_create_by_config(&agent_config, &deps.db_pool).await?;

    let messages = Message::find_by_container(container_id, &deps.db_pool).await?;
    let topic = ChatStreamEvent::topic(&container_id.to_string());

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

    // Phase 1: Agent with tools (non-streaming)
    let oai_messages = build_oai_messages(&agent.preamble, &messages);

    // Wire on_tool_result callback to publish ToolResult events to StreamHub
    let hub = deps.stream_hub.clone();
    let topic_clone = topic.clone();
    let cid = container_id.to_string();

    let agent_result = deps
        .ai
        .agent("gpt-4o")
        .tool(SearchPostsTool::new(
            deps.db_pool.clone(),
            deps.embedding_service.clone(),
        ))
        .on_tool_result(move |tool_name, call_id, result_json| {
            let hub = hub.clone();
            let topic = topic_clone.clone();
            let cid = cid.clone();
            let tn = tool_name.to_string();
            let ci = call_id.to_string();
            let rj = result_json.to_string();
            tokio::spawn(async move {
                let results = serde_json::from_str::<serde_json::Value>(&rj)
                    .unwrap_or(serde_json::Value::Null);
                let event = ChatStreamEvent::ToolResult {
                    container_id: cid,
                    tool_name: tn,
                    call_id: ci,
                    results,
                };
                if let Ok(val) = serde_json::to_value(event) {
                    hub.publish(&topic, val).await;
                }
            });
        })
        .max_iterations(3)
        .build()
        .chat_with_history(oai_messages)
        .await;

    let response = match agent_result {
        Ok(r) => r,
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
            return Err(anyhow::anyhow!("Agent reply failed: {}", e));
        }
    };

    info!(
        tool_calls = ?response.tool_calls_made,
        iterations = response.iterations,
        "Agent phase complete, simulating stream"
    );

    // Phase 2: Simulate streaming by chunking the response
    let content = &response.content;
    let chunk_size = 4;
    let mut pos = 0;

    while pos < content.len() {
        // Respect char boundaries
        let end = content
            .char_indices()
            .map(|(i, _)| i)
            .find(|&i| i >= pos + chunk_size)
            .unwrap_or(content.len());

        let delta = &content[pos..end];
        if !delta.is_empty() {
            deps.stream_hub
                .publish(
                    &topic,
                    serde_json::to_value(ChatStreamEvent::TokenDelta {
                        container_id: container_id.to_string(),
                        delta: delta.to_string(),
                    })?,
                )
                .await;

            tokio::time::sleep(std::time::Duration::from_millis(15)).await;
        }
        pos = end;
    }

    // Save complete message to DB
    let sequence_number = Message::next_sequence_number(container_id, &deps.db_pool).await?;

    let new_message = Message::create(
        container_id,
        "assistant".to_string(),
        response.content.clone(),
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
                content: response.content,
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

fn build_greeting_prompt(preamble: &str) -> String {
    format!(
        r#"{preamble}

Generate a brief, friendly greeting (1-2 sentences) welcoming the user.
Mention what you can help with based on your capabilities above.
Keep it concise and professional. Do not use asterisks for formatting.

Your greeting:"#
    )
}

/// Build OpenAI message array from preamble + conversation history.
fn build_oai_messages(preamble: &str, messages: &[Message]) -> Vec<serde_json::Value> {
    let mut oai_messages = vec![serde_json::json!({
        "role": "system",
        "content": preamble
    })];

    for msg in messages {
        let role = match msg.role.as_str() {
            "user" => "user",
            "assistant" => "assistant",
            _ => continue,
        };
        oai_messages.push(serde_json::json!({
            "role": role,
            "content": msg.content
        }));
    }

    oai_messages
}
