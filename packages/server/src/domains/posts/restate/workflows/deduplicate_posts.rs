//! Deduplicate posts workflow
//!
//! Per-website Restate workflow that identifies duplicate pending posts and
//! creates sync proposals for admin review.
//!
//! Phase 1: Merge pending posts among themselves (LLM finds duplicates)
//! Phase 2: Match remaining pending posts against active posts (LLM finds updates)
//!
//! All proposals go into one sync batch for admin review.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::{EmptyRequest, PostId, WebsiteId};
use crate::domains::posts::activities::deduplication::{
    find_duplicate_pending_posts, match_pending_to_active_posts, Phase1Result, Phase2Result,
};
use crate::domains::posts::models::Post;
use crate::domains::sync::activities::{stage_proposals, ProposedOperation};
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Journaling wrapper types
// =============================================================================

/// Wrapper for journaling a created revision post ID
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreatedRevision {
    id: Uuid,
}

impl_restate_serde!(CreatedRevision);

/// Wrapper for journaling the staging result
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StagingDone {
    ok: bool,
}

impl_restate_serde!(StagingDone);

// =============================================================================
// Request / Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicatePostsRequest {
    pub website_id: Uuid,
}

impl_restate_serde!(DeduplicatePostsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicatePostsWorkflowResult {
    pub duplicates_found: i32,
    pub proposals_created: i32,
    pub status: String,
}

impl_restate_serde!(DeduplicatePostsWorkflowResult);

// =============================================================================
// Workflow definition
// =============================================================================

#[restate_sdk::workflow]
#[name = "DeduplicatePostsWorkflow"]
pub trait DeduplicatePostsWorkflow {
    async fn run(
        req: DeduplicatePostsRequest,
    ) -> Result<DeduplicatePostsWorkflowResult, HandlerError>;

    #[shared]
    async fn get_status(req: EmptyRequest) -> Result<String, HandlerError>;
}

pub struct DeduplicatePostsWorkflowImpl {
    deps: Arc<ServerDeps>,
}

impl DeduplicatePostsWorkflowImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl DeduplicatePostsWorkflow for DeduplicatePostsWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        req: DeduplicatePostsRequest,
    ) -> Result<DeduplicatePostsWorkflowResult, HandlerError> {
        let website_id = WebsiteId::from_uuid(req.website_id);
        info!(website_id = %req.website_id, "Starting deduplicate posts workflow");

        // Step 1: Load posts
        ctx.set("status", "Loading posts...".to_string());

        let pending_posts = Post::find_pending_by_website(website_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(format!("Failed to load pending posts: {}", e)))?;

        let active_posts = Post::find_active_only_by_website(website_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(format!("Failed to load active posts: {}", e)))?;

        info!(
            website_id = %req.website_id,
            pending = pending_posts.len(),
            active = active_posts.len(),
            "Posts loaded"
        );

        if pending_posts.is_empty() {
            ctx.set(
                "status",
                "Completed: no pending posts to deduplicate".to_string(),
            );
            return Ok(DeduplicatePostsWorkflowResult {
                duplicates_found: 0,
                proposals_created: 0,
                status: "completed".to_string(),
            });
        }

        // Step 2: Phase 1 — Find duplicates among pending posts (journaled)
        ctx.set(
            "status",
            format!(
                "Analyzing {} pending posts for duplicates...",
                pending_posts.len()
            ),
        );

        let phase1 = ctx
            .run(|| async {
                find_duplicate_pending_posts(&pending_posts, self.deps.ai.as_ref())
                    .await
                    .map(|groups| Phase1Result { groups })
                    .map_err(Into::into)
            })
            .await?;

        info!(
            website_id = %req.website_id,
            groups = phase1.groups.len(),
            "Phase 1 complete"
        );

        // Collect IDs of posts that are duplicates (will be removed from phase 2 input)
        let mut duplicate_pending_ids: std::collections::HashSet<Uuid> =
            std::collections::HashSet::new();
        for group in &phase1.groups {
            for dup_id_str in &group.duplicate_ids {
                if let Ok(id) = Uuid::parse_str(dup_id_str) {
                    duplicate_pending_ids.insert(id);
                }
            }
        }

        // Remaining pending posts for phase 2 (exclude duplicates found in phase 1)
        let remaining_posts_owned: Vec<Post> = pending_posts
            .iter()
            .filter(|p| !duplicate_pending_ids.contains(&p.id.into_uuid()))
            .cloned()
            .collect();

        // Step 3: Phase 2 — Match remaining pending against active (journaled)
        ctx.set(
            "status",
            format!(
                "Matching {} pending posts against {} active posts...",
                remaining_posts_owned.len(),
                active_posts.len()
            ),
        );

        let phase2 = ctx
            .run(|| async {
                match_pending_to_active_posts(
                    &remaining_posts_owned,
                    &active_posts,
                    self.deps.ai.as_ref(),
                )
                .await
                .map(|matches| Phase2Result { matches })
                .map_err(Into::into)
            })
            .await?;

        info!(
            website_id = %req.website_id,
            matches = phase2.matches.len(),
            "Phase 2 complete"
        );

        // Step 4: Create revision posts for phase 2 matches (each journaled)
        if !phase2.matches.is_empty() {
            ctx.set("status", "Creating revision drafts...".to_string());
        }

        let mut revision_map: Vec<(Uuid, Uuid, Uuid)> = Vec::new(); // (pending_id, active_id, revision_id)

        for m in &phase2.matches {
            let pending_post = pending_posts
                .iter()
                .find(|p| p.id.into_uuid() == m.pending_id);

            if let Some(source) = pending_post {
                let active_id = m.active_id;
                let created = ctx
                    .run(|| async {
                        let rev = Post::create_revision_from(
                            source,
                            PostId::from(active_id),
                            &self.deps.db_pool,
                        )
                        .await
                        .map_err(|e| -> HandlerError {
                            TerminalError::new(format!("Failed to create revision: {}", e)).into()
                        })?;
                        Ok(CreatedRevision {
                            id: rev.id.into_uuid(),
                        })
                    })
                    .await?;

                revision_map.push((m.pending_id, m.active_id, created.id));
            } else {
                warn!(
                    pending_id = %m.pending_id,
                    "Pending post from phase 2 match not found"
                );
            }
        }

        // Step 5: Build and stage all proposals (journaled)
        ctx.set("status", "Staging proposals for review...".to_string());

        let mut proposed_ops: Vec<ProposedOperation> = Vec::new();

        // Phase 1 proposals: merge groups
        for group in &phase1.groups {
            let canonical_uuid = match Uuid::parse_str(&group.canonical_id) {
                Ok(u) => u,
                Err(_) => continue,
            };

            let mut merge_source_ids = Vec::new();
            for dup_id_str in &group.duplicate_ids {
                if let Ok(uuid) = Uuid::parse_str(dup_id_str) {
                    merge_source_ids.push(uuid);
                }
            }

            // If LLM provided merged content, create a revision for the canonical post
            let draft_id =
                if group.merged_title.is_some() || group.merged_description.is_some() {
                    let canonical_post = pending_posts
                        .iter()
                        .find(|p| p.id.into_uuid() == canonical_uuid);
                    if let Some(canonical) = canonical_post {
                        let mut source = canonical.clone();
                        if let Some(ref t) = group.merged_title {
                            source.title = t.clone();
                        }
                        if let Some(ref d) = group.merged_description {
                            source.description = d.clone();
                        }

                        match ctx
                            .run(|| async {
                                let rev = Post::create_revision_from(
                                    &source,
                                    PostId::from(canonical_uuid),
                                    &self.deps.db_pool,
                                )
                                .await
                                .map_err(|e| -> HandlerError {
                                    TerminalError::new(format!(
                                        "Failed to create merge revision: {}",
                                        e
                                    ))
                                    .into()
                                })?;
                                Ok(CreatedRevision {
                                    id: rev.id.into_uuid(),
                                })
                            })
                            .await
                        {
                            Ok(rev) => Some(rev.id),
                            Err(e) => {
                                warn!(error = %e, "Failed to create merge revision, continuing");
                                None
                            }
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

            proposed_ops.push(ProposedOperation {
                operation: "merge".to_string(),
                entity_type: "post".to_string(),
                draft_entity_id: draft_id,
                target_entity_id: Some(canonical_uuid),
                reason: Some(group.reasoning.clone()),
                merge_source_ids,
            });
        }

        // Phase 2 proposals: update + delete pairs
        for (pending_id, active_id, revision_id) in &revision_map {
            // Update proposal: revision replaces active post content
            proposed_ops.push(ProposedOperation {
                operation: "update".to_string(),
                entity_type: "post".to_string(),
                draft_entity_id: Some(*revision_id),
                target_entity_id: Some(*active_id),
                reason: Some(
                    phase2
                        .matches
                        .iter()
                        .find(|m| m.pending_id == *pending_id)
                        .map(|m| {
                            format!("Pending post duplicates active post: {}", m.reasoning)
                        })
                        .unwrap_or_else(|| "Pending post duplicates active post".to_string()),
                ),
                merge_source_ids: vec![],
            });

            // Delete proposal: remove the original pending post (superseded by revision)
            proposed_ops.push(ProposedOperation {
                operation: "delete".to_string(),
                entity_type: "post".to_string(),
                draft_entity_id: None,
                target_entity_id: Some(*pending_id),
                reason: Some("Superseded by update to active post".to_string()),
                merge_source_ids: vec![],
            });
        }

        let total_duplicates = phase1
            .groups
            .iter()
            .map(|g| g.duplicate_ids.len())
            .sum::<usize>()
            + phase2.matches.len();
        let total_proposals = proposed_ops.len();

        // Stage all proposals in one batch
        let summary = format!(
            "Deduplication: {} merge groups, {} active matches, {} total proposals",
            phase1.groups.len(),
            phase2.matches.len(),
            total_proposals
        );

        ctx.run(|| async {
            stage_proposals(
                "post",
                req.website_id,
                Some(&summary),
                proposed_ops,
                &self.deps.db_pool,
            )
            .await
            .map(|_| StagingDone { ok: true })
            .map_err(Into::into)
        })
        .await?;

        let result = DeduplicatePostsWorkflowResult {
            duplicates_found: total_duplicates as i32,
            proposals_created: total_proposals as i32,
            status: "completed".to_string(),
        };

        ctx.set(
            "status",
            format!(
                "Completed: {} duplicates found, {} proposals created",
                result.duplicates_found, result.proposals_created
            ),
        );

        info!(
            website_id = %req.website_id,
            duplicates_found = result.duplicates_found,
            proposals_created = result.proposals_created,
            "Deduplicate posts workflow completed"
        );

        Ok(result)
    }

    async fn get_status(
        &self,
        ctx: SharedWorkflowContext<'_>,
        _req: EmptyRequest,
    ) -> Result<String, HandlerError> {
        Ok(ctx
            .get::<String>("status")
            .await?
            .unwrap_or_else(|| "pending".to_string()))
    }
}
