//! LLM-based post deduplication
//!
//! Core insight: Post identity = Organization × Service × Audience
//! Two posts are duplicates only if same org + same service + same audience.
//!
//! Key principles:
//! 1. Published posts (active) are immutable - cannot delete, only unpublish
//! 2. Pending posts can be merged/dropped freely
//! 3. Different audience = different post (volunteer vs recipient = 2 posts)
//! 4. Same service described differently = merge (LLM understands identity)

use anyhow::Result;
use openai_client::OpenAIClient;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::{PostId, WebsiteId};
use crate::domains::posts::models::{Post, UpdatePostContent};
use crate::kernel::LlmRequestExt;

/// Post data formatted for deduplication analysis
#[derive(Debug, Clone, Serialize)]
pub struct PostForDedup {
    pub id: String,
    pub title: String,
    pub description: String,
    pub primary_audience: Option<String>,
    pub status: String,
}

/// A group of duplicate posts identified by the LLM
#[derive(Debug, Clone, Deserialize)]
pub struct DuplicateGroup {
    /// ID of the post to keep (canonical)
    pub canonical_id: String,
    /// IDs of posts to merge into canonical (will be soft-deleted)
    pub duplicate_ids: Vec<String>,
    /// Improved title (if merge improves it)
    #[serde(default)]
    pub merged_title: Option<String>,
    /// Improved description (if merge improves it)
    #[serde(default)]
    pub merged_description: Option<String>,
    /// Reasoning for why these are duplicates
    pub reasoning: String,
}

/// Result of LLM deduplication analysis
#[derive(Debug, Clone, Deserialize)]
pub struct DuplicateAnalysis {
    /// Groups of duplicate posts
    pub duplicate_groups: Vec<DuplicateGroup>,
    /// Post IDs that are unique (no duplicates found)
    pub unique_post_ids: Vec<String>,
}

/// Analyze posts for a website using LLM to identify duplicates
///
/// Returns a DuplicateAnalysis with groups of duplicates and unique posts.
pub async fn deduplicate_posts_llm(
    website_id: WebsiteId,
    ai: &OpenAIClient,
    pool: &PgPool,
) -> Result<DuplicateAnalysis> {
    // Get all non-deleted posts for this website
    let posts = Post::find_active_by_website(website_id, pool).await?;

    if posts.len() < 2 {
        info!(
            website_id = %website_id,
            posts_count = posts.len(),
            "Too few posts to deduplicate"
        );
        return Ok(DuplicateAnalysis {
            duplicate_groups: vec![],
            unique_post_ids: posts.iter().map(|p| p.id.as_uuid().to_string()).collect(),
        });
    }

    info!(
        website_id = %website_id,
        posts_count = posts.len(),
        "Running LLM deduplication analysis"
    );

    // Convert posts to dedup format
    let posts_for_dedup: Vec<PostForDedup> = posts
        .iter()
        .map(|p| {
            // Extract primary audience from tags if available
            // For now, we'll rely on the LLM to infer it from the content
            PostForDedup {
                id: p.id.as_uuid().to_string(),
                title: p.title.clone(),
                description: p.description.clone(),
                primary_audience: None, // Will be inferred by LLM
                status: p.status.clone(),
            }
        })
        .collect();

    // Build the prompt
    let posts_json = serde_json::to_string_pretty(&posts_for_dedup)?;

    let result: DuplicateAnalysis = ai
        .request()
        .model("gpt-5")
        .system(DEDUP_SYSTEM_PROMPT)
        .user(&format!(
            "Analyze these posts for duplicates:\n\n{}",
            posts_json
        ))
        .schema_hint(DEDUP_SCHEMA)
        .max_retries(3)
        .output()
        .await?;

    info!(
        website_id = %website_id,
        duplicate_groups = result.duplicate_groups.len(),
        unique_posts = result.unique_post_ids.len(),
        "LLM deduplication analysis complete"
    );

    Ok(result)
}

