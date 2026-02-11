//! Regenerate posts workflow
//!
//! Long-running workflow that re-extracts posts from any source type using cached
//! extraction pages. Uses Restate K/V state for progress tracking via a shared `get_status` handler.
//!
//! Flow:
//! 1. Load source and its cached pages
//! 2. Extract posts from pages (no agent purpose)
//! 3. LLM sync proposals for admin review

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::{EmptyRequest, SourceId};
use crate::domains::crawling::activities::post_extraction::extract_posts_from_pages_with_tags;
use crate::domains::notes::activities::{attach_notes_to_org_posts, generate_notes_for_organization};
use crate::domains::organization::models::Organization;
use crate::domains::posts::activities::llm_sync::llm_sync_posts;
use crate::domains::source::models::Source;
use crate::domains::tag::models::tag_kind_config::build_tag_instructions;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request / Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegeneratePostsRequest {
    pub source_id: Uuid,
}

impl_restate_serde!(RegeneratePostsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegeneratePostsWorkflowResult {
    pub proposals_staged: i32,
    pub status: String,
}

impl_restate_serde!(RegeneratePostsWorkflowResult);

// =============================================================================
// Workflow definition
// =============================================================================

#[restate_sdk::workflow]
#[name = "RegeneratePostsWorkflow"]
pub trait RegeneratePostsWorkflow {
    async fn run(req: RegeneratePostsRequest)
        -> Result<RegeneratePostsWorkflowResult, HandlerError>;

    #[shared]
    async fn get_status(req: EmptyRequest) -> Result<String, HandlerError>;
}

pub struct RegeneratePostsWorkflowImpl {
    deps: Arc<ServerDeps>,
}

impl RegeneratePostsWorkflowImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl RegeneratePostsWorkflow for RegeneratePostsWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        req: RegeneratePostsRequest,
    ) -> Result<RegeneratePostsWorkflowResult, HandlerError> {
        info!(source_id = %req.source_id, "Starting regenerate posts workflow");

        let pool = &self.deps.db_pool;
        let source_id = SourceId::from_uuid(req.source_id);

        let source =
            Source::find_by_id(source_id, pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

        let site_url = source
            .site_url(pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        // Build tag instructions from all active tag kinds
        let tag_instructions = build_tag_instructions(pool)
            .await
            .unwrap_or_default();

        // Phase 1: Search for pages using the extraction service
        ctx.set("status", "Searching for pages...".to_string());

        let extraction = self.deps.extraction.as_ref().ok_or_else(|| {
            TerminalError::new("Extraction service not configured")
        })?;

        let pages = match extraction
            .get_pages_for_site(&site_url)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                let msg = format!("Failed to load pages: {}", e);
                warn!(source_id = %req.source_id, error = %e, "Page load failed");
                ctx.set("status", msg.clone());
                return Ok(RegeneratePostsWorkflowResult {
                    proposals_staged: 0,
                    status: "failed".to_string(),
                });
            }
        };

        if pages.is_empty() {
            ctx.set("status", "No pages found for this website.".to_string());
            return Ok(RegeneratePostsWorkflowResult {
                proposals_staged: 0,
                status: "no_pages".to_string(),
            });
        }

        // Phase 2: Extract posts using system-level extraction
        ctx.set(
            "status",
            format!("Extracting posts from {} pages...", pages.len()),
        );

        let posts = match extract_posts_from_pages_with_tags(
            &pages,
            &site_url,
            &tag_instructions,
            &self.deps,
        )
        .await
        {
            Ok(p) => p,
            Err(e) => {
                let msg = format!("Extraction failed: {}", e);
                warn!(source_id = %req.source_id, error = %e, "Extraction failed");
                ctx.set("status", msg);
                return Ok(RegeneratePostsWorkflowResult {
                    proposals_staged: 0,
                    status: "failed".to_string(),
                });
            }
        };

        if posts.is_empty() {
            ctx.set("status", "No posts extracted from pages.".to_string());
            return Ok(RegeneratePostsWorkflowResult {
                proposals_staged: 0,
                status: "no_posts".to_string(),
            });
        }

        // Phase 3: LLM sync — creates proposals for admin review
        ctx.set(
            "status",
            format!("Analyzing {} posts...", posts.len()),
        );

        let sync_result = match llm_sync_posts(
            &source.source_type,
            req.source_id,
            posts,
            self.deps.ai.as_ref(),
            pool,
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                let msg = format!("LLM sync failed: {}", e);
                warn!(source_id = %req.source_id, error = %e, "LLM sync failed");
                ctx.set("status", msg);
                return Ok(RegeneratePostsWorkflowResult {
                    proposals_staged: 0,
                    status: "failed".to_string(),
                });
            }
        };

        let total_proposals = (sync_result.staged_inserts
            + sync_result.staged_updates
            + sync_result.staged_deletes
            + sync_result.staged_merges) as i32;

        // Phase 4: Generate notes for the organization (best-effort)
        if let Some(org_id) = source.organization_id {
            ctx.set("status", "Generating notes...".to_string());
            match Organization::find_by_id(org_id, pool).await {
                Ok(org) => {
                    match generate_notes_for_organization(org_id, &org.name, &self.deps).await {
                        Ok(r) => {
                            info!(org_id = %org_id, notes_created = r.notes_created, "Note generation complete");
                            if let Err(e) = attach_notes_to_org_posts(org_id, &self.deps).await {
                                warn!(org_id = %org_id, error = %e, "Failed to attach notes to posts");
                            }
                        }
                        Err(e) => warn!(org_id = %org_id, error = %e, "Note generation failed (non-blocking)"),
                    }
                }
                Err(e) => warn!(org_id = %org_id, error = %e, "Failed to load org for note generation"),
            }
        }

        let result = RegeneratePostsWorkflowResult {
            proposals_staged: total_proposals,
            status: "completed".to_string(),
        };

        ctx.set(
            "status",
            format!("Completed: {} proposals staged for review", total_proposals),
        );

        info!(
            source_id = %req.source_id,
            proposals_staged = total_proposals,
            "Regenerate posts workflow completed"
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
