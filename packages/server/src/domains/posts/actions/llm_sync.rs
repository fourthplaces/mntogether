//! LLM-powered post synchronization
//!
//! Instead of simple title-matching + separate deduplication, this uses
//! a single LLM call to intelligently diff fresh extractions against
//! existing database posts.
//!
//! The LLM determines:
//! - INSERT: New posts that don't exist in DB
//! - UPDATE: Fresh posts that match existing DB posts (semantically)
//! - DELETE: DB posts that no longer exist in fresh extraction
//! - MERGE: Pre-existing duplicates in DB that should be consolidated

use anyhow::Result;
use openai_client::OpenAIClient;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::common::{ExtractedPost, PostId, SyncBatchId, WebsiteId};
use crate::domains::posts::models::{CreatePost, Post, PostContact, UpdatePostContent};
use crate::domains::sync::actions::{stage_proposals, ProposedOperation};
use crate::kernel::LlmRequestExt;

/// A fresh post from extraction, with a temporary ID for matching
#[derive(Debug, Clone, Serialize)]
pub struct FreshPost {
    /// Temporary ID for LLM to reference (e.g., "fresh_1", "fresh_2")
    pub temp_id: String,
    pub title: String,
    pub tldr: String,
    pub description: String,
    /// Primary audience roles: "recipient", "volunteer", "donor", etc.
    pub audience_roles: Vec<String>,
    pub location: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
}

/// An existing post from the database
#[derive(Debug, Clone, Serialize)]
pub struct ExistingPost {
    pub id: String,
    pub title: String,
    pub tldr: Option<String>,
    pub description: String,
    pub status: String,
    pub location: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
}

/// LLM's decision for a single sync operation
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "operation")]
pub enum SyncOperation {
    /// Insert a new post (fresh post has no match in DB)
    #[serde(rename = "insert", alias = "INSERT")]
    Insert { fresh_id: String },

    /// Update an existing post with fresh data
    #[serde(rename = "update", alias = "UPDATE")]
    Update {
        fresh_id: String,
        existing_id: String,
        /// If true, merge fresh description with existing (don't overwrite completely)
        #[serde(default)]
        merge_description: bool,
    },

    /// Delete an existing post (no longer in fresh extraction)
    #[serde(rename = "delete", alias = "DELETE")]
    Delete { existing_id: String, reason: String },

    /// Merge duplicate existing posts (consolidate pre-existing duplicates)
    #[serde(rename = "merge", alias = "MERGE")]
    Merge {
        /// The post to keep (also accept "target_id" from LLM variations)
        #[serde(alias = "target_id")]
        canonical_id: String,
        /// Posts to delete (merge into canonical) - also accept "target_ids" from LLM
        #[serde(alias = "target_ids")]
        duplicate_ids: Vec<String>,
        /// Optional: improved title from merging
        #[serde(default)]
        merged_title: Option<String>,
        /// Optional: improved description from merging
        #[serde(default)]
        merged_description: Option<String>,
        #[serde(default)]
        reason: String,
    },
}

/// Result of LLM sync analysis - accepts multiple formats
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum SyncAnalysisResponse {
    /// Full format: {"operations": [...], "summary": "..."}
    Full {
        operations: Vec<SyncOperation>,
        summary: String,
    },
    /// Array only: [op1, op2, ...]
    ArrayOnly(Vec<SyncOperation>),
}

impl SyncAnalysisResponse {
    pub fn into_parts(self) -> (Vec<SyncOperation>, String) {
        match self {
            Self::Full {
                operations,
                summary,
            } => (operations, summary),
            Self::ArrayOnly(operations) => (operations, "No summary".to_string()),
        }
    }
}

/// Result of LLM-based sync - proposals are staged for review
#[derive(Debug)]
pub struct LlmSyncResult {
    pub batch_id: SyncBatchId,
    pub staged_inserts: usize,
    pub staged_updates: usize,
    pub staged_deletes: usize,
    pub staged_merges: usize,
    pub errors: Vec<String>,
}

