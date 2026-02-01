//! AI Semantic Deduplication for Resources
//!
//! This module implements the AI-powered deduplication strategy:
//! 1. Generate embedding for new content
//! 2. Find similar existing resources using vector search (pre-filter)
//! 3. If similar resources found, ask AI to decide: NEW, UPDATE, or SKIP
//!
//! The embedding pre-filter ensures we only call AI when there's a potential
//! duplicate, making this approach scalable to large datasets.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::info;

use crate::common::WebsiteId;
use crate::domains::resources::models::Resource;
use crate::kernel::llm_request::LlmRequestExt;
use crate::kernel::{BaseAI, BaseEmbeddingService};

/// Similarity threshold for pre-filter (0.0 to 1.0)
/// Resources with similarity above this are candidates for AI comparison
const SIMILARITY_THRESHOLD: f32 = 0.75;

/// Maximum number of similar resources to compare against
const MAX_SIMILAR_CANDIDATES: i32 = 5;

/// Result of deduplication decision
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum DedupAction {
    /// Create as new resource
    New {
        reasoning: String,
    },
    /// Update an existing resource
    Update {
        existing_id: uuid::Uuid,
        similarity_score: f32,
        reasoning: String,
    },
    /// Skip - duplicate with no new information
    Skip {
        existing_id: uuid::Uuid,
        similarity_score: f32,
        reasoning: String,
    },
}

/// Input for deduplication check
#[derive(Debug, Clone)]
pub struct DedupInput {
    pub title: String,
    pub content: String,
    pub location: Option<String>,
    pub organization_name: Option<String>,
}

/// AI response structure for deduplication decision
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AiDedupResponse {
    decision: String,  // "new", "update", "skip"
    existing_id: Option<String>,
    reasoning: String,
}

/// Check for duplicates and decide what action to take
///
/// Returns a DedupAction indicating whether to:
/// - Create a new resource (no duplicates found)
/// - Update an existing resource (same entity, new info)
/// - Skip (duplicate with no new information)
pub async fn deduplicate_resource(
    input: &DedupInput,
    website_id: WebsiteId,
    embedding_service: &dyn BaseEmbeddingService,
    ai: &dyn BaseAI,
    pool: &PgPool,
) -> Result<DedupAction> {
    // Step 1: Generate embedding for the new content
    let content_for_embedding = format!(
        "{}\n\n{}\n\nLocation: {}\nOrganization: {}",
        input.title,
        input.content,
        input.location.as_deref().unwrap_or("Not specified"),
        input.organization_name.as_deref().unwrap_or("Unknown")
    );

    let embedding = embedding_service.generate(&content_for_embedding).await?;

    // Step 2: Find similar resources using vector search
    let similar_resources = Resource::find_similar_by_embedding(
        &embedding,
        website_id,
        SIMILARITY_THRESHOLD,
        MAX_SIMILAR_CANDIDATES,
        pool,
    )
    .await?;

    if similar_resources.is_empty() {
        // No similar resources found - create new
        info!(
            title = %input.title,
            "No similar resources found, will create new"
        );
        return Ok(DedupAction::New {
            reasoning: "No similar existing resources found".to_string(),
        });
    }

    info!(
        title = %input.title,
        similar_count = similar_resources.len(),
        "Found similar resources, asking AI to decide"
    );

    // Step 3: Ask AI to make the decision
    let ai_decision = ask_ai_for_decision(input, &similar_resources, ai).await?;

    Ok(ai_decision)
}

