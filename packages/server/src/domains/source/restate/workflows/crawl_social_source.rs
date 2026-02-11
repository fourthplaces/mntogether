//! Crawl social source workflow
//!
//! Scrape-only workflow that ingests social media content into extraction_pages.
//! Does NOT extract posts or run LLM sync — that happens at the org level
//! via ExtractOrgPostsWorkflow.
//!
//! Flow:
//! 1. Scrape social posts via platform Ingestor (Apify)
//! 2. Store as CachedPages in extraction_pages
//! 3. Done — org-level extraction picks up the content later

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::EmptyRequest;
use crate::domains::source::activities::ingest_social::ingest_social_source;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request / Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlSocialSourceRequest {
    pub source_id: Uuid,
}

impl_restate_serde!(CrawlSocialSourceRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlSocialSourceResult {
    pub pages_stored: usize,
    pub status: String,
}

impl_restate_serde!(CrawlSocialSourceResult);

// =============================================================================
// Workflow definition
// =============================================================================

#[restate_sdk::workflow]
#[name = "CrawlSocialSourceWorkflow"]
pub trait CrawlSocialSourceWorkflow {
    async fn run(req: CrawlSocialSourceRequest) -> Result<CrawlSocialSourceResult, HandlerError>;

    #[shared]
    async fn get_status(req: EmptyRequest) -> Result<String, HandlerError>;
}

pub struct CrawlSocialSourceWorkflowImpl {
    deps: Arc<ServerDeps>,
}

impl CrawlSocialSourceWorkflowImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl CrawlSocialSourceWorkflow for CrawlSocialSourceWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        req: CrawlSocialSourceRequest,
    ) -> Result<CrawlSocialSourceResult, HandlerError> {
        info!(source_id = %req.source_id, "Starting crawl social source workflow");

        // Scrape and store to extraction_pages (durable)
        ctx.set("status", "Scraping social media posts...".to_string());

        let result = ctx
            .run(|| async {
                let r = ingest_social_source(req.source_id, &self.deps)
                    .await
                    .map_err(|e| restate_sdk::errors::TerminalError::new(e.to_string()))?;
                Ok(r.pages_stored as u64)
            })
            .await;

        match result {
            Ok(pages_stored) => {
                let pages_stored = pages_stored as usize;
                let msg = format!(
                    "Completed: {} pages stored in extraction_pages",
                    pages_stored
                );
                ctx.set("status", msg);

                info!(
                    source_id = %req.source_id,
                    pages_stored = pages_stored,
                    "Crawl social source workflow completed"
                );

                Ok(CrawlSocialSourceResult {
                    pages_stored,
                    status: "completed".to_string(),
                })
            }
            Err(e) => {
                let msg = format!("Failed to ingest social source: {}", e);
                warn!(source_id = %req.source_id, error = %e, "Social ingest failed");
                ctx.set("status", msg);

                Ok(CrawlSocialSourceResult {
                    pages_stored: 0,
                    status: "failed".to_string(),
                })
            }
        }
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
