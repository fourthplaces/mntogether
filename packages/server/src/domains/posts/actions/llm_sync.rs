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

use crate::common::{ExtractedPost, PostId, WebsiteId};
use crate::domains::posts::actions::create_extracted_post;
use crate::domains::posts::models::{CreatePost, Post, PostContact, UpdatePostContent};
use crate::domains::website::models::Website;
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

/// Result of applying LLM-based sync operations
#[derive(Debug, Default)]
pub struct LlmSyncResult {
    pub inserted: usize,
    pub updated: usize,
    pub deleted: usize,
    pub merged: usize,
    pub errors: Vec<String>,
}

/// Analyze and sync fresh posts against existing DB posts using LLM
pub async fn llm_sync_posts(
    website_id: WebsiteId,
    fresh_posts: Vec<ExtractedPost>,
    ai: &OpenAIClient,
    pool: &PgPool,
) -> Result<LlmSyncResult> {
    // Load existing posts from DB
    let existing_db_posts = Post::find_by_website_id(website_id, pool).await?;

    info!(
        website_id = %website_id,
        fresh_count = fresh_posts.len(),
        existing_count = existing_db_posts.len(),
        "Starting LLM sync analysis"
    );

    // Log fresh posts being considered
    for (i, post) in fresh_posts.iter().enumerate() {
        info!(
            fresh_id = format!("fresh_{}", i + 1),
            title = %post.title,
            has_contact = post.contact.is_some(),
            "Fresh post for sync"
        );
    }

    // Log existing posts being considered
    for post in &existing_db_posts {
        info!(
            existing_id = %post.id,
            title = %post.title,
            status = %post.status,
            "Existing post for sync"
        );
    }

    // If no fresh posts and no existing posts, nothing to do
    if fresh_posts.is_empty() && existing_db_posts.is_empty() {
        return Ok(LlmSyncResult::default());
    }

    // Convert to LLM-friendly format
    let fresh: Vec<FreshPost> = fresh_posts
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
        .collect();

    // Load contact info for existing posts
    let mut existing: Vec<ExistingPost> = Vec::with_capacity(existing_db_posts.len());
    for p in &existing_db_posts {
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

    // Build prompt
    let fresh_json = serde_json::to_string_pretty(&fresh)?;
    let existing_json = serde_json::to_string_pretty(&existing)?;

    let user_prompt = format!(
        "## Fresh Posts (just extracted from website)\n\n{}\n\n## Existing Posts (currently in database)\n\n{}",
        fresh_json, existing_json
    );

    // Call LLM
    let response: SyncAnalysisResponse = ai
        .request()
        .system(SYNC_SYSTEM_PROMPT)
        .user(&user_prompt)
        .schema_hint(SYNC_SCHEMA)
        .max_retries(3)
        .output()
        .await?;

    let (operations, summary) = response.into_parts();

    info!(
        website_id = %website_id,
        operations_count = operations.len(),
        summary = %summary,
        "LLM sync analysis complete"
    );

    // Log each operation for debugging
    for op in &operations {
        match op {
            SyncOperation::Insert { fresh_id } => {
                if let Some(fresh) = fresh.iter().find(|f| &f.temp_id == fresh_id) {
                    info!(
                        op = "INSERT",
                        fresh_id = %fresh_id,
                        title = %fresh.title,
                        "LLM decision: insert new post"
                    );
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
                info!(
                    op = "UPDATE",
                    fresh_id = %fresh_id,
                    existing_id = %existing_id,
                    fresh_title = %fresh_title,
                    existing_title = %existing_title,
                    merge_description = %merge_description,
                    "LLM decision: update existing post"
                );
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
                info!(
                    op = "DELETE",
                    existing_id = %existing_id,
                    existing_title = %existing_title,
                    reason = %reason,
                    "LLM decision: delete stale post"
                );
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
                info!(
                    op = "MERGE",
                    canonical_id = %canonical_id,
                    canonical_title = %canonical_title,
                    duplicate_count = duplicate_ids.len(),
                    reason = %reason,
                    "LLM decision: merge duplicates"
                );
            }
        }
    }

    // Apply operations
    let result = apply_sync_operations(
        website_id,
        &fresh_posts,
        &existing_db_posts,
        operations,
        pool,
    )
    .await?;

    Ok(result)
}

/// Apply the sync operations to the database
async fn apply_sync_operations(
    website_id: WebsiteId,
    fresh_posts: &[ExtractedPost],
    existing_posts: &[Post],
    operations: Vec<SyncOperation>,
    pool: &PgPool,
) -> Result<LlmSyncResult> {
    let mut result = LlmSyncResult::default();

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
        "Applying sync operations"
    );

    for op in operations {
        match op {
            SyncOperation::Insert { fresh_id } => {
                if let Some(fresh) = fresh_by_id.get(&fresh_id) {
                    info!(
                        action = "INSERTING",
                        fresh_id = %fresh_id,
                        title = %fresh.title,
                        "Inserting new post into database"
                    );
                    match insert_post(website_id, fresh, pool).await {
                        Ok(post_id) => {
                            info!(
                                action = "INSERTED",
                                post_id = %post_id,
                                title = %fresh.title,
                                "Successfully inserted new post"
                            );
                            result.inserted += 1;
                        }
                        Err(e) => {
                            tracing::error!(
                                action = "INSERT_FAILED",
                                fresh_id = %fresh_id,
                                title = %fresh.title,
                                error = %e,
                                "Failed to insert post"
                            );
                            result.errors.push(format!("Insert {}: {}", fresh_id, e));
                        }
                    }
                } else {
                    tracing::warn!(
                        fresh_id = %fresh_id,
                        "Insert operation references unknown fresh_id"
                    );
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
                    info!(
                        action = "UPDATING",
                        post_id = %existing.id,
                        old_title = %existing.title,
                        new_title = %fresh.title,
                        "Updating existing post"
                    );
                    match update_post(existing.id, fresh, merge_description, pool).await {
                        Ok(_) => {
                            info!(
                                action = "UPDATED",
                                post_id = %existing.id,
                                title = %fresh.title,
                                "Successfully updated post"
                            );
                            result.updated += 1;
                        }
                        Err(e) => {
                            tracing::error!(
                                action = "UPDATE_FAILED",
                                post_id = %existing_id,
                                error = %e,
                                "Failed to update post"
                            );
                            result.errors.push(format!("Update {}: {}", existing_id, e));
                        }
                    }
                } else {
                    tracing::warn!(
                        fresh_id = %fresh_id,
                        existing_id = %existing_id,
                        "Update operation references unknown IDs"
                    );
                }
            }

            SyncOperation::Delete {
                existing_id,
                reason,
            } => {
                if let Ok(id) = Uuid::parse_str(&existing_id) {
                    let post_id = PostId::from(id);
                    // Only delete posts that are explicitly rejected or expired
                    // Protect active AND pending_approval posts from accidental deletion
                    if let Some(existing) = existing_by_id.get(&existing_id) {
                        let protected_statuses = ["active", "pending_approval"];
                        if protected_statuses.contains(&existing.status.as_str()) {
                            info!(
                                action = "DELETE_SKIPPED",
                                post_id = %existing_id,
                                title = %existing.title,
                                status = %existing.status,
                                "Skipping delete of protected post (only rejected/expired can be deleted)"
                            );
                            continue;
                        }
                    }
                    let title = existing_by_id
                        .get(&existing_id)
                        .map(|e| e.title.as_str())
                        .unwrap_or("?");
                    info!(
                        action = "DELETING",
                        post_id = %existing_id,
                        title = %title,
                        reason = %reason,
                        "Soft-deleting stale post"
                    );
                    match Post::soft_delete(post_id, &reason, pool).await {
                        Ok(_) => {
                            info!(
                                action = "DELETED",
                                post_id = %existing_id,
                                title = %title,
                                "Successfully soft-deleted post"
                            );
                            result.deleted += 1;
                        }
                        Err(e) => {
                            tracing::error!(
                                action = "DELETE_FAILED",
                                post_id = %existing_id,
                                error = %e,
                                "Failed to delete post"
                            );
                            result.errors.push(format!("Delete {}: {}", existing_id, e));
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
                let canonical_title = existing_by_id
                    .get(&canonical_id)
                    .map(|e| e.title.as_str())
                    .unwrap_or("?");
                info!(
                    action = "MERGING",
                    canonical_id = %canonical_id,
                    canonical_title = %canonical_title,
                    duplicate_count = duplicate_ids.len(),
                    reason = %reason,
                    "Merging duplicate posts"
                );

                // Update canonical with merged content if provided
                if let Ok(id) = Uuid::parse_str(&canonical_id) {
                    let post_id = PostId::from(id);
                    if merged_title.is_some() || merged_description.is_some() {
                        info!(
                            action = "MERGE_UPDATE_CANONICAL",
                            post_id = %canonical_id,
                            new_title = ?merged_title,
                            "Updating canonical post with merged content"
                        );
                        let _ = Post::update_content(
                            UpdatePostContent::builder()
                                .id(post_id)
                                .title(merged_title)
                                .description(merged_description)
                                .build(),
                            pool,
                        )
                        .await;
                    }
                }

                // Delete duplicates
                for dup_id in duplicate_ids {
                    if let Ok(id) = Uuid::parse_str(&dup_id) {
                        let post_id = PostId::from(id);
                        let dup_title = existing_by_id
                            .get(&dup_id)
                            .map(|e| e.title.as_str())
                            .unwrap_or("?");
                        // Don't delete active posts
                        if let Some(existing) = existing_by_id.get(&dup_id) {
                            if existing.status == "active" {
                                info!(
                                    action = "MERGE_SKIP_ACTIVE",
                                    dup_id = %dup_id,
                                    dup_title = %dup_title,
                                    "Skipping merge of active duplicate"
                                );
                                continue;
                            }
                        }
                        info!(
                            action = "MERGE_DELETE_DUP",
                            dup_id = %dup_id,
                            dup_title = %dup_title,
                            canonical_id = %canonical_id,
                            "Deleting duplicate post"
                        );
                        match Post::soft_delete(post_id, &reason, pool).await {
                            Ok(_) => {
                                info!(
                                    action = "MERGE_DELETED",
                                    dup_id = %dup_id,
                                    dup_title = %dup_title,
                                    "Successfully merged (deleted) duplicate"
                                );
                                result.merged += 1;
                            }
                            Err(e) => {
                                tracing::error!(
                                    action = "MERGE_DELETE_FAILED",
                                    dup_id = %dup_id,
                                    error = %e,
                                    "Failed to delete duplicate"
                                );
                                result.errors.push(format!("Merge {}: {}", dup_id, e));
                            }
                        }
                    }
                }
            }
        }
    }

    info!(
        website_id = %website_id,
        inserted = result.inserted,
        updated = result.updated,
        deleted = result.deleted,
        merged = result.merged,
        errors = result.errors.len(),
        "Sync operations complete"
    );

    Ok(result)
}

/// Insert a new post from fresh extraction
async fn insert_post(
    website_id: WebsiteId,
    fresh: &ExtractedPost,
    pool: &PgPool,
) -> Result<PostId> {
    let website = Website::find_by_id(website_id, pool).await?;

    let post = create_extracted_post(
        &website.domain,
        fresh,
        Some(website_id),
        Some(format!("https://{}", website.domain)),
        pool,
    )
    .await?;

    Ok(post.id)
}

/// Update an existing post by creating a revision for review
///
/// Instead of directly modifying the original post, this creates a revision
/// post with `revision_of_post_id` pointing to the original. The original
/// stays untouched until an admin approves the revision.
///
/// If a revision already exists for this post, it gets replaced with the new data.
async fn update_post(
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
            if let Err(e) = PostContact::create_from_json(existing_revision.id, &contact_json, pool).await {
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
