//! Crawl website workflow
//!
//! Durable workflow that orchestrates website crawling:
//! 1. Ingest website pages (Firecrawl or HTTP)
//! 2. Extract narratives from pages
//! 3. Investigate posts in parallel (with AI)
//! 4. Sync and deduplicate to database

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::WebsiteId;
use crate::domains::crawling::activities;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

/// Request to crawl a website
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlWebsiteRequest {
    pub website_id: Uuid,
    pub visitor_id: Uuid,
    pub use_firecrawl: bool,
}

impl_restate_serde!(CrawlWebsiteRequest);

/// Result of crawl workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlWebsiteResult {
    pub website_id: Uuid,
    pub posts_synced: usize,
    pub status: String,
}

impl_restate_serde!(CrawlWebsiteResult);

#[restate_sdk::workflow]
pub trait CrawlWebsiteWorkflow {
    async fn run(request: CrawlWebsiteRequest) -> Result<CrawlWebsiteResult, HandlerError>;
}

pub struct CrawlWebsiteWorkflowImpl {
    pub deps: ServerDeps,
}

impl CrawlWebsiteWorkflowImpl {
    pub fn new(deps: ServerDeps) -> Self {
        Self { deps }
    }
}

impl CrawlWebsiteWorkflow for CrawlWebsiteWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        request: CrawlWebsiteRequest,
    ) -> Result<CrawlWebsiteResult, HandlerError> {
        tracing::info!(
            website_id = %request.website_id,
            visitor_id = %request.visitor_id,
            "Starting crawl website workflow"
        );

        let website_id_typed = WebsiteId::from_uuid(request.website_id);

        // Durable execution - orchestrate the full crawl pipeline
        let result = ctx
            .run(|| async {
                // Call high-level crawl activity that orchestrates all steps
                activities::crawl_website_full(
                    website_id_typed,
                    request.visitor_id,
                    request.use_firecrawl,
                    &self.deps,
                )
                .await
                .map_err(Into::into)
            })
            .await?;

        Ok(result)
    }
}
