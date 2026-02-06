//! Crawl website workflow
//!
//! Multi-step durable workflow:
//! 1. Ingest website pages (Firecrawl or HTTP)
//! 2. Extract narratives from pages
//! 3. Investigate posts in parallel (with AI)
//! 4. Sync and deduplicate to database

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request to crawl a website
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlWebsiteRequest {
    pub website_id: Uuid,
    pub visitor_id: Uuid,
    pub use_firecrawl: bool,
}

/// Result of crawl workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlWebsiteResult {
    pub website_id: Uuid,
    pub posts_synced: usize,
    pub status: String,
}

/// Crawl website workflow service
#[restate_sdk::service]
#[name = "CrawlWebsite"]
pub trait CrawlWebsiteWorkflow {
    async fn run(request: CrawlWebsiteRequest) -> Result<CrawlWebsiteResult, HandlerError>;
}

pub struct CrawlWebsiteWorkflowImpl;

#[restate_sdk::service]
impl CrawlWebsiteWorkflow for CrawlWebsiteWorkflowImpl {
    async fn run(
        &self,
        _ctx: Context,
        request: CrawlWebsiteRequest,
    ) -> Result<CrawlWebsiteResult, HandlerError> {
        tracing::info!(
            website_id = %request.website_id,
            visitor_id = %request.visitor_id,
            "Starting crawl workflow (not yet implemented)"
        );

        // TODO: Implement workflow steps using activities from domains/crawling/activities/

        Ok(CrawlWebsiteResult {
            website_id: request.website_id,
            posts_synced: 0,
            status: "not_implemented".to_string(),
        })
    }
}