/// Convert fresh ExtractedPosts to LLM-friendly FreshPost format
fn convert_fresh_posts(fresh_posts: &[ExtractedPost]) -> Vec<FreshPost> {
    fresh_posts
        .iter()
        .enumerate()
        .map(|(i, p)| FreshPost {
            temp_id: format!("fresh_{}", i + 1),
            title: p.title.clone(),
            tldr: p.tldr.clone(),
            description: p.description.clone(),
            audience_roles: p.audience_roles.clone(),
            location: p.location.clone(),
            contact_phone: p.contact.as_ref().and_then(|c| c.phone.clone()),
            contact_email: p.contact.as_ref().and_then(|c| c.email.clone()),
        })
        .collect()
}

/// Convert existing Posts to LLM-friendly ExistingPost format, loading contact info
async fn convert_existing_posts(existing_posts: &[Post], pool: &PgPool) -> Vec<ExistingPost> {
    let mut existing = Vec::with_capacity(existing_posts.len());
    for p in existing_posts {
        let contacts = PostContact::find_by_post(p.id, pool)
            .await
            .unwrap_or_default();
        let contact_phone = contacts
            .iter()
            .find(|c| c.contact_type == "phone")
            .map(|c| c.contact_value.clone());
        let contact_email = contacts
            .iter()
            .find(|c| c.contact_type == "email")
            .map(|c| c.contact_value.clone());

        existing.push(ExistingPost {
            id: p.id.as_uuid().to_string(),
            title: p.title.clone(),
            tldr: p.tldr.clone(),
            description: p.description.clone(),
            status: p.status.clone(),
            location: p.location.clone(),
            contact_phone,
            contact_email,
        });
    }
    existing
}

/// Log diagnostic info about posts being synced (debug only)
fn log_sync_diagnostics(fresh_posts: &[ExtractedPost], existing_posts: &[Post]) {
    for (i, post) in fresh_posts.iter().enumerate() {
        info!(fresh_id = format!("fresh_{}", i + 1), title = %post.title, has_contact = post.contact.is_some(), "Fresh post for sync");
    }
    for post in existing_posts {
        info!(existing_id = %post.id, title = %post.title, status = %post.status, "Existing post for sync");
    }
}

/// Log each sync operation decision from the LLM
fn log_sync_operations(
    operations: &[SyncOperation],
    fresh: &[FreshPost],
    existing: &[ExistingPost],
) {
    for op in operations {
        match op {
            SyncOperation::Insert { fresh_id } => {
                if let Some(f) = fresh.iter().find(|f| &f.temp_id == fresh_id) {
                    info!(op = "INSERT", fresh_id = %fresh_id, title = %f.title, "LLM decision: insert new post");
                }
            }
            SyncOperation::Update {
                fresh_id,
                existing_id,
                merge_description,
            } => {
                let fresh_title = fresh
                    .iter()
                    .find(|f| &f.temp_id == fresh_id)
                    .map(|f| f.title.as_str())
                    .unwrap_or("?");
                let existing_title = existing
                    .iter()
                    .find(|e| &e.id == existing_id)
                    .map(|e| e.title.as_str())
                    .unwrap_or("?");
                info!(op = "UPDATE", fresh_id = %fresh_id, existing_id = %existing_id, fresh_title = %fresh_title, existing_title = %existing_title, merge_description = %merge_description, "LLM decision: update existing post");
            }
            SyncOperation::Delete {
                existing_id,
                reason,
            } => {
                let existing_title = existing
                    .iter()
                    .find(|e| &e.id == existing_id)
                    .map(|e| e.title.as_str())
                    .unwrap_or("?");
                info!(op = "DELETE", existing_id = %existing_id, existing_title = %existing_title, reason = %reason, "LLM decision: delete stale post");
            }
            SyncOperation::Merge {
                canonical_id,
                duplicate_ids,
                reason,
                ..
            } => {
                let canonical_title = existing
                    .iter()
                    .find(|e| &e.id == canonical_id)
                    .map(|e| e.title.as_str())
                    .unwrap_or("?");
                info!(op = "MERGE", canonical_id = %canonical_id, canonical_title = %canonical_title, duplicate_count = duplicate_ids.len(), reason = %reason, "LLM decision: merge duplicates");
            }
        }
    }
}

