//! Organization-level post extraction workflow
//!
//! Pools all raw content from all sources for an organization, runs the
//! 3-pass extraction pipeline, and LLM syncs at the org level.
//!
//! Flow:
//! 1. Load org + active sources
//! 2. Resolve site_url for each source
//! 3. Query extraction_pages for all site_urls
//! 4. Run 3-pass extraction (narrative → dedupe → investigate)
//! 5. LLM sync at org level (compare to org's existing posts)
//! 6. Update organizations.last_extracted_at
//! 7. Generate notes (best-effort)

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::{EmptyRequest, OrganizationId};
use crate::domains::crawling::activities::post_extraction::extract_posts_from_pages_with_tags;
use crate::domains::notes::activities::{attach_notes_to_org_posts, generate_notes_for_organization};
use crate::domains::organization::models::Organization;
use crate::domains::posts::activities::llm_sync::llm_sync_posts_for_org;
use crate::domains::source::models::Source;
use crate::domains::tag::models::tag_kind_config::build_tag_instructions;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request / Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractOrgPostsRequest {
    pub organization_id: Uuid,
}

impl_restate_serde!(ExtractOrgPostsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractOrgPostsResult {
    pub proposals_staged: i32,
    pub pages_pooled: usize,
    pub sources_included: usize,
    pub status: String,
}

impl_restate_serde!(ExtractOrgPostsResult);

// =============================================================================
// Workflow definition
// =============================================================================

#[restate_sdk::workflow]
#[name = "ExtractOrgPostsWorkflow"]
pub trait ExtractOrgPostsWorkflow {
    async fn run(req: ExtractOrgPostsRequest) -> Result<ExtractOrgPostsResult, HandlerError>;

    #[shared]
    async fn get_status(req: EmptyRequest) -> Result<String, HandlerError>;
}

pub struct ExtractOrgPostsWorkflowImpl {
    deps: Arc<ServerDeps>,
}

impl ExtractOrgPostsWorkflowImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl ExtractOrgPostsWorkflow for ExtractOrgPostsWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        req: ExtractOrgPostsRequest,
    ) -> Result<ExtractOrgPostsResult, HandlerError> {
        info!(organization_id = %req.organization_id, "Starting extract org posts workflow");

        let pool = &self.deps.db_pool;
        let org_id = OrganizationId::from(req.organization_id);

        // Phase 1: Load org + active sources (durable)
        ctx.set("status", "Gathering sources...".to_string());

        let org = Organization::find_by_id(org_id, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let all_sources = Source::find_by_organization(org_id, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let active_sources: Vec<_> = all_sources
            .into_iter()
            .filter(|s| s.status == "approved" && s.active)
            .collect();

        if active_sources.is_empty() {
            ctx.set("status", "No active sources for this organization.".to_string());
            return Ok(ExtractOrgPostsResult {
                proposals_staged: 0,
                pages_pooled: 0,
                sources_included: 0,
                status: "no_sources".to_string(),
            });
        }

        let sources_count = active_sources.len();

        // Phase 2: Pool all pages from extraction_pages (durable)
        ctx.set(
            "status",
            format!("Pooling content from {} sources...", sources_count),
        );

        let extraction = self.deps.extraction.as_ref().ok_or_else(|| {
            TerminalError::new("Extraction service not configured")
        })?;

        let mut all_pages = Vec::new();
        for source in &active_sources {
            let site_url = match source.site_url(pool).await {
                Ok(url) => url,
                Err(e) => {
                    warn!(source_id = %source.id, error = %e, "Failed to resolve site_url, skipping");
                    continue;
                }
            };
            match extraction.get_pages_for_site(&site_url).await {
                Ok(pages) => all_pages.extend(pages),
                Err(e) => {
                    warn!(source_id = %source.id, site_url = %site_url, error = %e, "Failed to get pages, skipping");
                }
            }
        }

        if all_pages.is_empty() {
            ctx.set("status", "No pages found for any source.".to_string());
            return Ok(ExtractOrgPostsResult {
                proposals_staged: 0,
                pages_pooled: 0,
                sources_included: sources_count,
                status: "no_pages".to_string(),
            });
        }

        let pages_count = all_pages.len();
        info!(
            organization_id = %req.organization_id,
            pages_pooled = pages_count,
            sources = sources_count,
            "Content pooled from all sources"
        );

        // Phase 3: Build tag instructions
        let tag_instructions = build_tag_instructions(pool)
            .await
            .unwrap_or_default();

        // Phase 4: Run 3-pass extraction pipeline (durable — this is the expensive step)
        ctx.set(
            "status",
            format!("Extracting posts from {} pages...", pages_count),
        );

        let posts = match extract_posts_from_pages_with_tags(
            &all_pages,
            &org.name,
            &tag_instructions,
            &self.deps,
        )
        .await
        {
            Ok(p) => p,
            Err(e) => {
                let msg = format!("Extraction failed: {}", e);
                warn!(organization_id = %req.organization_id, error = %e, "Extraction failed");
                ctx.set("status", msg);
                return Ok(ExtractOrgPostsResult {
                    proposals_staged: 0,
                    pages_pooled: pages_count,
                    sources_included: sources_count,
                    status: "failed".to_string(),
                });
            }
        };

        if posts.is_empty() {
            ctx.set("status", "No posts extracted from pages.".to_string());
            return Ok(ExtractOrgPostsResult {
                proposals_staged: 0,
                pages_pooled: pages_count,
                sources_included: sources_count,
                status: "no_posts".to_string(),
            });
        }

        // Phase 5: LLM sync at org level
        ctx.set(
            "status",
            format!("Analyzing {} posts...", posts.len()),
        );

        let sync_result = match llm_sync_posts_for_org(
            req.organization_id,
            posts,
            self.deps.ai.as_ref(),
            pool,
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                let msg = format!("LLM sync failed: {}", e);
                warn!(organization_id = %req.organization_id, error = %e, "LLM sync failed");
                ctx.set("status", msg);
                return Ok(ExtractOrgPostsResult {
                    proposals_staged: 0,
                    pages_pooled: pages_count,
                    sources_included: sources_count,
                    status: "failed".to_string(),
                });
            }
        };

        let total_proposals = (sync_result.staged_inserts
            + sync_result.staged_updates
            + sync_result.staged_deletes
            + sync_result.staged_merges) as i32;

        // Phase 6: Update last_extracted_at
        if let Err(e) = Organization::update_last_extracted(org_id, pool).await {
            warn!(organization_id = %req.organization_id, error = %e, "Failed to update last_extracted_at");
        }

        // Phase 7: Generate notes (best-effort)
        ctx.set("status", "Generating notes...".to_string());
        match generate_notes_for_organization(org_id, &org.name, &self.deps).await {
            Ok(r) => {
                info!(organization_id = %req.organization_id, notes_created = r.notes_created, "Note generation complete");
                if let Err(e) = attach_notes_to_org_posts(org_id, &self.deps).await {
                    warn!(organization_id = %req.organization_id, error = %e, "Failed to attach notes to posts");
                }
            }
            Err(e) => warn!(organization_id = %req.organization_id, error = %e, "Note generation failed (non-blocking)"),
        }

        let result = ExtractOrgPostsResult {
            proposals_staged: total_proposals,
            pages_pooled: pages_count,
            sources_included: sources_count,
            status: "completed".to_string(),
        };

        ctx.set(
            "status",
            format!(
                "Completed: {} proposals from {} pages across {} sources",
                total_proposals, pages_count, sources_count
            ),
        );

        info!(
            organization_id = %req.organization_id,
            proposals_staged = total_proposals,
            pages_pooled = pages_count,
            sources = sources_count,
            "Extract org posts workflow completed"
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
