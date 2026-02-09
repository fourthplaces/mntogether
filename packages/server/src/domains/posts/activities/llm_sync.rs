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
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

use crate::common::{ExtractedPost, PostId, SyncBatchId, WebsiteId};
use crate::domains::contacts::Contact;
use crate::domains::posts::models::{CreatePost, Post, UpdatePostContent};
use crate::domains::sync::activities::{stage_proposals, ProposedOperation};

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
    /// Dynamic tags from AI extraction, keyed by tag kind slug.
    #[serde(default)]
    pub tags: HashMap<String, Vec<String>>,
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

/// LLM's decision for a single sync operation.
///
/// Flat struct (not a tagged enum) for maximum compatibility with OpenAI structured output.
/// The `operation` field discriminates: "insert", "update", "delete", or "merge".
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SyncOperation {
    /// Operation type: "insert", "update", "delete", or "merge"
    pub operation: String,
    /// For insert/update: the temporary ID of the fresh post (e.g., "fresh_1")
    pub fresh_id: Option<String>,
    /// For update/delete/merge: the UUID of the existing post
    pub existing_id: Option<String>,
    /// For update: whether to merge descriptions (default: false)
    pub merge_description: Option<bool>,
    /// For merge: the post to keep
    pub canonical_id: Option<String>,
    /// For merge: posts to merge into canonical
    pub duplicate_ids: Option<Vec<String>>,
    /// For merge: improved title
    pub merged_title: Option<String>,
    /// For merge: improved description
    pub merged_description: Option<String>,
    /// Reason for delete or merge
    pub reason: Option<String>,
}

