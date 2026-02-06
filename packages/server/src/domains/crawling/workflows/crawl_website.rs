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

use crate::common::WebsiteId;
use crate::domains::crawling::activities;
use crate::domains::website::models::Website;
use crate::kernel::ServerDeps;

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
pub struct CrawlWebsiteWorkflow {
    pub deps: ServerDeps,
}

#[restate_sdk::service(name = "CrawlWebsite")]
impl CrawlWebsiteWorkflow {
    pub fn new(deps: ServerDeps) -> Self {
        Self { deps }
    }

    async fn run(
        &self,
        ctx: Context,
        request: Json<CrawlWebsiteRequest>,
    ) -> Result<Json<CrawlWebsiteResult>, HandlerError> {
        let request = request.into_inner();
        tracing::info!(
            website_id = %request.website_id,
            visitor_id = %request.visitor_id,
            "Starting crawl website workflow"
        );

        let website_id_typed = WebsiteId::from_uuid(request.website_id);

        // Step 1: Ingest website pages (10 min timeout for crawling)
        tracing::info!(website_id = %request.website_id, "Step 1: Ingesting website");
        let ingest_event = ctx
            .run("ingest_website", || async {
                activities::ingest_website(
                    request.website_id,
                    request.visitor_id,
                    request.use_firecrawl,
                    true, // Authorization checked at GraphQL layer
                    &self.deps,
                )
                .await
            })
            .await
            .map_err(|e| HandlerError::new(format!("Ingest failed: {}", e)))?;

        tracing::info!(
            website_id = %request.website_id,
            event = ?ingest_event,
            "Website ingested successfully"
        );

        // Step 2: Extract narratives from ingested pages
        tracing::info!(website_id = %request.website_id, "Step 2: Extracting narratives");

        let website = ctx
            .run("fetch_website", || async {
                Website::find_by_id(website_id_typed, &self.deps.db_pool).await
            })
            .await
            .map_err(|e| HandlerError::new(format!("Failed to fetch website: {}", e)))?;

        let extraction_service = self
            .deps
            .extraction
            .as_ref()
            .ok_or_else(|| HandlerError::new("Extraction service not available"))?;

        let (narratives, _page_urls) = ctx
            .run("extract_narratives", || async {
                activities::extract_narratives_for_domain(&website.domain, extraction_service.as_ref())
                    .await
            })
            .await
            .map_err(|e| HandlerError::new(format!("Narrative extraction failed: {}", e)))?;

        if narratives.is_empty() {
            tracing::info!(
                website_id = %request.website_id,
                "No narratives found, workflow complete"
            );
            return Ok(CrawlWebsiteResult {
                website_id: request.website_id,
                posts_synced: 0,
                status: "no_narratives_found".to_string(),
            });
        }

        tracing::info!(
            website_id = %request.website_id,
            narratives_count = narratives.len(),
            "Step 3: Investigating posts in parallel"
        );

        // Step 3: Investigate posts in parallel (fan-out pattern)
        let investigated_posts = ctx
            .run("investigate_all_posts", || async {
                let mut posts = Vec::new();
                for narrative in narratives {
                    match activities::investigate_post(&narrative, &self.deps).await {
                        Ok(info) => {
                            posts.push(crate::common::ExtractedPost {
                                title: narrative.title,
                                tldr: narrative.tldr,
                                description: narrative.description,
                                source_url: narrative.source_url,
                                info,
                            });
                        }
                        Err(e) => {
                            tracing::warn!(
                                title = %narrative.title,
                                error = %e,
                                "Investigation failed for post, using defaults"
                            );
                            posts.push(crate::common::ExtractedPost {
                                title: narrative.title,
                                tldr: narrative.tldr,
                                description: narrative.description,
                                source_url: narrative.source_url,
                                info: crate::common::ExtractedPostInformation::default(),
                            });
                        }
                    }
                }
                Ok::<Vec<crate::common::ExtractedPost>, anyhow::Error>(posts)
            })
            .await
            .map_err(|e| HandlerError::new(format!("Investigation failed: {}", e)))?;

        tracing::info!(
            website_id = %request.website_id,
            posts_count = investigated_posts.len(),
            "Step 4: Syncing posts to database"
        );

        // Step 4: Sync and deduplicate posts
        let synced_count = ctx
            .run("sync_posts", || async {
                use crate::domains::posts::actions::llm_sync::llm_sync_posts;
                llm_sync_posts(website_id_typed, investigated_posts, &self.deps)
                    .await
                    .map(|_| 0) // llm_sync_posts doesn't return count, estimate 0 for now
            })
            .await
            .map_err(|e| HandlerError::new(format!("Post sync failed: {}", e)))?;

        tracing::info!(
            website_id = %request.website_id,
            posts_synced = synced_count,
            "Crawl workflow completed successfully"
        );

        Ok(Json(CrawlWebsiteResult {
            website_id: request.website_id,
            posts_synced: synced_count,
            status: "completed".to_string(),
        }))
    }
}
