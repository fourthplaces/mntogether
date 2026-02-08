//! Regenerate posts workflow
//!
//! Long-running workflow that re-extracts posts from a website's crawled pages.
//! Uses Restate K/V state for progress tracking via a shared `get_status` handler.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::{EmptyRequest, ExtractedPostInformation, WebsiteId};
use crate::domains::crawling::activities::post_extraction::{
    extract_narratives_for_domain, investigate_post,
};
use crate::domains::posts::activities::sync_utils::{sync_posts, ExtractedPostInput};
use crate::domains::tag::models::tag_kind_config::build_tag_instructions;
use crate::domains::website::models::Website;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request / Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegeneratePostsRequest {
    pub website_id: Uuid,
}

impl_restate_serde!(RegeneratePostsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegeneratePostsWorkflowResult {
    pub posts_created: i32,
    pub posts_updated: i32,
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
        info!(website_id = %req.website_id, "Starting regenerate posts workflow");

        let website =
            Website::find_by_id(WebsiteId::from_uuid(req.website_id), &self.deps.db_pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

        let extraction = self
            .deps
            .extraction
            .as_ref()
            .ok_or_else(|| TerminalError::new("Extraction service not configured"))?;

        // Phase 1: Extract + deduplicate narratives
        ctx.set("status", "Extracting narratives...".to_string());

        let (narratives, _page_urls) =
            extract_narratives_for_domain(&website.domain, extraction)
                .await
                .map_err(|e| TerminalError::new(format!("Extraction failed: {}", e)))?;

        if narratives.is_empty() {
            ctx.set("status", "Completed: no narratives found".to_string());
            return Ok(RegeneratePostsWorkflowResult {
                posts_created: 0,
                posts_updated: 0,
                status: "completed".to_string(),
            });
        }

        info!(
            website_id = %req.website_id,
            narratives = narratives.len(),
            "Narratives extracted, starting investigation"
        );

        // Build dynamic tag instructions once for all investigations
        let tag_instructions = build_tag_instructions(&self.deps.db_pool)
            .await
            .unwrap_or_default();

        // Phase 2: Investigate each post with progress tracking
        let total = narratives.len();
        let mut post_inputs = Vec::new();

        for (i, narrative) in narratives.iter().enumerate() {
            ctx.set(
                "status",
                format!("Investigating post {}/{}...", i + 1, total),
            );

            let info = match investigate_post(narrative, &tag_instructions, &self.deps).await {
                Ok(i) => i,
                Err(e) => {
                    warn!(
                        title = %narrative.title,
                        error = %e,
                        "Investigation failed, using defaults"
                    );
                    ExtractedPostInformation::default()
                }
            };

            post_inputs.push(ExtractedPostInput {
                title: narrative.title.clone(),
                description: narrative.description.clone(),
                description_markdown: None,
                tldr: Some(narrative.tldr.clone()),
                contact: info.contact_or_none().and_then(|c| serde_json::to_value(c).ok()),
                location: info.location,
                urgency: Some(info.urgency),
                confidence: Some(info.confidence),
                source_url: Some(narrative.source_url.clone()),
                audience_roles: info.audience_roles,
                tags: info.tags,
            });
        }

        // Phase 3: Sync to database
        ctx.set(
            "status",
            format!("Syncing {} posts to database...", post_inputs.len()),
        );

        let sync_result = sync_posts(
            &self.deps.db_pool,
            WebsiteId::from_uuid(req.website_id),
            post_inputs,
        )
        .await
        .map_err(|e| TerminalError::new(format!("Sync failed: {}", e)))?;

        let result = RegeneratePostsWorkflowResult {
            posts_created: sync_result.new_posts.len() as i32,
            posts_updated: sync_result.updated_posts.len() as i32,
            status: "completed".to_string(),
        };

        ctx.set(
            "status",
            format!(
                "Completed: {} created, {} updated",
                result.posts_created, result.posts_updated
            ),
        );

        info!(
            website_id = %req.website_id,
            posts_created = result.posts_created,
            posts_updated = result.posts_updated,
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
