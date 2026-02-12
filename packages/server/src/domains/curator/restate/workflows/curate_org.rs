use std::sync::Arc;

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use crate::common::{EmptyRequest, OrganizationId};
use crate::domains::curator::models::CuratorAction;
use crate::domains::curator::activities::{
    brief_extraction::extract_briefs_for_org, curator::run_curator,
    org_document::compile_org_document, safety_review::review_and_fix_actions,
    stage_actions::stage_curator_actions, writer::rewrite_post_copy,
};
use crate::domains::organization::models::Organization;
use crate::domains::source::models::Source;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurateOrgRequest {
    pub organization_id: Uuid,
}
impl_restate_serde!(CurateOrgRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurateOrgResult {
    pub batch_id: Option<Uuid>,
    pub actions_proposed: usize,
    pub pages_briefed: usize,
    pub status: String,
}
impl_restate_serde!(CurateOrgResult);

#[restate_sdk::workflow]
#[name = "CurateOrgWorkflow"]
pub trait CurateOrgWorkflow {
    async fn run(req: CurateOrgRequest) -> Result<CurateOrgResult, HandlerError>;
    #[shared]
    async fn get_status(req: EmptyRequest) -> Result<String, HandlerError>;
}

pub struct CurateOrgWorkflowImpl {
    deps: Arc<ServerDeps>,
}

impl CurateOrgWorkflowImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl CurateOrgWorkflow for CurateOrgWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        req: CurateOrgRequest,
    ) -> Result<CurateOrgResult, HandlerError> {
        let pool = &self.deps.db_pool;
        let org_id = req.organization_id;

        info!(organization_id = %org_id, "Starting curator workflow");

        // 1. Load org + sources (idempotent reads, no durability needed)
        ctx.set("status", "Loading organization...".to_string());

        let org = Organization::find_by_id(OrganizationId::from(org_id), pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let sources = Source::find_by_organization(OrganizationId::from(org_id), pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let mut source_urls = Vec::new();
        let mut source_url_map: Vec<(String, Source)> = Vec::new();
        for s in &sources {
            if let Ok(url) = s.site_url(pool).await {
                source_urls.push(url.clone());
                source_url_map.push((url, s.clone()));
            }
        }

        if source_urls.is_empty() {
            return Ok(CurateOrgResult {
                status: "no_sources".into(),
                actions_proposed: 0,
                pages_briefed: 0,
                batch_id: None,
            });
        }

        // 2. Load all extraction pages (idempotent read)
        ctx.set("status", "Gathering crawled pages...".to_string());

        let extraction = self
            .deps
            .extraction
            .as_ref()
            .ok_or_else(|| TerminalError::new("Extraction service not configured"))?;

        let mut pages = Vec::new();
        for url in &source_urls {
            if let Ok(site_pages) = extraction.get_pages_for_site(url).await {
                pages.extend(site_pages);
            }
        }

        if pages.is_empty() {
            return Ok(CurateOrgResult {
                status: "no_pages".into(),
                actions_proposed: 0,
                pages_briefed: 0,
                batch_id: None,
            });
        }

        // 3. Extract page briefs (map step — LLM calls, memo-cached)
        ctx.set("status", format!("Briefing {} pages...", pages.len()));

        let briefs = extract_briefs_for_org(&org.name, &pages, &self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        if briefs.is_empty() {
            return Ok(CurateOrgResult {
                status: "no_useful_content".into(),
                actions_proposed: 0,
                pages_briefed: 0,
                batch_id: None,
            });
        }

        let briefs_count = briefs.len();

        // 4. Compile org document (deterministic, no LLM)
        ctx.set("status", "Compiling org document...".to_string());

        let org_doc = compile_org_document(org_id, &org.name, &briefs, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        info!(
            org = org.name,
            tokens = org_doc.token_estimate,
            briefs = org_doc.briefs_included,
            posts = org_doc.posts_included,
            notes = org_doc.notes_included,
            "Org document compiled"
        );

        // Log the full org document so we can see exactly what the curator reads
        info!(
            org = org.name,
            doc_len = org_doc.content.len(),
            "\n--- ORG DOCUMENT START ---\n{}\n--- ORG DOCUMENT END ---",
            org_doc.content
        );

        // 5. Run curator (single LLM call — the reduce step)
        ctx.set("status", "Curator analyzing...".to_string());

        let mut response = run_curator(&org_doc.content, &self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        if response.actions.is_empty() {
            Organization::update_last_extracted(OrganizationId::from(org_id), pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

            return Ok(CurateOrgResult {
                status: "no_actions_needed".into(),
                actions_proposed: 0,
                pages_briefed: briefs_count,
                batch_id: None,
            });
        }

        // 5.5. Rewrite post copy in parallel (writer pass)
        let posts_to_rewrite: Vec<usize> = response
            .actions
            .iter()
            .enumerate()
            .filter(|(_, a)| a.action_type == "create_post" || a.action_type == "update_post")
            .map(|(i, _)| i)
            .collect();

        if !posts_to_rewrite.is_empty() {
            ctx.set(
                "status",
                format!("Rewriting {} posts...", posts_to_rewrite.len()),
            );

            // Build existing feed context for angle dedup
            let existing_feed = build_existing_feed(&response.actions);
            let org_content = &org_doc.content;

            // Clone actions for the writer calls to avoid borrow conflicts
            let actions_for_writer: Vec<_> = posts_to_rewrite
                .iter()
                .map(|&i| response.actions[i].clone())
                .collect();

            let rewrite_futures: Vec<_> = actions_for_writer
                .iter()
                .map(|action| rewrite_post_copy(action, org_content, &existing_feed, &self.deps))
                .collect();

            let results = futures::future::join_all(rewrite_futures).await;

            // Merge rewritten copy back into actions
            let mut rewritten = 0;
            for (&idx, result) in posts_to_rewrite.iter().zip(results) {
                match result {
                    Ok(copy) => {
                        let action = &mut response.actions[idx];
                        action.title = Some(copy.title);
                        action.summary = Some(copy.summary);
                        action.description = Some(copy.text.clone());
                        action.description_markdown = Some(copy.text);
                        rewritten += 1;
                    }
                    Err(e) => {
                        // Keep the curator's draft copy on failure
                        info!(
                            error = %e,
                            idx = idx,
                            "Writer rewrite failed, keeping draft copy"
                        );
                    }
                }
            }

            info!(rewritten = rewritten, total = posts_to_rewrite.len(), "Writer pass complete");
        }

        // 5.7. Safety review — check for omitted eligibility restrictions, fix or block
        ctx.set("status", "Safety reviewing posts...".to_string());

        let safety_outcome =
            review_and_fix_actions(&mut response.actions, &briefs, &self.deps)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

        info!(
            fixes = safety_outcome.fixes_applied,
            blocked = safety_outcome.posts_blocked,
            remaining = response.actions.len(),
            "Safety review complete"
        );

        if response.actions.is_empty() {
            Organization::update_last_extracted(OrganizationId::from(org_id), pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

            return Ok(CurateOrgResult {
                status: "all_blocked_by_safety".into(),
                actions_proposed: 0,
                pages_briefed: briefs_count,
                batch_id: None,
            });
        }

        // 6. Stage actions as proposals
        let actions_count = response.actions.len();
        ctx.set("status", format!("Staging {} proposals...", actions_count));

        let staging = stage_curator_actions(
            org_id,
            &response.actions,
            &response.org_summary,
            &source_url_map,
            &self.deps,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        // 7. Update last_extracted_at
        Organization::update_last_extracted(OrganizationId::from(org_id), pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        info!(
            org = org.name,
            actions = staging.proposals_staged,
            briefs = briefs_count,
            "Curator workflow completed"
        );

        Ok(CurateOrgResult {
            batch_id: Some(staging.batch_id),
            actions_proposed: staging.proposals_staged,
            pages_briefed: briefs_count,
            status: "completed".into(),
        })
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

/// Build a compact feed of draft titles/summaries for the writer to avoid repeating angles.
/// Includes both the curator's draft posts AND existing posts from the org document.
fn build_existing_feed(actions: &[CuratorAction]) -> String {
    let mut feed = String::new();
    for action in actions {
        if action.action_type != "create_post" && action.action_type != "update_post" {
            continue;
        }
        if let Some(title) = &action.title {
            feed.push_str(&format!("- **{}**", title));
            if let Some(summary) = &action.summary {
                feed.push_str(&format!(": {}", summary));
            }
            feed.push('\n');
        }
    }
    feed
}
