//! Regenerate posts workflow for social sources
//!
//! Long-running workflow that scrapes a social media source via Apify,
//! preserves original captions verbatim, and extracts metadata via LLM.
//!
//! Flow:
//! 1. Scrape social posts via Apify → Vec<ScrapedSocialPost>
//! 2. Extract metadata only (title, summary, contacts, etc.) — caption preserved as description
//! 3. LLM sync proposals for admin review
//! 4. Generate notes from social content

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::{EmptyRequest, OrganizationId};
use crate::domains::notes::activities::{attach_notes_to_org_posts, extract_and_create_notes, SourceContent};
use crate::domains::organization::models::Organization;
use crate::domains::posts::activities::llm_sync::llm_sync_posts;
use crate::domains::source::activities::extract_social::extract_posts_from_social;
use crate::domains::source::activities::scrape_social::scrape_social_source;
use crate::domains::tag::models::tag_kind_config::build_tag_instructions;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request / Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegenerateSocialPostsRequest {
    pub source_id: Uuid,
}

impl_restate_serde!(RegenerateSocialPostsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegenerateSocialPostsResult {
    pub proposals_staged: i32,
    pub status: String,
}

impl_restate_serde!(RegenerateSocialPostsResult);

// =============================================================================
// Workflow definition
// =============================================================================

#[restate_sdk::workflow]
#[name = "RegenerateSocialPostsWorkflow"]
pub trait RegenerateSocialPostsWorkflow {
    async fn run(
        req: RegenerateSocialPostsRequest,
    ) -> Result<RegenerateSocialPostsResult, HandlerError>;

    #[shared]
    async fn get_status(req: EmptyRequest) -> Result<String, HandlerError>;
}

pub struct RegenerateSocialPostsWorkflowImpl {
    deps: Arc<ServerDeps>,
}

impl RegenerateSocialPostsWorkflowImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl RegenerateSocialPostsWorkflow for RegenerateSocialPostsWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        req: RegenerateSocialPostsRequest,
    ) -> Result<RegenerateSocialPostsResult, HandlerError> {
        info!(source_id = %req.source_id, "Starting regenerate social posts workflow");

        let pool = &self.deps.db_pool;

        // Build tag instructions from all active tag kinds
        let tag_instructions = build_tag_instructions(pool)
            .await
            .unwrap_or_default();

        // Phase 1: Scrape social source via Apify
        ctx.set("status", "Scraping social media posts...".to_string());

        let social_posts = match scrape_social_source(req.source_id, &self.deps).await {
            Ok(p) => p,
            Err(e) => {
                let msg = format!("Failed to scrape social source: {}", e);
                warn!(source_id = %req.source_id, error = %e, "Social scrape failed");
                ctx.set("status", msg);
                return Ok(RegenerateSocialPostsResult {
                    proposals_staged: 0,
                    status: "failed".to_string(),
                });
            }
        };

        if social_posts.is_empty() {
            ctx.set(
                "status",
                "No recent posts found for this social source.".to_string(),
            );
            return Ok(RegenerateSocialPostsResult {
                proposals_staged: 0,
                status: "no_pages".to_string(),
            });
        }

        // Phase 2: Extract metadata from social posts (caption preserved verbatim)
        ctx.set(
            "status",
            format!("Extracting metadata from {} social media posts...", social_posts.len()),
        );

        let posts = match extract_posts_from_social(
            &social_posts,
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
                return Ok(RegenerateSocialPostsResult {
                    proposals_staged: 0,
                    status: "failed".to_string(),
                });
            }
        };

        if posts.is_empty() {
            ctx.set(
                "status",
                "No posts extracted from social media content.".to_string(),
            );
            return Ok(RegenerateSocialPostsResult {
                proposals_staged: 0,
                status: "no_posts".to_string(),
            });
        }

        // Phase 3: LLM sync — creates proposals for admin review
        ctx.set(
            "status",
            format!("Analyzing {} posts...", posts.len()),
        );

        // Determine source_type from the source record
        let source = crate::domains::source::models::Source::find_by_id(
            crate::common::SourceId::from_uuid(req.source_id),
            pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        let sync_result = match llm_sync_posts(
            &source.source_type,
            req.source_id,
            posts,
            self.deps.ai_next.as_ref(),
            pool,
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                let msg = format!("LLM sync failed: {}", e);
                warn!(source_id = %req.source_id, error = %e, "LLM sync failed");
                ctx.set("status", msg);
                return Ok(RegenerateSocialPostsResult {
                    proposals_staged: 0,
                    status: "failed".to_string(),
                });
            }
        };

        let total_proposals = (sync_result.staged_inserts
            + sync_result.staged_updates
            + sync_result.staged_deletes
            + sync_result.staged_merges) as i32;

        // Phase 4: Generate notes for the organization using already-scraped content (best-effort)
        if let Some(org_uuid) = source.organization_id {
            let org_id = OrganizationId::from(org_uuid);
            ctx.set("status", "Generating notes...".to_string());
            match Organization::find_by_id(org_id, pool).await {
                Ok(org) => {
                    // Convert ScrapedSocialPosts to CachedPages, then to SourceContent for note extraction
                    let pages: Vec<_> = social_posts.iter().map(|p| p.to_cached_page()).collect();
                    let social_content: Vec<SourceContent> = pages.iter().map(|page| {
                        SourceContent {
                            source_id: uuid::Uuid::new_v4(),
                            source_type: source.source_type.clone(),
                            source_url: page.url.clone(),
                            content: page.content.clone(),
                        }
                    }).collect();

                    match extract_and_create_notes(org_id, &org.name, social_content, &self.deps).await {
                        Ok(r) => {
                            info!(org_id = %org_id, notes_created = r.notes_created, "Note generation complete");
                            if let Err(e) = attach_notes_to_org_posts(org_id, &self.deps).await {
                                warn!(org_id = %org_id, error = %e, "Failed to attach notes to posts");
                            }
                        }
                        Err(e) => warn!(org_id = %org_id, error = %e, "Note generation failed (non-blocking)"),
                    }
                }
                Err(e) => warn!(org_id = %org_uuid, error = %e, "Failed to load org for note generation"),
            }
        }

        let result = RegenerateSocialPostsResult {
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
            "Regenerate social posts workflow completed"
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
