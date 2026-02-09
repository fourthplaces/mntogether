//! Regenerate posts workflow
//!
//! Long-running workflow that re-extracts posts from a website using linked agents.
//! Uses Restate K/V state for progress tracking via a shared `get_status` handler.
//!
//! Each linked agent extracts with its own purpose and tag kinds, then
//! `llm_sync_posts` creates proposals for admin review.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::{EmptyRequest, WebsiteId};
use crate::domains::agents::activities::extract::extract_posts_for_website;
use crate::domains::agents::models::{AgentCuratorConfig, AgentRequiredTagKind, AgentWebsite};
use crate::domains::posts::activities::llm_sync::llm_sync_posts;
use crate::domains::tag::models::tag_kind_config::build_tag_instructions_for_kinds;
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
        info!(website_id = %req.website_id, "Starting regenerate posts workflow");

        let pool = &self.deps.db_pool;
        let website_id = WebsiteId::from_uuid(req.website_id);

        let _website =
            Website::find_by_id(website_id, pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

        // Find linked agents for this website
        let agent_websites = AgentWebsite::find_by_website(req.website_id, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        if agent_websites.is_empty() {
            ctx.set("status", "Failed: No agents linked to this website. Link an agent to enable extraction.".to_string());
            return Ok(RegeneratePostsWorkflowResult {
                proposals_staged: 0,
                status: "failed".to_string(),
            });
        }

        let mut total_proposals: i32 = 0;

        for agent_website in &agent_websites {
            let agent_id = agent_website.agent_id;

            // Load agent curator config
            let curator = match AgentCuratorConfig::find_by_agent(agent_id, pool).await {
                Ok(c) => c,
                Err(e) => {
                    warn!(agent_id = %agent_id, error = %e, "No curator config, skipping agent");
                    continue;
                }
            };

            // Build tag instructions from agent's required tag kinds
            let required_tag_kinds = AgentRequiredTagKind::find_by_agent(agent_id, pool)
                .await
                .unwrap_or_default();
            let tag_kind_ids: Vec<Uuid> = required_tag_kinds.iter().map(|r| r.tag_kind_id).collect();
            let tag_instructions = build_tag_instructions_for_kinds(&tag_kind_ids, pool)
                .await
                .unwrap_or_default();

            // Phase 1: Extract posts using agent's purpose
            ctx.set(
                "status",
                format!("Agent {}: extracting...", curator.purpose.chars().take(50).collect::<String>()),
            );

            let extraction_result = match extract_posts_for_website(
                req.website_id,
                &curator.purpose,
                &tag_instructions,
                &self.deps,
            )
            .await
            {
                Ok(r) => r,
                Err(e) => {
                    warn!(agent_id = %agent_id, error = %e, "Extraction failed for agent, skipping");
                    continue;
                }
            };

            if extraction_result.posts.is_empty() {
                info!(agent_id = %agent_id, "No posts extracted, skipping sync");
                continue;
            }

            // Phase 2: LLM sync — creates proposals for admin review
            ctx.set(
                "status",
                format!("Analyzing {} posts...", extraction_result.posts.len()),
            );

            let sync_result = match llm_sync_posts(
                website_id,
                Some(agent_id),
                extraction_result.posts,
                self.deps.ai.as_ref(),
                pool,
            )
            .await
            {
                Ok(r) => r,
                Err(e) => {
                    warn!(agent_id = %agent_id, error = %e, "LLM sync failed for agent");
                    continue;
                }
            };

            let agent_proposals = (sync_result.staged_inserts
                + sync_result.staged_updates
                + sync_result.staged_deletes
                + sync_result.staged_merges) as i32;

            total_proposals += agent_proposals;

            info!(
                agent_id = %agent_id,
                proposals = agent_proposals,
                "Agent sync complete"
            );
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
            website_id = %req.website_id,
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