/// Build the LLM prompt for sync analysis
fn build_sync_prompt(fresh: &[FreshPost], existing: &[ExistingPost]) -> Result<String> {
    let fresh_json = serde_json::to_string_pretty(fresh)?;
    let existing_json = serde_json::to_string_pretty(existing)?;
    Ok(format!(
        "## Fresh Posts (just extracted from website)\n\n{}\n\n## Existing Posts (currently in database)\n\n{}",
        fresh_json, existing_json
    ))
}

/// Analyze and sync fresh posts against existing DB posts using LLM.
///
/// Instead of applying changes immediately, stages all operations as proposals
/// for human review. INSERTs create draft posts, UPDATEs create revisions,
/// DELETEs and MERGEs are recorded as proposals only.
pub async fn llm_sync_posts(
    website_id: WebsiteId,
    fresh_posts: Vec<ExtractedPost>,
    ai: &OpenAIClient,
    pool: &PgPool,
) -> Result<LlmSyncResult> {
    let existing_db_posts = Post::find_by_website_id(website_id, pool).await?;

    info!(website_id = %website_id, fresh_count = fresh_posts.len(), existing_count = existing_db_posts.len(), "Starting LLM sync analysis");

    if fresh_posts.is_empty() && existing_db_posts.is_empty() {
        // Create an empty batch so callers always get a batch_id
        let stage_result = stage_proposals(
            "post",
            website_id.into_uuid(),
            Some("No posts to sync"),
            vec![],
            pool,
        )
        .await?;
        return Ok(LlmSyncResult {
            batch_id: stage_result.batch_id,
            staged_inserts: 0,
            staged_updates: 0,
            staged_deletes: 0,
            staged_merges: 0,
            errors: vec![],
        });
    }

    log_sync_diagnostics(&fresh_posts, &existing_db_posts);

    let fresh = convert_fresh_posts(&fresh_posts);
    let existing = convert_existing_posts(&existing_db_posts, pool).await;

    let user_prompt = build_sync_prompt(&fresh, &existing)?;

    let response: SyncAnalysisResponse = ai
        .request()
        .system(SYNC_SYSTEM_PROMPT)
        .user(&user_prompt)
        .schema_hint(SYNC_SCHEMA)
        .max_retries(3)
        .output()
        .await?;

    let (operations, summary) = response.into_parts();
    info!(website_id = %website_id, operations_count = operations.len(), summary = %summary, "LLM sync analysis complete");

    log_sync_operations(&operations, &fresh, &existing);

    stage_sync_operations(
        website_id,
        &fresh_posts,
        &existing_db_posts,
        operations,
        &summary,
        pool,
    )
    .await
}