/// Apply deduplication results to the database
///
/// For each duplicate group:
/// 1. Find the canonical post (published/active wins, else most complete)
/// 2. Optionally update canonical with merged content if improved
/// 3. Soft-delete duplicates with reason explaining the merge
///
/// Returns the count of posts soft-deleted.
pub async fn apply_dedup_results(
    result: DuplicateAnalysis,
    ai: &OpenAIClient,
    pool: &PgPool,
) -> Result<usize> {
    let mut deleted_count = 0;

    for group in result.duplicate_groups {
        let canonical_id = match Uuid::parse_str(&group.canonical_id) {
            Ok(id) => PostId::from(id),
            Err(e) => {
                warn!(
                    canonical_id = %group.canonical_id,
                    error = %e,
                    "Failed to parse canonical post ID, skipping group"
                );
                continue;
            }
        };

        // Load canonical post
        let canonical = match Post::find_by_id(canonical_id, pool).await? {
            Some(p) => p,
            None => {
                warn!(
                    canonical_id = %canonical_id,
                    "Canonical post not found, skipping group"
                );
                continue;
            }
        };

        // Update canonical post with merged content if provided
        if group.merged_title.is_some() || group.merged_description.is_some() {
            if let Err(e) = Post::update_content(
                UpdatePostContent::builder()
                    .id(canonical_id)
                    .title(group.merged_title)
                    .description(group.merged_description)
                    .build(),
                pool,
            )
            .await
            {
                warn!(
                    canonical_id = %canonical_id,
                    error = %e,
                    "Failed to update canonical post with merged content"
                );
            }
        }

        // Soft-delete each duplicate
        for dup_id_str in &group.duplicate_ids {
            let dup_id = match Uuid::parse_str(dup_id_str) {
                Ok(id) => PostId::from(id),
                Err(e) => {
                    warn!(
                        duplicate_id = %dup_id_str,
                        error = %e,
                        "Failed to parse duplicate post ID, skipping"
                    );
                    continue;
                }
            };

            // Load duplicate to get its title for the reason
            let dup_post = match Post::find_by_id(dup_id, pool).await? {
                Some(p) => p,
                None => {
                    warn!(
                        duplicate_id = %dup_id,
                        "Duplicate post not found, skipping"
                    );
                    continue;
                }
            };

            // Don't delete active (published) posts - only pending ones
            if dup_post.status == "active" {
                info!(
                    duplicate_id = %dup_id,
                    title = %dup_post.title,
                    "Skipping active/published post (immutable)"
                );
                continue;
            }

            // Generate merge reason using AI
            let reason = generate_merge_reason(
                &dup_post.title,
                &canonical.title,
                canonical_id,
                &group.reasoning,
                ai,
            )
            .await
            .unwrap_or_else(|_| {
                format!(
                    "Merged with '{}' ({}). {}",
                    canonical.title,
                    canonical_id.as_uuid(),
                    group.reasoning
                )
            });

            match Post::soft_delete(dup_id, &reason, pool).await {
                Ok(_) => {
                    info!(
                        duplicate_id = %dup_id,
                        canonical_id = %canonical_id,
                        duplicate_title = %dup_post.title,
                        canonical_title = %canonical.title,
                        "Soft-deleted duplicate post"
                    );
                    deleted_count += 1;
                }
                Err(e) => {
                    warn!(
                        duplicate_id = %dup_id,
                        error = %e,
                        "Failed to soft-delete duplicate post"
                    );
                }
            }
        }
    }

    Ok(deleted_count)
}

