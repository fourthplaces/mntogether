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
use ai_client::OpenAi;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::auth::{Actor, AdminCapability};
use crate::common::{ExtractedPost, MemberId, PostId, SyncBatchId};
use crate::domains::posts::models::{Post, UpdatePostContent};
use crate::domains::source::models::Source;
use crate::domains::sync::activities::{stage_proposals, ProposedOperation};
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
    ai: &OpenAi,
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
    ai: &OpenAi,
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
    ai: &OpenAi,
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
            "You write brief, user-friendly explanations for content merges. Keep responses under 200 characters.",
            &prompt,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Merge reason generation failed: {}", e))?;

    // Clean up response (remove quotes if AI wrapped it)
    let reason = reason.trim().trim_matches('"').to_string();

    Ok(reason)
}

// ============================================================================
// Entry Point
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
// Cross-Source Deduplication (via Sync Batches)
// ============================================================================

/// Post data formatted for cross-source deduplication analysis
#[derive(Debug, Clone, Serialize)]
pub struct PostForCrossSourceDedup {
    pub id: String,
    pub title: String,
    pub description: String,
    pub summary: Option<String>,
    pub source_type: String,
    pub status: String,
}

/// Result of staging cross-source dedup proposals for a single org
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageCrossSourceResult {
    pub batch_id: Option<SyncBatchId>,
    pub proposals_staged: usize,
}

impl_restate_serde!(StageCrossSourceResult);

/// Result of running cross-source dedup across all orgs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossSourceDedupResult {
    pub batches_created: usize,
    pub total_proposals: usize,
    pub orgs_processed: usize,
}

impl_restate_serde!(CrossSourceDedupResult);