/// Stage sync operations as proposals for human review.
///
/// INSERTs: creates the draft post (pending_approval), records proposal
/// UPDATEs: creates a revision post, records proposal
/// DELETEs: only records proposal (no post changes)
/// MERGEs: creates revision for canonical if merged content provided, records proposal + merge sources
async fn stage_sync_operations(
    website_id: WebsiteId,
    fresh_posts: &[ExtractedPost],
    existing_posts: &[Post],
    operations: Vec<SyncOperation>,
    summary: &str,
    pool: &PgPool,
) -> Result<LlmSyncResult> {
    use crate::domains::posts::actions::create_post::create_extracted_post;
    use crate::domains::website::models::Website;

    let mut proposed_ops: Vec<ProposedOperation> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut staged_inserts = 0;
    let mut staged_updates = 0;
    let mut staged_deletes = 0;
    let mut staged_merges = 0;

    // Build lookup maps
    let fresh_by_id: std::collections::HashMap<String, &ExtractedPost> = fresh_posts
        .iter()
        .enumerate()
        .map(|(i, p)| (format!("fresh_{}", i + 1), p))
        .collect();

    let existing_by_id: std::collections::HashMap<String, &Post> = existing_posts
        .iter()
        .map(|p| (p.id.as_uuid().to_string(), p))
        .collect();

    info!(
        website_id = %website_id,
        operations_count = operations.len(),
        "Staging sync operations as proposals"
    );

    for op in operations {
        match op {
            SyncOperation::Insert { fresh_id } => {
                if let Some(fresh) = fresh_by_id.get(&fresh_id) {
                    let website = match Website::find_by_id(website_id, pool).await {
                        Ok(w) => w,
                        Err(e) => {
                            errors.push(format!("Failed to load website: {}", e));
                            continue;
                        }
                    };

                    match create_extracted_post(
                        &website.domain,
                        fresh,
                        Some(website_id),
                        fresh
                            .source_url
                            .clone()
                            .or_else(|| Some(format!("https://{}", website.domain))),
                        pool,
                    )
                    .await
                    {
                        Ok(post) => {
                            proposed_ops.push(ProposedOperation {
                                operation: "insert".to_string(),
                                entity_type: "post".to_string(),
                                draft_entity_id: Some(post.id.into_uuid()),
                                target_entity_id: None,
                                reason: Some(format!("New post: {}", fresh.title)),
                                merge_source_ids: vec![],
                            });
                            staged_inserts += 1;
                        }
                        Err(e) => {
                            errors.push(format!("Insert {}: {}", fresh_id, e));
                        }
                    }
                } else {
                    tracing::warn!(fresh_id = %fresh_id, "Insert operation references unknown fresh_id");
                }
            }

            SyncOperation::Update {
                fresh_id,
                existing_id,
                merge_description,
            } => {
                if let (Some(fresh), Some(existing)) =
                    (fresh_by_id.get(&fresh_id), existing_by_id.get(&existing_id))
                {
                    match update_post(existing.id, fresh, merge_description, pool).await {
                        Ok(()) => {
                            // Find the revision that was just created
                            let revision = Post::find_revision_for_post(existing.id, pool).await;
                            let revision_id = revision.ok().flatten().map(|r| r.id.into_uuid());

                            proposed_ops.push(ProposedOperation {
                                operation: "update".to_string(),
                                entity_type: "post".to_string(),
                                draft_entity_id: revision_id,
                                target_entity_id: Some(existing.id.into_uuid()),
                                reason: Some(format!("Updated content for: {}", fresh.title)),
                                merge_source_ids: vec![],
                            });
                            staged_updates += 1;
                        }
                        Err(e) => {
                            errors.push(format!("Update {}: {}", existing_id, e));
                        }
                    }
                } else {
                    tracing::warn!(fresh_id = %fresh_id, existing_id = %existing_id, "Update operation references unknown IDs");
                }
            }

            SyncOperation::Delete {
                existing_id,
                reason,
            } => {
                if existing_by_id.contains_key(&existing_id) {
                    let target_uuid = Uuid::parse_str(&existing_id);
                    match target_uuid {
                        Ok(uuid) => {
                            proposed_ops.push(ProposedOperation {
                                operation: "delete".to_string(),
                                entity_type: "post".to_string(),
                                draft_entity_id: None,
                                target_entity_id: Some(uuid),
                                reason: Some(reason),
                                merge_source_ids: vec![],
                            });
                            staged_deletes += 1;
                        }
                        Err(e) => {
                            errors.push(format!("Invalid UUID {}: {}", existing_id, e));
                        }
                    }
                }
            }

            SyncOperation::Merge {
                canonical_id,
                duplicate_ids,
                merged_title,
                merged_description,
                reason,
            } => {
                if let Some(canonical) = existing_by_id.get(&canonical_id) {
                    let canonical_uuid = match Uuid::parse_str(&canonical_id) {
                        Ok(u) => u,
                        Err(e) => {
                            errors.push(format!("Invalid canonical UUID {}: {}", canonical_id, e));
                            continue;
                        }
                    };

                    // If merged content is provided, create a revision for the canonical post
                    let draft_id = if merged_title.is_some() || merged_description.is_some() {
                        let fake_fresh = ExtractedPost {
                            title: merged_title.unwrap_or_else(|| canonical.title.clone()),
                            tldr: canonical.tldr.clone().unwrap_or_default(),
                            description: merged_description
                                .unwrap_or_else(|| canonical.description.clone()),
                            audience_roles: vec![],
                            location: canonical.location.clone(),
                            contact: None,
                            source_url: canonical.source_url.clone(),
                            urgency: None,
                            confidence: None,
                            source_page_snapshot_id: None,
                        };
                        match update_post(canonical.id, &fake_fresh, false, pool).await {
                            Ok(()) => Post::find_revision_for_post(canonical.id, pool)
                                .await
                                .ok()
                                .flatten()
                                .map(|r| r.id.into_uuid()),
                            Err(e) => {
                                errors.push(format!("Merge revision for {}: {}", canonical_id, e));
                                None
                            }
                        }
                    } else {
                        None
                    };

                    // Parse duplicate IDs
                    let mut merge_source_ids = Vec::new();
                    for dup_id in &duplicate_ids {
                        match Uuid::parse_str(dup_id) {
                            Ok(uuid) => merge_source_ids.push(uuid),
                            Err(e) => {
                                errors.push(format!("Invalid duplicate UUID {}: {}", dup_id, e));
                            }
                        }
                    }

                    if merge_source_ids.is_empty() {
                        tracing::warn!(
                            canonical_id = %canonical_id,
                            "LLM returned merge with no duplicate_ids, skipping"
                        );
                    } else {
                        proposed_ops.push(ProposedOperation {
                            operation: "merge".to_string(),
                            entity_type: "post".to_string(),
                            draft_entity_id: draft_id,
                            target_entity_id: Some(canonical_uuid),
                            reason: Some(reason),
                            merge_source_ids,
                        });
                        staged_merges += 1;
                    }
                }
            }
        }
    }

    // Build a summary that reflects what we actually staged
    let actual_summary = if proposed_ops.is_empty() {
        format!("No actionable operations (LLM suggested: {})", summary)
    } else {
        summary.to_string()
    };

    let stage_result = stage_proposals(
        "post",
        website_id.into_uuid(),
        Some(&actual_summary),
        proposed_ops,
        pool,
    )
    .await?;

    info!(
        website_id = %website_id,
        batch_id = %stage_result.batch_id,
        staged_inserts,
        staged_updates,
        staged_deletes,
        staged_merges,
        errors = errors.len(),
        expired_batches = stage_result.expired_batches,
        "Sync operations staged as proposals"
    );

    Ok(LlmSyncResult {
        batch_id: stage_result.batch_id,
        staged_inserts,
        staged_updates,
        staged_deletes,
        staged_merges,
        errors,
    })
}

