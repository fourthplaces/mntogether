//! Post deduplication - LLM-based duplicate detection and merging
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
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::auth::{Actor, AdminCapability};
use crate::common::{MemberId, PostId};
use crate::domains::posts::models::{Post, UpdatePostContent};
use crate::domains::website::models::Website;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// ============================================================================
// Types
// ============================================================================

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
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DuplicateAnalysis {
    /// Groups of duplicate posts
    pub duplicate_groups: Vec<DuplicateGroup>,
    /// Post IDs that are unique (no duplicates found)
    pub unique_post_ids: Vec<String>,
}

/// Result of a deduplication run
pub struct DeduplicationRunResult {
    pub duplicates_found: usize,
    pub posts_merged: usize,
    pub posts_deleted: usize,
}

// ============================================================================
// LLM Analysis
// ============================================================================

/// Analyze posts for a source using LLM to identify duplicates
///
/// Returns a DuplicateAnalysis with groups of duplicates and unique posts.
pub async fn deduplicate_posts_llm(
    source_type: &str,
    source_id: Uuid,
    ai: &OpenAIClient,
    pool: &PgPool,
) -> Result<DuplicateAnalysis> {
    // Get all non-deleted posts for this source
    let posts = Post::find_active_by_source(source_type, source_id, pool).await?;

    if posts.len() < 2 {
        info!(
            source_type = %source_type,
            source_id = %source_id,
            posts_count = posts.len(),
            "Too few posts to deduplicate"
        );
        return Ok(DuplicateAnalysis {
            duplicate_groups: vec![],
            unique_post_ids: posts.iter().map(|p| p.id.as_uuid().to_string()).collect(),
        });
    }

    info!(
        source_type = %source_type,
        source_id = %source_id,
        posts_count = posts.len(),
        "Running LLM deduplication analysis"
    );

    // Convert posts to dedup format
    let posts_for_dedup: Vec<PostForDedup> = posts
        .iter()
        .map(|p| {
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
        .extract(
            "gpt-4o",
            DEDUP_SYSTEM_PROMPT,
            &format!("Analyze these posts for duplicates:\n\n{}", posts_json),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Deduplication analysis failed: {}", e))?;

    info!(
        source_type = %source_type,
        source_id = %source_id,
        duplicate_groups = result.duplicate_groups.len(),
        unique_posts = result.unique_post_ids.len(),
        "LLM deduplication analysis complete"
    );

    Ok(result)
}

// ============================================================================
// Apply Results
// ============================================================================

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
        .chat_completion(
            openai_client::ChatRequest::new("gpt-4o")
                .message(openai_client::Message::system("You write brief, user-friendly explanations for content merges. Keep responses under 200 characters."))
                .message(openai_client::Message::user(&prompt)),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Merge reason generation failed: {}", e))
        .map(|r| r.content)?;

    // Clean up response (remove quotes if AI wrapped it)
    let reason = reason.trim().trim_matches('"').to_string();

    Ok(reason)
}

// ============================================================================
// Entry Point (GraphQL)
// ============================================================================

/// Deduplicate posts using LLM-based similarity (admin only)
/// Returns deduplication statistics.
pub async fn deduplicate_posts(
    member_id: Uuid,
    is_admin: bool,
    deps: &ServerDeps,
) -> Result<DeduplicationRunResult> {
    let requested_by = MemberId::from_uuid(member_id);

    Actor::new(requested_by, is_admin)
        .can(AdminCapability::FullAdmin)
        .check(deps)
        .await
        .map_err(|auth_err| anyhow::anyhow!("Authorization denied: {}", auth_err))?;

    info!("Starting LLM-based post deduplication");

    let websites = match Website::find_approved(&deps.db_pool).await {
        Ok(w) => w,
        Err(e) => {
            warn!(error = %e, "Failed to fetch websites for deduplication");
            return Ok(DeduplicationRunResult {
                duplicates_found: 0,
                posts_merged: 0,
                posts_deleted: 0,
            });
        }
    };

    let mut total_deleted = 0;
    let mut total_groups = 0;

    for website in &websites {
        let dedup_result =
            match deduplicate_posts_llm("website", website.id.into_uuid(), deps.ai.as_ref(), &deps.db_pool).await {
                Ok(r) => r,
                Err(e) => {
                    warn!(website_id = %website.id, error = %e, "Failed LLM deduplication");
                    continue;
                }
            };

        total_groups += dedup_result.duplicate_groups.len();

        let deleted = match apply_dedup_results(dedup_result, deps.ai.as_ref(), &deps.db_pool).await
        {
            Ok(d) => d,
            Err(e) => {
                warn!(website_id = %website.id, error = %e, "Failed to apply deduplication");
                continue;
            }
        };

        total_deleted += deleted;

        if deleted > 0 {
            info!(website_id = %website.id, deleted = deleted, "Deduplicated posts");
        }
    }

    info!(
        total_groups = total_groups,
        total_deleted = total_deleted,
        "LLM deduplication complete"
    );

    Ok(DeduplicationRunResult {
        duplicates_found: total_groups,
        posts_merged: total_groups,
        posts_deleted: total_deleted,
    })
}

// ============================================================================
// Per-Website Deduplication (for Restate workflow)
// ============================================================================

/// A match between a pending post and an active post
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PendingActiveMatch {
    pub pending_id: String,
    pub active_id: String,
    pub reasoning: String,
}

impl_restate_serde!(PendingActiveMatch);

/// Wrapper for journaling phase 1 results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase1Result {
    pub groups: Vec<DuplicateGroup>,
}

impl_restate_serde!(Phase1Result);

/// Wrapper for journaling phase 2 results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase2Result {
    pub matches: Vec<PendingActiveMatch>,
}

impl_restate_serde!(Phase2Result);

/// Phase 1: Analyze pending posts to find groups of duplicates among themselves.
///
/// Returns groups where each group has a canonical post and duplicates to remove.
/// If < 2 pending posts, returns empty vec.
pub async fn find_duplicate_pending_posts(
    pending_posts: &[Post],
    ai: &OpenAIClient,
) -> Result<Vec<DuplicateGroup>> {
    if pending_posts.len() < 2 {
        return Ok(vec![]);
    }

    let posts_for_dedup: Vec<PostForDedup> = pending_posts
        .iter()
        .map(|p| PostForDedup {
            id: p.id.as_uuid().to_string(),
            title: p.title.clone(),
            description: p.description.clone(),
            primary_audience: None,
            status: p.status.clone(),
        })
        .collect();

    let posts_json = serde_json::to_string_pretty(&posts_for_dedup)?;

    let result: DuplicateAnalysis = ai
        .extract(
            "gpt-4o",
            DEDUP_PENDING_SYSTEM_PROMPT,
            &format!("Analyze these draft posts for duplicates:\n\n{}", posts_json),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Pending deduplication failed: {}", e))?;

    info!(
        groups = result.duplicate_groups.len(),
        unique = result.unique_post_ids.len(),
        "Phase 1 deduplication complete"
    );

    Ok(result.duplicate_groups)
}

/// Phase 2: For each pending post, check if it duplicates an existing active post.
///
/// Returns pairs of (pending_id, active_id) with reasoning.
/// If either list is empty, returns empty vec.
pub async fn match_pending_to_active_posts(
    pending_posts: &[Post],
    active_posts: &[Post],
    ai: &OpenAIClient,
) -> Result<Vec<PendingActiveMatch>> {
    if pending_posts.is_empty() || active_posts.is_empty() {
        return Ok(vec![]);
    }

    let pending_for_llm: Vec<PostForDedup> = pending_posts
        .iter()
        .map(|p| PostForDedup {
            id: p.id.as_uuid().to_string(),
            title: p.title.clone(),
            description: p.description.clone(),
            primary_audience: None,
            status: "pending".to_string(),
        })
        .collect();

    let active_for_llm: Vec<PostForDedup> = active_posts
        .iter()
        .map(|p| PostForDedup {
            id: p.id.as_uuid().to_string(),
            title: p.title.clone(),
            description: p.description.clone(),
            primary_audience: None,
            status: "active".to_string(),
        })
        .collect();

    let pending_json = serde_json::to_string_pretty(&pending_for_llm)?;
    let active_json = serde_json::to_string_pretty(&active_for_llm)?;

    let result: PendingActiveAnalysis = ai
        .extract(
            "gpt-4o",
            MATCH_PENDING_ACTIVE_SYSTEM_PROMPT,
            &format!(
                "## Draft Posts (pending approval)\n\n{}\n\n## Published Posts (active)\n\n{}",
                pending_json, active_json
            ),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Pending-active matching failed: {}", e))?;

    info!(
        matches = result.matches.len(),
        unmatched = result.unmatched_pending_ids.len(),
        "Phase 2 matching complete"
    );

    Ok(result.matches)
}

/// Result of matching pending posts against active posts
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct PendingActiveAnalysis {
    pub matches: Vec<PendingActiveMatch>,
    pub unmatched_pending_ids: Vec<String>,
}

// ============================================================================
// Constants
// ============================================================================

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

const DEDUP_PENDING_SYSTEM_PROMPT: &str = r#"You are analyzing DRAFT posts from a single organization's website to identify duplicates among them.

## Core Principle: Post Identity = Organization × Service × Audience

Two posts are DUPLICATES only if they describe:
1. The SAME service/program
2. The SAME target audience

## Key Rules

### Different Audience = Different Post (NOT duplicates)
- "Food Shelf" (for recipients) ≠ "Food Shelf - Volunteer" (for helpers)
- These serve DIFFERENT user needs and should remain separate

### Same Service + Same Audience = Duplicates (should merge)
- "Valley Outreach Food Pantry" and "Food Pantry at Valley Outreach" → Same thing
- "Help with Groceries" and "Food Assistance Program" → If same service, merge

## Analysis Instructions

1. Group draft posts that describe the SAME service for the SAME audience
2. Pick the most complete post as canonical
3. If merging descriptions would create a better post, provide merged_title/merged_description
4. Provide clear reasoning for each group

## Output Format

Return JSON with:
- duplicate_groups: Array of groups, each with canonical_id, duplicate_ids, optional merged content, and reasoning
- unique_post_ids: Array of post IDs that have no duplicates"#;

const MATCH_PENDING_ACTIVE_SYSTEM_PROMPT: &str = r#"You are comparing DRAFT posts against PUBLISHED posts from the same organization's website.

For each draft post, determine if it describes the same service/program for the same audience as any published post.

## Core Principle: Post Identity = Organization × Service × Audience

A draft MATCHES a published post only if:
1. Same service/program
2. Same target audience

## Key Rules

- Different audience = NOT a match (volunteer vs recipient = different posts)
- Same service described differently = MATCH
- A draft that adds genuinely new information to a published post IS a match (it's an update)
- A draft about a completely different service = NOT a match

## Output Format

Return JSON with:
- matches: Array of {pending_id, active_id, reasoning} for drafts that duplicate published posts
- unmatched_pending_ids: Array of draft post IDs that are genuinely new (no published equivalent)"#;

// ============================================================================
// Tests
// ============================================================================

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