/// Detect cross-source duplicates for a single organization using LLM analysis.
pub async fn detect_cross_source_duplicates(
    org_id: Uuid,
    ai: &OpenAi,
    pool: &PgPool,
) -> Result<DuplicateAnalysis> {
    let posts = Post::find_active_pending_by_organization_with_source(org_id, pool).await?;

    if posts.len() < 2 {
        return Ok(DuplicateAnalysis {
            duplicate_groups: vec![],
            unique_post_ids: posts.iter().map(|p| p.id.as_uuid().to_string()).collect(),
        });
    }

    // Check if there are multiple source types - if all from same source, no cross-source dupes
    let source_types: std::collections::HashSet<&str> =
        posts.iter().map(|p| p.source_type.as_str()).collect();
    if source_types.len() < 2 {
        info!(
            org_id = %org_id,
            source_type = ?source_types,
            "Single source type for org, skipping cross-source dedup"
        );
        return Ok(DuplicateAnalysis {
            duplicate_groups: vec![],
            unique_post_ids: posts.iter().map(|p| p.id.as_uuid().to_string()).collect(),
        });
    }

    info!(
        org_id = %org_id,
        posts_count = posts.len(),
        source_types = ?source_types,
        "Running cross-source deduplication analysis"
    );

    let posts_for_dedup: Vec<PostForCrossSourceDedup> = posts
        .iter()
        .map(|p| PostForCrossSourceDedup {
            id: p.id.as_uuid().to_string(),
            title: p.title.clone(),
            description: p.description.clone(),
            summary: p.summary.clone(),
            source_type: p.source_type.clone(),
            status: p.status.clone(),
        })
        .collect();

    let posts_json = serde_json::to_string_pretty(&posts_for_dedup)?;

    let result: DuplicateAnalysis = ai
        .extract(
            "gpt-4o",
            CROSS_SOURCE_DEDUP_SYSTEM_PROMPT,
            &format!(
                "Analyze these posts from the same organization across different sources for cross-platform duplicates:\n\n{}",
                posts_json
            ),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Cross-source deduplication analysis failed: {}", e))?;

    info!(
        org_id = %org_id,
        duplicate_groups = result.duplicate_groups.len(),
        unique_posts = result.unique_post_ids.len(),
        "Cross-source deduplication analysis complete"
    );

    Ok(result)
}

/// Stage cross-source dedup proposals for a single organization.
/// Creates a sync batch with merge proposals for admin review.
pub async fn stage_cross_source_dedup(
    org_id: Uuid,
    ai: &OpenAi,
    pool: &PgPool,
) -> Result<StageCrossSourceResult> {
    let analysis = detect_cross_source_duplicates(org_id, ai, pool).await?;

    if analysis.duplicate_groups.is_empty() {
        return Ok(StageCrossSourceResult {
            batch_id: None,
            proposals_staged: 0,
        });
    }

    let mut proposed_ops = Vec::new();

    for group in &analysis.duplicate_groups {
        let canonical_uuid = match Uuid::parse_str(&group.canonical_id) {
            Ok(u) => u,
            Err(e) => {
                warn!(
                    canonical_id = %group.canonical_id,
                    error = %e,
                    "Invalid canonical UUID in cross-source dedup, skipping group"
                );
                continue;
            }
        };

        // If the LLM provided merged content, create a revision for review
        let draft_id = if group.merged_title.is_some() || group.merged_description.is_some() {
            let canonical = match Post::find_by_id(PostId::from(canonical_uuid), pool).await? {
                Some(p) => p,
                None => {
                    warn!(canonical_id = %canonical_uuid, "Canonical post not found, skipping");
                    continue;
                }
            };

            let fake_fresh = ExtractedPost {
                title: group
                    .merged_title
                    .clone()
                    .unwrap_or_else(|| canonical.title.clone()),
                summary: canonical.summary.clone().unwrap_or_default(),
                description: group
                    .merged_description
                    .clone()
                    .unwrap_or_else(|| canonical.description.clone()),
                location: canonical.location.clone(),
                contact: None,
                source_url: canonical.source_url.clone(),
                urgency: None,
                confidence: None,
                source_page_snapshot_id: None,
                zip_code: None,
                city: None,
                state: None,
                tags: std::collections::HashMap::new(),
                schedule: Vec::new(),
            };

            match super::llm_sync::update_post_with_owner(
                canonical.id,
                &fake_fresh,
                false,
                None,
                pool,
            )
            .await
            {
                Ok(()) => Post::find_revision_for_post(canonical.id, pool)
                    .await
                    .ok()
                    .flatten()
                    .map(|r| r.id.into_uuid()),
                Err(e) => {
                    warn!(
                        canonical_id = %canonical_uuid,
                        error = %e,
                        "Failed to create merge revision for cross-source dedup"
                    );
                    None
                }
            }
        } else {
            None
        };

        let merge_source_ids: Vec<Uuid> = group
            .duplicate_ids
            .iter()
            .filter(|id| id.as_str() != group.canonical_id)
            .filter_map(|id| Uuid::parse_str(id).ok())
            .collect();

        if merge_source_ids.is_empty() {
            warn!(
                canonical_id = %canonical_uuid,
                "Cross-source merge with no valid duplicate_ids, skipping"
            );
            continue;
        }

        proposed_ops.push(ProposedOperation {
            operation: "merge".to_string(),
            entity_type: "post".to_string(),
            draft_entity_id: draft_id,
            target_entity_id: Some(canonical_uuid),
            reason: Some(group.reasoning.clone()),
            merge_source_ids,
        });
    }

    if proposed_ops.is_empty() {
        return Ok(StageCrossSourceResult {
            batch_id: None,
            proposals_staged: 0,
        });
    }

    let proposal_count = proposed_ops.len();

    let stage_result = stage_proposals(
        "post",
        org_id,
        Some("Cross-source duplicate detection"),
        proposed_ops,
        pool,
    )
    .await?;

    info!(
        org_id = %org_id,
        batch_id = %stage_result.batch_id,
        proposals = proposal_count,
        "Staged cross-source dedup proposals"
    );

    Ok(StageCrossSourceResult {
        batch_id: Some(stage_result.batch_id),
        proposals_staged: proposal_count,
    })
}

/// Run cross-source deduplication across all organizations with multiple sources.
/// Admin-only. Creates sync batches with merge proposals for review.
pub async fn deduplicate_cross_source_all_orgs(
    member_id: Uuid,
    is_admin: bool,
    deps: &ServerDeps,
) -> Result<CrossSourceDedupResult> {
    let requested_by = MemberId::from_uuid(member_id);

    Actor::new(requested_by, is_admin)
        .can(AdminCapability::FullAdmin)
        .check(deps)
        .await
        .map_err(|auth_err| anyhow::anyhow!("Authorization denied: {}", auth_err))?;

    info!("Starting cross-source deduplication across all organizations");

    let org_ids = Source::find_org_ids_with_multiple_sources(&deps.db_pool).await?;

    info!(
        org_count = org_ids.len(),
        "Found organizations with multiple sources"
    );

    let mut batches_created = 0;
    let mut total_proposals = 0;

    for org_id in &org_ids {
        match stage_cross_source_dedup(org_id.into_uuid(), deps.ai.as_ref(), &deps.db_pool).await {
            Ok(result) => {
                if result.batch_id.is_some() {
                    batches_created += 1;
                    total_proposals += result.proposals_staged;
                }
            }
            Err(e) => {
                warn!(
                    org_id = %org_id,
                    error = %e,
                    "Failed cross-source dedup for org, continuing"
                );
            }
        }
    }

    info!(
        orgs_processed = org_ids.len(),
        batches_created = batches_created,
        total_proposals = total_proposals,
        "Cross-source deduplication complete"
    );

    Ok(CrossSourceDedupResult {
        batches_created,
        total_proposals,
        orgs_processed: org_ids.len(),
    })
}

const CROSS_SOURCE_DEDUP_SYSTEM_PROMPT: &str = r#"You are analyzing posts from the SAME organization that were scraped from DIFFERENT sources (website, Instagram, Facebook, TikTok, X/Twitter).

## Goal
Identify posts that describe the SAME resource/service/event but were found on different platforms.

## Core Principle
Cross-platform duplicates are common because organizations post the same content on their website AND social media. The same food shelf might appear as:
- A detailed page on their website
- A shorter Instagram post with a photo
- A Facebook post with slightly different wording

## Source Priority (for choosing canonical)
When choosing which post to keep as canonical, prefer in this order:
1. **website** — most detailed, structured content
2. **facebook** — often includes good descriptions
3. **instagram** — shorter, but may have fresher info
4. **tiktok** — video-first, least text
5. **x** — shortest content

## Merge Strategy
When merging:
- Use the **website** version for structure and detail
- Incorporate any **unique information** from social posts (e.g., updated hours, special events, phone numbers)
- The merged result should be the best of both worlds

## Key Rules

### Same Resource = Duplicate
- "Valley Outreach Food Pantry" on website + "Food pantry open Tues/Thurs!" on Instagram for the same org → DUPLICATES

### Different Services = NOT Duplicates
- "Food Shelf" from website + "Legal Aid Clinic" from Instagram → NOT duplicates (different services)

### Different Audience = NOT Duplicates
- "Volunteer at our food shelf" + "Get free groceries" → NOT duplicates (different audiences)

### Published Posts ("active") Preferred as Canonical
- If one is "active" and another "pending_approval", the active one is canonical

## Output Format

Return JSON with:
- duplicate_groups: Array of groups, each with canonical_id, duplicate_ids, optional merged_title/merged_description, and reasoning
- unique_post_ids: Array of post IDs that have no cross-source duplicates"#;

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
    ai: &OpenAi,
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
    ai: &OpenAi,
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