/// Update an existing post by creating a revision for review
///
/// Instead of directly modifying the original post, this creates a revision
/// post with `revision_of_post_id` pointing to the original. The original
/// stays untouched until an admin approves the revision.
///
/// If a revision already exists for this post, it gets replaced with the new data.
pub async fn update_post(
    post_id: PostId,
    fresh: &ExtractedPost,
    _merge_description: bool,
    pool: &PgPool,
) -> Result<()> {
    // Get original post for context
    let original = Post::find_by_id(post_id, pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Original post not found"))?;

    // Check if revision already exists for this post
    if let Some(existing_revision) = Post::find_revision_for_post(post_id, pool).await? {
        // Replace existing revision by updating its content
        info!(
            post_id = %post_id,
            revision_id = %existing_revision.id,
            "Replacing existing revision with new data"
        );
        Post::update_content(
            UpdatePostContent::builder()
                .id(existing_revision.id)
                .title(Some(fresh.title.clone()))
                .description(Some(fresh.description.clone()))
                .tldr(Some(fresh.tldr.clone()))
                .location(fresh.location.clone())
                .build(),
            pool,
        )
        .await?;

        // Update contact info on the revision if available
        if let Some(ref contact) = fresh.contact {
            let _ = PostContact::delete_all_for_post(existing_revision.id, pool).await;
            let contact_json = serde_json::json!({
                "phone": contact.phone,
                "email": contact.email,
                "website": contact.website
            });
            if let Err(e) =
                PostContact::create_from_json(existing_revision.id, &contact_json, pool).await
            {
                tracing::warn!(
                    revision_id = %existing_revision.id,
                    error = %e,
                    "Failed to update contact info on revision"
                );
            }
        }
        return Ok(());
    }

    // Create revision post with revision_of_post_id set
    info!(
        post_id = %post_id,
        title = %fresh.title,
        "Creating revision post for review"
    );

    let revision = Post::create(
        CreatePost::builder()
            .organization_name(original.organization_name.clone())
            .title(fresh.title.clone())
            .description(fresh.description.clone())
            .tldr(Some(fresh.tldr.clone()))
            .post_type(original.post_type.clone())
            .category(original.category.clone())
            .capacity_status(original.capacity_status.clone())
            .location(fresh.location.clone())
            .source_language(original.source_language.clone())
            .submission_type(Some("revision".to_string()))
            .website_id(original.website_id)
            .source_url(original.source_url.clone())
            .organization_id(original.organization_id)
            .revision_of_post_id(Some(post_id))
            .build(),
        pool,
    )
    .await?;

    // Create contact info on revision if available
    if let Some(ref contact) = fresh.contact {
        let contact_json = serde_json::json!({
            "phone": contact.phone,
            "email": contact.email,
            "website": contact.website
        });
        if let Err(e) = PostContact::create_from_json(revision.id, &contact_json, pool).await {
            tracing::warn!(
                revision_id = %revision.id,
                error = %e,
                "Failed to create contact info on revision"
            );
        }
    }

    Ok(())
}

const SYNC_SYSTEM_PROMPT: &str = r#"You are synchronizing freshly extracted posts with existing database posts.

## Your Task

Compare the fresh posts (just extracted from the website) with existing posts (in the database).
Determine which operations are needed:

1. **INSERT**: Fresh post is NEW - doesn't match any existing post
2. **UPDATE**: Fresh post MATCHES an existing post - update the existing with fresh data
3. **DELETE**: Existing post has NO MATCH in fresh extraction - the content no longer exists on website
4. **MERGE**: Multiple existing posts are DUPLICATES - consolidate into one with COMBINED content

## Matching Rules

Two posts MATCH (same identity) if they:
- Describe the SAME program/service
- Target the SAME audience (recipient vs volunteer vs donor = DIFFERENT posts)
- Have semantically similar titles (ignore minor wording differences)

Examples of MATCHES:
- "Food Shelf" ↔ "Food Pantry" (same service, different names)
- "Mardi Gras Fundraiser Event" ↔ "Mardi Gras Fundraising Event" (same event)

Examples of NON-MATCHES:
- "Food Shelf" ↔ "Food Shelf - Volunteer" (different audiences)
- "Legal Aid" ↔ "Housing Assistance" (different services)

## MERGE Content Rules

When merging duplicates, CREATE BETTER COMBINED CONTENT:
- Pick the BEST title (clearest, most descriptive)
- COMBINE descriptions - include useful details from ALL duplicates
- Don't lose information - if one duplicate has contact info another lacks, include it

## Important Rules

1. **BE VERY CONSERVATIVE WITH DELETE**: Only DELETE if you're CERTAIN the program/service was removed from the website. If unsure, DO NOT DELETE. It's better to keep an extra post than lose a valid one.
2. **Active and pending posts are protected**: Never DELETE posts with status "active" or "pending_approval"
3. **Prefer UPDATE over INSERT+DELETE**: If fresh matches existing, UPDATE it
4. **Merge content intelligently**: When merging, combine the best parts of each duplicate
5. **If fresh posts << existing posts**: This usually means extraction was incomplete. Prefer UPDATE/INSERT over DELETE in this case.

## Output Order

1. MERGE operations first (consolidate duplicates with combined content)
2. UPDATE operations (refresh existing posts)
3. INSERT operations (add new posts)
4. DELETE operations ONLY if certain (remove truly stale posts)"#;

const SYNC_SCHEMA: &str = r#"EXACT structure required (use lowercase operation names):

{
  "operations": [
    {"operation": "insert", "fresh_id": "fresh_1"},
    {"operation": "update", "fresh_id": "fresh_2", "existing_id": "550e8400-e29b-41d4-a716-446655440000"},
    {"operation": "delete", "existing_id": "550e8400-e29b-41d4-a716-446655440000", "reason": "No longer on website"},
    {"operation": "merge", "canonical_id": "550e8400-e29b-41d4-a716-446655440000", "duplicate_ids": ["6ba7b810-9dad-11d1-80b4-00c04fd430c8"], "merged_title": "Best combined title", "merged_description": "Combined description with details from all duplicates", "reason": "Duplicate entries for same service"}
  ],
  "summary": "1 insert, 1 merge"
}

CRITICAL RULES:
1. Use LOWERCASE operation names: "insert", "update", "delete", "merge"
2. For fresh_id: Use EXACT values from Fresh Posts (e.g., "fresh_1", "fresh_2") - do NOT invent IDs
3. For existing_id/canonical_id/duplicate_ids: Use EXACT UUIDs from Existing Posts (the "id" field) - NEVER use placeholders like "uuid-123" or made-up IDs
4. For MERGE: provide merged_title and merged_description with COMBINED content from all duplicates"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_operation_deserialize() {
        let json = r#"{"operation": "insert", "fresh_id": "fresh_1"}"#;
        let op: SyncOperation = serde_json::from_str(json).unwrap();
        assert!(matches!(op, SyncOperation::Insert { fresh_id } if fresh_id == "fresh_1"));

        let json = r#"{"operation": "update", "fresh_id": "fresh_2", "existing_id": "abc-123"}"#;
        let op: SyncOperation = serde_json::from_str(json).unwrap();
        assert!(matches!(op, SyncOperation::Update { .. }));

        let json =
            r#"{"operation": "delete", "existing_id": "abc-123", "reason": "Removed from site"}"#;
        let op: SyncOperation = serde_json::from_str(json).unwrap();
        assert!(matches!(op, SyncOperation::Delete { .. }));

        let json = r#"{
            "operation": "merge",
            "canonical_id": "abc-123",
            "duplicate_ids": ["def-456"],
            "reason": "Same event"
        }"#;
        let op: SyncOperation = serde_json::from_str(json).unwrap();
        assert!(matches!(op, SyncOperation::Merge { .. }));
    }

    #[test]
    fn test_sync_analysis_deserialize_full() {
        let json = r#"{
            "operations": [
                { "operation": "insert", "fresh_id": "fresh_1" },
                { "operation": "update", "fresh_id": "fresh_2", "existing_id": "abc-123" }
            ],
            "summary": "1 new post, 1 update"
        }"#;
        let response: SyncAnalysisResponse = serde_json::from_str(json).unwrap();
        let (operations, summary) = response.into_parts();
        assert_eq!(operations.len(), 2);
        assert_eq!(summary, "1 new post, 1 update");
    }

    #[test]
    fn test_sync_analysis_deserialize_array_only() {
        // LLM sometimes returns just an array without the wrapper
        let json = r#"[
            { "operation": "insert", "fresh_id": "fresh_1" },
            { "operation": "MERGE", "canonical_id": "abc", "duplicate_ids": ["def"], "reason": "dupes" }
        ]"#;
        let response: SyncAnalysisResponse = serde_json::from_str(json).unwrap();
        let (operations, _summary) = response.into_parts();
        assert_eq!(operations.len(), 2);
    }
}