/// Result of LLM sync analysis — structured output from OpenAI
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SyncAnalysisResponse {
    pub operations: Vec<SyncOperation>,
    pub summary: String,
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
            tags: p.tags.clone(),
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
        let contacts = Contact::find_by_post(p.id, pool)
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
        let op_type = op.operation.as_str();
        match op_type {
            "insert" => {
                if let Some(fresh_id) = &op.fresh_id {
                    if let Some(f) = fresh.iter().find(|f| &f.temp_id == fresh_id) {
                        info!(op = "INSERT", fresh_id = %fresh_id, title = %f.title, "LLM decision: insert new post");
                    }
                }
            }
            "update" => {
                let fresh_id = op.fresh_id.as_deref().unwrap_or("?");
                let existing_id = op.existing_id.as_deref().unwrap_or("?");
                let merge_desc = op.merge_description.unwrap_or(false);
                let fresh_title = fresh
                    .iter()
                    .find(|f| f.temp_id == fresh_id)
                    .map(|f| f.title.as_str())
                    .unwrap_or("?");
                let existing_title = existing
                    .iter()
                    .find(|e| e.id == existing_id)
                    .map(|e| e.title.as_str())
                    .unwrap_or("?");
                info!(op = "UPDATE", fresh_id = %fresh_id, existing_id = %existing_id, fresh_title = %fresh_title, existing_title = %existing_title, merge_description = %merge_desc, "LLM decision: update existing post");
            }
            "delete" => {
                let existing_id = op.existing_id.as_deref().unwrap_or("?");
                let reason = op.reason.as_deref().unwrap_or("");
                let existing_title = existing
                    .iter()
                    .find(|e| e.id == existing_id)
                    .map(|e| e.title.as_str())
                    .unwrap_or("?");
                info!(op = "DELETE", existing_id = %existing_id, existing_title = %existing_title, reason = %reason, "LLM decision: delete stale post");
            }
            "merge" => {
                let canonical_id = op.canonical_id.as_deref().unwrap_or("?");
                let dup_ids = op.duplicate_ids.as_deref().unwrap_or(&[]);
                let reason = op.reason.as_deref().unwrap_or("");
                let canonical_title = existing
                    .iter()
                    .find(|e| e.id == canonical_id)
                    .map(|e| e.title.as_str())
                    .unwrap_or("?");
                info!(op = "MERGE", canonical_id = %canonical_id, canonical_title = %canonical_title, duplicate_count = dup_ids.len(), reason = %reason, "LLM decision: merge duplicates");
            }
            other => {
                tracing::warn!(op = %other, "Unknown sync operation type from LLM");
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
    let submitted_by_id: Option<Uuid> = None;
    let source_id = website_id.into_uuid();

    info!(website_id = %website_id, fresh_count = fresh_posts.len(), existing_count = existing_db_posts.len(), "Starting LLM sync analysis");

    if fresh_posts.is_empty() && existing_db_posts.is_empty() {
        // Create an empty batch so callers always get a batch_id
        let stage_result = stage_proposals(
            "post",
            source_id,
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
        .extract("gpt-4o", SYNC_SYSTEM_PROMPT, &user_prompt)
        .await
        .map_err(|e| anyhow::anyhow!("LLM sync analysis failed: {}", e))?;

    let mut operations = response.operations;
    let summary = response.summary;
    info!(website_id = %website_id, operations_count = operations.len(), summary = %summary, "LLM sync analysis complete");

    log_sync_operations(&operations, &fresh, &existing);

    // Safety net: auto-INSERT any fresh posts not referenced by any operation.
    // LLMs sometimes skip posts (especially on retry after a parse failure).
    let referenced_fresh_ids: std::collections::HashSet<String> = operations
        .iter()
        .filter_map(|op| match op.operation.as_str() {
            "insert" | "update" => op.fresh_id.clone(),
            _ => None,
        })
        .collect();

    let all_fresh_ids: Vec<String> = (1..=fresh_posts.len())
        .map(|i| format!("fresh_{}", i))
        .collect();

    let missing: Vec<&String> = all_fresh_ids
        .iter()
        .filter(|id| !referenced_fresh_ids.contains(*id))
        .collect();

    if !missing.is_empty() {
        let missing_titles: Vec<&str> = missing
            .iter()
            .filter_map(|id| {
                fresh.iter().find(|f| &f.temp_id == *id).map(|f| f.title.as_str())
            })
            .collect();
        tracing::warn!(
            missing_count = missing.len(),
            missing_ids = ?missing,
            missing_titles = ?missing_titles,
            "LLM skipped fresh posts — auto-inserting"
        );
        for id in missing {
            operations.push(SyncOperation {
                operation: "insert".to_string(),
                fresh_id: Some(id.clone()),
                existing_id: None,
                merge_description: None,
                canonical_id: None,
                duplicate_ids: None,
                merged_title: None,
                merged_description: None,
                reason: None,
            });
        }
    }

    stage_sync_operations(
        website_id,
        source_id,
        submitted_by_id,
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
    source_id: Uuid,
    submitted_by_id: Option<Uuid>,
    fresh_posts: &[ExtractedPost],
    existing_posts: &[Post],
    operations: Vec<SyncOperation>,
    summary: &str,
    pool: &PgPool,
) -> Result<LlmSyncResult> {
    use crate::domains::posts::activities::create_post::create_extracted_post;
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
        let op_type = op.operation.as_str();
        match op_type {
            "insert" => {
                let fresh_id = match &op.fresh_id {
                    Some(id) => id.clone(),
                    None => {
                        tracing::warn!("Insert operation missing fresh_id");
                        continue;
                    }
                };
                if let Some(fresh) = fresh_by_id.get(&fresh_id) {
                    let website = match Website::find_by_id(website_id, pool).await {
                        Ok(w) => w,
                        Err(e) => {
                            errors.push(format!("Failed to load website: {}", e));
                            continue;
                        }
                    };

                    match create_extracted_post(
                        fresh,
                        Some(website_id),
                        fresh
                            .source_url
                            .clone()
                            .or_else(|| Some(format!("https://{}", website.domain))),
                        submitted_by_id,
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

            "update" => {
                let fresh_id = op.fresh_id.clone().unwrap_or_default();
                let existing_id = op.existing_id.clone().unwrap_or_default();
                let merge_description = op.merge_description.unwrap_or(false);

                if let (Some(fresh), Some(existing)) =
                    (fresh_by_id.get(&fresh_id), existing_by_id.get(&existing_id))
                {
                    match update_post_with_owner(existing.id, fresh, merge_description, submitted_by_id, pool).await {
                        Ok(()) => {
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

            "delete" => {
                let existing_id = op.existing_id.clone().unwrap_or_default();
                let reason = op.reason.clone().unwrap_or_default();

                if existing_by_id.contains_key(&existing_id) {
                    match Uuid::parse_str(&existing_id) {
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

            "merge" => {
                let canonical_id = op.canonical_id.clone().unwrap_or_default();
                let duplicate_ids = op.duplicate_ids.clone().unwrap_or_default();
                let merged_title = op.merged_title.clone();
                let merged_description = op.merged_description.clone();
                let reason = op.reason.clone().unwrap_or_default();

                if let Some(canonical) = existing_by_id.get(&canonical_id) {
                    let canonical_uuid = match Uuid::parse_str(&canonical_id) {
                        Ok(u) => u,
                        Err(e) => {
                            errors.push(format!("Invalid canonical UUID {}: {}", canonical_id, e));
                            continue;
                        }
                    };

                    let draft_id = if merged_title.is_some() || merged_description.is_some() {
                        let fake_fresh = ExtractedPost {
                            title: merged_title.clone().unwrap_or_else(|| canonical.title.clone()),
                            tldr: canonical.tldr.clone().unwrap_or_default(),
                            description: merged_description.clone()
                                .unwrap_or_else(|| canonical.description.clone()),
                            audience_roles: vec![],
                            location: canonical.location.clone(),
                            contact: None,
                            source_url: canonical.source_url.clone(),
                            urgency: None,
                            confidence: None,
                            source_page_snapshot_id: None,
                            zip_code: None,
                            city: None,
                            state: None,
                            tags: HashMap::new(),
                        };
                        match update_post_with_owner(canonical.id, &fake_fresh, false, submitted_by_id, pool).await {
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

            other => {
                tracing::warn!(op = %other, "Unknown sync operation type, skipping");
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
        source_id,
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
    update_post_with_owner(post_id, fresh, _merge_description, None, pool).await
}

/// Update an existing post by creating a revision, with optional ownership.
pub async fn update_post_with_owner(
    post_id: PostId,
    fresh: &ExtractedPost,
    _merge_description: bool,
    submitted_by_id: Option<Uuid>,
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
            let _ = Contact::delete_all_for_post(existing_revision.id, pool).await;
            let contact_json = serde_json::json!({
                "phone": contact.phone,
                "email": contact.email,
                "website": contact.website
            });
            if let Err(e) =
                Contact::create_from_json_for_post(existing_revision.id, &contact_json, pool).await
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
            .title(fresh.title.clone())
            .description(fresh.description.clone())
            .tldr(Some(fresh.tldr.clone()))
            .post_type(original.post_type.clone())
            .category(original.category.clone())
            .capacity_status(original.capacity_status.clone())
            .location(fresh.location.clone())
            .source_language(original.source_language.clone())
            .submission_type(Some("revision".to_string()))
            .submitted_by_id(submitted_by_id)
            .website_id(original.website_id)
            .source_url(original.source_url.clone())
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
        if let Err(e) = Contact::create_from_json_for_post(revision.id, &contact_json, pool).await {
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

## CRITICAL: Account for EVERY Fresh Post

**Every fresh post MUST appear in exactly one INSERT or UPDATE operation.** If a fresh post doesn't match any existing post, it MUST be INSERTed. Do NOT skip fresh posts.

## MERGE is ONLY for Existing Duplicates

MERGE is ONLY for consolidating existing database posts that are duplicates of EACH OTHER. It uses existing UUIDs only. Do NOT use MERGE to combine a fresh post with an existing post — use UPDATE for that.

## Operation Fields

Each operation object has an "operation" field plus relevant data:
- **insert**: Set `fresh_id` to the fresh post temp_id (e.g., "fresh_1"). Leave other fields null.
- **update**: Set `fresh_id` and `existing_id`. Optionally set `merge_description` to true.
- **delete**: Set `existing_id` and `reason`.
- **merge**: Set `canonical_id`, `duplicate_ids`, optionally `merged_title`/`merged_description`, and `reason`.

## ID Rules

1. Use LOWERCASE operation names: "insert", "update", "delete", "merge"
2. For fresh_id: Use EXACT values from Fresh Posts (e.g., "fresh_1", "fresh_2")
3. For existing_id/canonical_id/duplicate_ids: Use EXACT UUIDs from Existing Posts

## Output Order

1. MERGE operations first (consolidate duplicates with combined content)
2. UPDATE operations (refresh existing posts)
3. INSERT operations (add new posts)
4. DELETE operations ONLY if certain (remove truly stale posts)"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_operation_deserialize() {
        let json = r#"{"operation": "insert", "fresh_id": "fresh_1", "existing_id": null, "merge_description": null, "canonical_id": null, "duplicate_ids": null, "merged_title": null, "merged_description": null, "reason": null}"#;
        let op: SyncOperation = serde_json::from_str(json).unwrap();
        assert_eq!(op.operation, "insert");
        assert_eq!(op.fresh_id.as_deref(), Some("fresh_1"));

        let json = r#"{"operation": "update", "fresh_id": "fresh_2", "existing_id": "abc-123", "merge_description": false, "canonical_id": null, "duplicate_ids": null, "merged_title": null, "merged_description": null, "reason": null}"#;
        let op: SyncOperation = serde_json::from_str(json).unwrap();
        assert_eq!(op.operation, "update");

        let json = r#"{"operation": "delete", "fresh_id": null, "existing_id": "abc-123", "merge_description": null, "canonical_id": null, "duplicate_ids": null, "merged_title": null, "merged_description": null, "reason": "Removed from site"}"#;
        let op: SyncOperation = serde_json::from_str(json).unwrap();
        assert_eq!(op.operation, "delete");

        let json = r#"{"operation": "merge", "fresh_id": null, "existing_id": null, "merge_description": null, "canonical_id": "abc-123", "duplicate_ids": ["def-456"], "merged_title": null, "merged_description": null, "reason": "Same event"}"#;
        let op: SyncOperation = serde_json::from_str(json).unwrap();
        assert_eq!(op.operation, "merge");
        assert_eq!(op.duplicate_ids.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_sync_analysis_deserialize() {
        let json = r#"{
            "operations": [
                { "operation": "insert", "fresh_id": "fresh_1", "existing_id": null, "merge_description": null, "canonical_id": null, "duplicate_ids": null, "merged_title": null, "merged_description": null, "reason": null },
                { "operation": "update", "fresh_id": "fresh_2", "existing_id": "abc-123", "merge_description": false, "canonical_id": null, "duplicate_ids": null, "merged_title": null, "merged_description": null, "reason": null }
            ],
            "summary": "1 new post, 1 update"
        }"#;
        let response: SyncAnalysisResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.operations.len(), 2);
        assert_eq!(response.summary, "1 new post, 1 update");
    }

    #[test]
    fn test_sync_schema_generation() {
        use openai_client::StructuredOutput;
        let schema = SyncAnalysisResponse::openai_schema();
        let schema_str = serde_json::to_string_pretty(&schema).unwrap();
        // Should be an object with operations and summary
        assert!(schema_str.contains("operations"));
        assert!(schema_str.contains("summary"));
        assert!(schema_str.contains("additionalProperties"));
    }
}