/// Ask AI to compare new content against similar existing resources
async fn ask_ai_for_decision(
    input: &DedupInput,
    similar_resources: &[(Resource, f32)],
    ai: &dyn BaseAI,
) -> Result<DedupAction> {
    // Build context about existing resources
    let existing_context: String = similar_resources
        .iter()
        .enumerate()
        .map(|(i, (r, score))| {
            format!(
                "EXISTING RESOURCE #{} (similarity: {:.2})\n\
                 ID: {}\n\
                 Title: {}\n\
                 Content: {}\n\
                 Location: {}\n\
                 Organization: {}\n\
                 ---",
                i + 1,
                score,
                r.id.into_uuid(),
                r.title,
                truncate(&r.content, 500),
                r.location.as_deref().unwrap_or("Not specified"),
                r.organization_name.as_deref().unwrap_or("Unknown")
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let system_prompt = r#"You are a content deduplication expert. Your job is to determine whether newly extracted content is:
1. NEW: A distinct resource that should be created
2. UPDATE: The same resource as an existing one, but with new/updated information
3. SKIP: A duplicate of an existing resource with no meaningful new information

Consider:
- Same organization + same service/program = likely same resource
- Minor wording differences = probably the same
- Different contact info, hours, or details = might be an UPDATE
- Completely different services/programs = NEW

Return your decision as JSON with these fields:
- decision: "new", "update", or "skip"
- existing_id: If update/skip, the ID of the matching existing resource
- reasoning: Brief explanation of your decision"#;

    let user_prompt = format!(
        r#"Compare this newly extracted content against existing resources:

NEW EXTRACTED CONTENT:
Title: {}
Content: {}
Location: {}
Organization: {}

EXISTING RESOURCES:
{}

Based on this comparison, should this new content be:
- NEW: Create as a new resource
- UPDATE: Update one of the existing resources (same entity, new info)
- SKIP: Skip it (duplicate, no new info)

Return your decision as JSON."#,
        input.title,
        truncate(&input.content, 1000),
        input.location.as_deref().unwrap_or("Not specified"),
        input.organization_name.as_deref().unwrap_or("Unknown"),
        existing_context
    );

    let response: AiDedupResponse = ai
        .request()
        .system(system_prompt)
        .user(&user_prompt)
        .max_retries(2)
        .output()
        .await?;

    // Convert AI response to DedupAction
    match response.decision.to_lowercase().as_str() {
        "new" => Ok(DedupAction::New {
            reasoning: response.reasoning,
        }),
        "update" => {
            let existing_id = response
                .existing_id
                .ok_or_else(|| anyhow::anyhow!("AI said update but didn't provide existing_id"))?
                .parse::<uuid::Uuid>()?;

            // Find the similarity score for this resource
            let similarity = similar_resources
                .iter()
                .find(|(r, _)| r.id.into_uuid() == existing_id)
                .map(|(_, score)| *score)
                .unwrap_or(0.0);

            Ok(DedupAction::Update {
                existing_id,
                similarity_score: similarity,
                reasoning: response.reasoning,
            })
        }
        "skip" => {
            let existing_id = response
                .existing_id
                .ok_or_else(|| anyhow::anyhow!("AI said skip but didn't provide existing_id"))?
                .parse::<uuid::Uuid>()?;

            let similarity = similar_resources
                .iter()
                .find(|(r, _)| r.id.into_uuid() == existing_id)
                .map(|(_, score)| *score)
                .unwrap_or(0.0);

            Ok(DedupAction::Skip {
                existing_id,
                similarity_score: similarity,
                reasoning: response.reasoning,
            })
        }
        other => {
            // Default to NEW if AI gives unexpected response
            info!(
                decision = %other,
                "AI returned unexpected decision, defaulting to NEW"
            );
            Ok(DedupAction::New {
                reasoning: format!("AI returned unexpected decision '{}', creating new", other),
            })
        }
    }
}

/// Truncate a string to a maximum length
fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        match s[..max_len].rfind(char::is_whitespace) {
            Some(pos) => &s[..pos],
            None => &s[..max_len],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        // Short string returns as-is
        assert_eq!(truncate("hello", 10), "hello");
        // Truncates at word boundary
        assert_eq!(truncate("hello world test", 12), "hello world");
        // No word boundary - truncates at max
        assert_eq!(truncate("helloworld", 5), "hello");
    }
}