/// Generate an AI explanation for why two posts were merged
async fn generate_merge_reason(
    removed_title: &str,
    kept_title: &str,
    kept_id: PostId,
    reasoning: &str,
    ai: &OpenAIClient,
) -> Result<String> {
    let prompt = format!(
        r#"Write a brief, friendly explanation (1-2 sentences) for why a listing was merged with another.

Removed listing: "{}"
Kept listing: "{}" (ID: {})
Reasoning: {}

The explanation should:
- Be written for end users who might have bookmarked the old listing
- Explain they can find the same information at the kept listing
- Sound natural and helpful, not technical

Example: "This listing has been consolidated with 'Community Food Shelf' to provide you with the most complete and up-to-date information in one place.""#,
        removed_title,
        kept_title,
        kept_id.as_uuid(),
        reasoning
    );

    let reason: String = ai
        .request()
        .model("gpt-5")
        .system("You write brief, user-friendly explanations for content merges. Keep responses under 200 characters.")
        .user(&prompt)
        .text()
        .await?;

    // Clean up response (remove quotes if AI wrapped it)
    let reason = reason.trim().trim_matches('"').to_string();

    Ok(reason)
}

const DEDUP_SYSTEM_PROMPT: &str = r#"You are analyzing posts from a single organization's website to identify duplicates.

## Core Principle: Post Identity = Organization × Service × Audience

Two posts are DUPLICATES only if they describe:
1. The SAME organization (they all do - same website)
2. The SAME service/program
3. The SAME target audience

## Key Rules

### Different Audience = Different Post (NOT duplicates)
- "Food Shelf" (for recipients getting food) ≠ "Food Shelf - Volunteer" (for people helping)
- "Donate to X" (for donors) ≠ "Get Help from X" (for recipients)
- These serve DIFFERENT user needs and should remain separate

### Same Service + Same Audience = Duplicates (should merge)
- "Valley Outreach Food Pantry" and "Food Pantry at Valley Outreach" → Same thing, merge them
- "Help with Groceries" and "Food Assistance Program" → If same service, merge them

### Published Posts (status: "active") Are Immutable
- If a group has one "active" post → that's the canonical one
- Other posts in the group should be marked as duplicates
- Never delete/merge the active post - it may have external links

## Analysis Instructions

1. Group posts that describe the SAME service for the SAME audience
2. Identify which post should be canonical (prefer "active" status, then most complete)
3. Note if merging descriptions would improve the canonical post
4. Provide clear reasoning for each duplicate group

## Output Format

Return JSON with:
- duplicate_groups: Array of groups, each with canonical_id, duplicate_ids, optional merged content, and reasoning
- unique_post_ids: Array of post IDs that have no duplicates"#;

const DEDUP_SCHEMA: &str = r#"Return JSON in this format:
{
  "duplicate_groups": [
    {
      "canonical_id": "uuid - the post to keep",
      "duplicate_ids": ["uuid1", "uuid2", "... - posts to merge into canonical"],
      "merged_title": "string or null - improved title if merge improves it",
      "merged_description": "string or null - improved description if merge adds value",
      "reasoning": "string - why these are duplicates (same service + same audience)"
    }
  ],
  "unique_post_ids": ["uuid1", "uuid2", "... - posts with no duplicates"]
}"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_for_dedup_serialization() {
        let post = PostForDedup {
            id: "123e4567-e89b-12d3-a456-426614174000".to_string(),
            title: "Food Shelf".to_string(),
            description: "Free groceries for families".to_string(),
            primary_audience: Some("recipient".to_string()),
            status: "pending_approval".to_string(),
        };

        let json = serde_json::to_string(&post).unwrap();
        assert!(json.contains("Food Shelf"));
        assert!(json.contains("recipient"));
    }

    #[test]
    fn test_deduplication_result_deserialization() {
        let json = r#"{
            "duplicate_groups": [
                {
                    "canonical_id": "123e4567-e89b-12d3-a456-426614174000",
                    "duplicate_ids": ["123e4567-e89b-12d3-a456-426614174001"],
                    "merged_title": null,
                    "merged_description": null,
                    "reasoning": "Same food assistance service for recipients"
                }
            ],
            "unique_post_ids": ["123e4567-e89b-12d3-a456-426614174002"]
        }"#;

        let result: DuplicateAnalysis = serde_json::from_str(json).unwrap();
        assert_eq!(result.duplicate_groups.len(), 1);
        assert_eq!(result.unique_post_ids.len(), 1);
        assert_eq!(result.duplicate_groups[0].duplicate_ids.len(), 1);
    }
}
