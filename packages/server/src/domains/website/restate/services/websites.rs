//! Websites service (stateless)
//!
//! Cross-website operations: list, search.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use crate::common::auth::restate_auth::require_admin;
use crate::common::{EmptyRequest, MemberId, PaginationArgs};
use crate::domains::crawling::activities::ingest_website;
use crate::domains::posts::models::post::Post;
use crate::domains::website::activities;
use crate::domains::website::models::website::CreateWebsite;
use crate::domains::website::models::Website;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

use crate::domains::website::restate::virtual_objects::website::WebsiteResult;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListWebsitesRequest {
    pub status: Option<String>,
    pub first: Option<i32>,
    pub after: Option<String>,
    pub last: Option<i32>,
    pub before: Option<String>,
}

impl_restate_serde!(ListWebsitesRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchWebsitesRequest {
    pub query: String,
    pub limit: Option<i32>,
    pub threshold: Option<f32>,
}

impl_restate_serde!(SearchWebsitesRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitWebsiteRequest {
    pub url: String,
}

impl_restate_serde!(SubmitWebsiteRequest);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteListResult {
    pub websites: Vec<WebsiteResult>,
    pub total_count: i32,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

impl_restate_serde!(WebsiteListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteSearchResult {
    pub id: Uuid,
    pub domain: String,
    pub status: String,
    pub similarity: f64,
}

impl_restate_serde!(WebsiteSearchResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingWebsitesResult {
    pub websites: Vec<WebsiteResult>,
}

impl_restate_serde!(PendingWebsitesResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteSearchResults {
    pub results: Vec<WebsiteSearchResult>,
}

impl_restate_serde!(WebsiteSearchResults);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledScrapeResult {
    pub websites_scraped: i32,
    pub status: String,
}

impl_restate_serde!(ScheduledScrapeResult);

// =============================================================================
// Service definition
// =============================================================================

#[restate_sdk::service]
#[name = "Websites"]
pub trait WebsitesService {
    async fn list(req: ListWebsitesRequest) -> Result<WebsiteListResult, HandlerError>;
    async fn list_pending(
        req: ListWebsitesRequest,
    ) -> Result<PendingWebsitesResult, HandlerError>;
    async fn search(
        req: SearchWebsitesRequest,
    ) -> Result<WebsiteSearchResults, HandlerError>;
    async fn run_scheduled_scrape(
        req: EmptyRequest,
    ) -> Result<ScheduledScrapeResult, HandlerError>;
    async fn submit(req: SubmitWebsiteRequest) -> Result<WebsiteResult, HandlerError>;
}

pub struct WebsitesServiceImpl {
    deps: Arc<ServerDeps>,
}

impl WebsitesServiceImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl WebsitesService for WebsitesServiceImpl {
    async fn list(
        &self,
        ctx: Context<'_>,
        req: ListWebsitesRequest,
    ) -> Result<WebsiteListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let pagination_args = PaginationArgs {
            first: req.first,
            after: req.after,
            last: req.last,
            before: req.before,
        };
        let validated = pagination_args
            .validate()
            .map_err(|e| TerminalError::new(e))?;

        let connection =
            activities::get_websites_paginated(req.status.as_deref(), &validated, &self.deps)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

        // Collect website IDs for batch post count query
        let website_ids: Vec<Uuid> = connection
            .edges
            .iter()
            .filter_map(|e| uuid::Uuid::parse_str(&e.node.id).ok())
            .collect();

        let post_counts = Post::count_by_website_ids(&website_ids, &self.deps.db_pool)
            .await
            .unwrap_or_default();

        Ok(WebsiteListResult {
            websites: connection
                .edges
                .into_iter()
                .filter_map(|e| {
                    uuid::Uuid::parse_str(&e.node.id).ok().map(|id| WebsiteResult {
                        id,
                        domain: e.node.domain,
                        status: e.node.status,
                        active: e.node.active,
                        created_at: Some(e.node.created_at),
                        last_crawled_at: e.node.last_scraped_at,
                        post_count: Some(*post_counts.get(&id).unwrap_or(&0)),
                        crawl_status: e.node.crawl_status,
                    })
                })
                .collect(),
            total_count: connection.total_count,
            has_next_page: connection.page_info.has_next_page,
            has_previous_page: connection.page_info.has_previous_page,
        })
    }

    async fn list_pending(
        &self,
        ctx: Context<'_>,
        _req: ListWebsitesRequest,
    ) -> Result<PendingWebsitesResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let websites = activities::get_pending_websites(&self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(PendingWebsitesResult {
            websites: websites
                .into_iter()
                .map(|w| WebsiteResult::from(w))
                .collect(),
        })
    }

    async fn search(
        &self,
        ctx: Context<'_>,
        req: SearchWebsitesRequest,
    ) -> Result<WebsiteSearchResults, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let threshold = req.threshold.unwrap_or(0.5);
        let limit = req.limit.unwrap_or(20);

        let results =
            activities::search_websites_semantic(&req.query, threshold, limit, &self.deps)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(WebsiteSearchResults {
            results: results
                .into_iter()
                .map(|r| WebsiteSearchResult {
                    id: r.website_id,
                    domain: r.website_domain,
                    status: r.recommendation,
                    similarity: r.similarity,
                })
                .collect(),
        })
    }

    async fn run_scheduled_scrape(
        &self,
        ctx: Context<'_>,
        _req: EmptyRequest,
    ) -> Result<ScheduledScrapeResult, HandlerError> {
        tracing::info!("Running scheduled website scrape");

        let pool = &self.deps.db_pool;

        let sources = Website::find_due_for_scraping(pool)
            .await
            .map_err(|e| HandlerError::from(e.to_string()))?;

        if sources.is_empty() {
            tracing::info!("No websites due for scraping");

            // Schedule next run
            ctx.service_client::<WebsitesServiceClient>()
                .run_scheduled_scrape(EmptyRequest {})
                .send_after(Duration::from_secs(3600));

            return Ok(ScheduledScrapeResult {
                websites_scraped: 0,
                status: "no_websites_due".to_string(),
            });
        }

        tracing::info!("Found {} websites due for scraping", sources.len());
        let mut scraped_count = 0i32;

        for website in &sources {
            let website_id = website.id.into_uuid();
            let domain = website.domain.clone();

            let result = ctx
                .run(|| {
                    let deps = &self.deps;
                    async move {
                        ingest_website(
                            website_id,
                            MemberId::nil().into_uuid(),
                            true,
                            true,
                            deps,
                        )
                        .await
                        .map_err(Into::into)
                    }
                })
                .await;

            match result {
                Ok(ingested) => {
                    tracing::info!(
                        website_id = %website_id,
                        domain = %domain,
                        job_id = %ingested.job_id,
                        pages_crawled = ingested.pages_crawled,
                        pages_summarized = ingested.pages_summarized,
                        "Ingested website"
                    );
                    scraped_count += 1;
                }
                Err(e) => {
                    tracing::error!(
                        website_id = %website_id,
                        domain = %domain,
                        error = %e,
                        "Failed to ingest website"
                    );
                }
            }
        }

        // Schedule next run
        ctx.service_client::<WebsitesServiceClient>()
            .run_scheduled_scrape(EmptyRequest {})
            .send_after(Duration::from_secs(3600));

        Ok(ScheduledScrapeResult {
            websites_scraped: scraped_count,
            status: "completed".to_string(),
        })
    }

    async fn submit(
        &self,
        ctx: Context<'_>,
        req: SubmitWebsiteRequest,
    ) -> Result<WebsiteResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let input = CreateWebsite::builder()
            .url_or_domain(req.url)
            .submitted_by(Some(user.member_id))
            .submitter_type("admin".to_string())
            .build();

        let website = Website::create(input, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(WebsiteResult::from(website))
    }
}
