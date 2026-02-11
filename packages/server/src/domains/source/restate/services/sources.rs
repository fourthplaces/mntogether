use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use crate::common::auth::restate_auth::require_admin;
use crate::common::{EmptyRequest, MemberId, OrganizationId, PaginationArgs, SourceId};
use crate::domains::crawling::activities::ingest_website;
use crate::domains::crawling::models::ExtractionPage;
use crate::domains::posts::models::PostSource;
use crate::domains::source::models::{
    find_or_create_social_source, find_or_create_website_source, Source,
    WebsiteSource,
};
use crate::domains::website::activities;
use crate::domains::website::models::SearchQuery;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSourcesRequest {
    pub status: Option<String>,
    pub source_type: Option<String>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub organization_id: Option<Uuid>,
    pub first: Option<i32>,
    pub after: Option<String>,
    pub last: Option<i32>,
    pub before: Option<String>,
}

impl_restate_serde!(ListSourcesRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitWebsiteRequest {
    pub url: String,
}

impl_restate_serde!(SubmitWebsiteRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSocialSourceRequest {
    pub platform: String,
    pub handle: String,
    pub url: Option<String>,
    pub organization_id: Option<Uuid>,
}

impl_restate_serde!(CreateSocialSourceRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteSourceRequest {
    pub id: Uuid,
}

impl_restate_serde!(DeleteSourceRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListByOrganizationRequest {
    pub organization_id: Uuid,
}

impl_restate_serde!(ListByOrganizationRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSourcesRequest {
    pub query: String,
    pub limit: Option<i32>,
    pub threshold: Option<f32>,
}

impl_restate_serde!(SearchSourcesRequest);

// Search query CRUD request types (unchanged from Websites service)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSearchQueryRequest {
    pub query_text: String,
    pub sort_order: Option<i32>,
}
impl_restate_serde!(CreateSearchQueryRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSearchQueryRequest {
    pub id: Uuid,
    pub query_text: String,
    pub sort_order: Option<i32>,
}
impl_restate_serde!(UpdateSearchQueryRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteSearchQueryRequest {
    pub id: Uuid,
}
impl_restate_serde!(DeleteSearchQueryRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToggleSearchQueryRequest {
    pub id: Uuid,
}
impl_restate_serde!(ToggleSearchQueryRequest);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceResult {
    pub id: String,
    pub source_type: String,
    pub identifier: String, // domain or handle
    pub url: Option<String>,
    pub status: String,
    pub active: bool,
    pub organization_id: Option<String>,
    pub organization_name: Option<String>,
    pub scrape_frequency_hours: i32,
    pub last_scraped_at: Option<String>,
    pub post_count: Option<i64>,
    pub snapshot_count: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

impl_restate_serde!(SourceResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceListResult {
    pub sources: Vec<SourceResult>,
    pub total_count: i32,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

impl_restate_serde!(SourceListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledScrapeResult {
    pub sources_scraped: i32,
    pub posts_created: i32,
    pub status: String,
}

impl_restate_serde!(ScheduledScrapeResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSearchResult {
    pub id: Uuid,
    pub domain: String,
    pub status: String,
    pub similarity: f64,
}
impl_restate_serde!(SourceSearchResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSearchResults {
    pub results: Vec<SourceSearchResult>,
}
impl_restate_serde!(SourceSearchResults);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledDiscoveryResult {
    pub queries_executed: i32,
    pub total_results: i32,
    pub websites_created: i32,
    pub websites_skipped: i32,
    pub status: String,
}
impl_restate_serde!(ScheduledDiscoveryResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQueryResult {
    pub id: Uuid,
    pub query_text: String,
    pub is_active: bool,
    pub sort_order: i32,
}
impl_restate_serde!(SearchQueryResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQueryListResult {
    pub queries: Vec<SearchQueryResult>,
}
impl_restate_serde!(SearchQueryListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmptyResult {}
impl_restate_serde!(EmptyResult);

// =============================================================================
// Service definition
// =============================================================================

#[restate_sdk::service]
#[name = "Sources"]
pub trait SourcesService {
    async fn list(req: ListSourcesRequest) -> Result<SourceListResult, HandlerError>;
    async fn list_by_organization(
        req: ListByOrganizationRequest,
    ) -> Result<SourceListResult, HandlerError>;
    async fn submit_website(req: SubmitWebsiteRequest) -> Result<SourceResult, HandlerError>;
    async fn create_social(
        req: CreateSocialSourceRequest,
    ) -> Result<SourceResult, HandlerError>;
    async fn delete(req: DeleteSourceRequest) -> Result<EmptyRequest, HandlerError>;
    async fn search(req: SearchSourcesRequest) -> Result<SourceSearchResults, HandlerError>;
    async fn run_scheduled_scrape(
        req: EmptyRequest,
    ) -> Result<ScheduledScrapeResult, HandlerError>;

    // Search query CRUD
    async fn list_search_queries(
        req: EmptyRequest,
    ) -> Result<SearchQueryListResult, HandlerError>;
    async fn create_search_query(
        req: CreateSearchQueryRequest,
    ) -> Result<SearchQueryResult, HandlerError>;
    async fn update_search_query(
        req: UpdateSearchQueryRequest,
    ) -> Result<SearchQueryResult, HandlerError>;
    async fn delete_search_query(
        req: DeleteSearchQueryRequest,
    ) -> Result<EmptyResult, HandlerError>;
    async fn toggle_search_query(
        req: ToggleSearchQueryRequest,
    ) -> Result<SearchQueryResult, HandlerError>;

    // Discovery
    async fn run_scheduled_discovery(
        req: EmptyRequest,
    ) -> Result<ScheduledDiscoveryResult, HandlerError>;
}

pub struct SourcesServiceImpl {
    deps: Arc<ServerDeps>,
}

impl SourcesServiceImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

fn search_query_result(q: SearchQuery) -> SearchQueryResult {
    SearchQueryResult {
        id: q.id,
        query_text: q.query_text,
        is_active: q.is_active,
        sort_order: q.sort_order,
    }
}

/// Build a SourceResult from a Source, looking up the identifier from extension tables
async fn source_to_result(source: Source, pool: &sqlx::PgPool) -> Result<SourceResult, HandlerError> {
    let identifier = crate::domains::source::models::get_source_identifier(source.id, pool)
        .await
        .unwrap_or_else(|_| "unknown".to_string());

    // Look up organization name if linked
    let organization_name = if let Some(org_id) = source.organization_id {
        crate::domains::organization::models::Organization::find_by_id(org_id, pool)
            .await
            .ok()
            .map(|o| o.name)
    } else {
        None
    };

    Ok(SourceResult {
        id: source.id.to_string(),
        source_type: source.source_type,
        identifier,
        url: source.url,
        status: source.status,
        active: source.active,
        organization_id: source.organization_id.map(|id| id.to_string()),
        organization_name,
        scrape_frequency_hours: source.scrape_frequency_hours,
        last_scraped_at: source.last_scraped_at.map(|dt| dt.to_rfc3339()),
        post_count: None,
        snapshot_count: None,
        created_at: source.created_at.to_rfc3339(),
        updated_at: source.updated_at.to_rfc3339(),
    })
}

impl SourcesService for SourcesServiceImpl {
    async fn list(
        &self,
        ctx: Context<'_>,
        req: ListSourcesRequest,
    ) -> Result<SourceListResult, HandlerError> {
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

        let (sources, has_more) = Source::find_paginated(
            req.status.as_deref(),
            req.source_type.as_deref(),
            req.search.as_deref(),
            req.organization_id,
            &validated,
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        let total_count = Source::count_with_filters(
            req.status.as_deref(),
            req.source_type.as_deref(),
            req.search.as_deref(),
            req.organization_id,
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        // Batch lookup identifiers
        let source_ids: Vec<Uuid> = sources.iter().map(|s| s.id.into_uuid()).collect();
        let website_domains = WebsiteSource::find_domains_by_source_ids(&source_ids, &self.deps.db_pool)
            .await
            .unwrap_or_default();
        let domain_map: std::collections::HashMap<Uuid, String> =
            website_domains.into_iter().collect();

        // Batch lookup social handles
        let social_handles: Vec<(Uuid, String)> = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT source_id, handle FROM social_sources WHERE source_id = ANY($1)",
        )
        .bind(&source_ids)
        .fetch_all(&self.deps.db_pool)
        .await
        .unwrap_or_default();
        let handle_map: std::collections::HashMap<Uuid, String> =
            social_handles.into_iter().collect();

        // Batch lookup post counts
        let post_counts = PostSource::count_by_sources_any_type(&source_ids, &self.deps.db_pool)
            .await
            .unwrap_or_default();

        let results: Vec<SourceResult> = sources
            .into_iter()
            .map(|s| {
                let sid = s.id.into_uuid();
                let identifier = domain_map
                    .get(&sid)
                    .or_else(|| handle_map.get(&sid))
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());

                SourceResult {
                    id: s.id.to_string(),
                    source_type: s.source_type,
                    identifier,
                    url: s.url,
                    status: s.status,
                    active: s.active,
                    organization_id: s.organization_id.map(|id| id.to_string()),
                    organization_name: None, // not needed for list view
                    scrape_frequency_hours: s.scrape_frequency_hours,
                    last_scraped_at: s.last_scraped_at.map(|dt| dt.to_rfc3339()),
                    post_count: Some(*post_counts.get(&sid).unwrap_or(&0)),
                    snapshot_count: None,
                    created_at: s.created_at.to_rfc3339(),
                    updated_at: s.updated_at.to_rfc3339(),
                }
            })
            .collect();

        Ok(SourceListResult {
            sources: results,
            total_count: total_count as i32,
            has_next_page: has_more,
            has_previous_page: false, // simplified
        })
    }

    async fn list_by_organization(
        &self,
        ctx: Context<'_>,
        req: ListByOrganizationRequest,
    ) -> Result<SourceListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let sources = Source::find_by_organization(
            OrganizationId::from(req.organization_id),
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        let mut results = Vec::new();
        for s in sources {
            // Compute snapshot (extraction page) count per source
            let snapshot_count = match s.site_url(&self.deps.db_pool).await {
                Ok(site_url) => ExtractionPage::count_by_domain(&site_url, &self.deps.db_pool)
                    .await
                    .unwrap_or(0) as i64,
                Err(_) => 0,
            };

            let mut result = source_to_result(s, &self.deps.db_pool).await?;
            result.snapshot_count = Some(snapshot_count);
            results.push(result);
        }

        let count = results.len() as i32;
        Ok(SourceListResult {
            sources: results,
            total_count: count,
            has_next_page: false,
            has_previous_page: false,
        })
    }

    async fn submit_website(
        &self,
        ctx: Context<'_>,
        req: SubmitWebsiteRequest,
    ) -> Result<SourceResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let (source, _ws) = find_or_create_website_source(
            &req.url,
            Some(user.member_id),
            "admin",
            None,
            2,
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        source_to_result(source, &self.deps.db_pool).await
    }

    async fn create_social(
        &self,
        ctx: Context<'_>,
        req: CreateSocialSourceRequest,
    ) -> Result<SourceResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let org_id = req.organization_id.map(OrganizationId::from);

        let (source, _ss) = find_or_create_social_source(
            &req.platform,
            &req.handle,
            req.url.as_deref(),
            org_id,
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        // Link to organization if provided and not already linked
        let source = if let Some(org_id) = org_id {
            if source.organization_id != Some(org_id) {
                Source::set_organization_id(source.id, org_id, &self.deps.db_pool)
                    .await
                    .map_err(|e| TerminalError::new(e.to_string()))?
            } else {
                source
            }
        } else {
            source
        };

        source_to_result(source, &self.deps.db_pool).await
    }

    async fn delete(
        &self,
        ctx: Context<'_>,
        req: DeleteSourceRequest,
    ) -> Result<EmptyRequest, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        Source::delete(SourceId::from_uuid(req.id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(EmptyRequest {})
    }

    async fn search(
        &self,
        ctx: Context<'_>,
        req: SearchSourcesRequest,
    ) -> Result<SourceSearchResults, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let threshold = req.threshold.unwrap_or(0.5);
        let limit = req.limit.unwrap_or(20);

        let results =
            activities::search_websites_semantic(&req.query, threshold, limit, &self.deps)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(SourceSearchResults {
            results: results
                .into_iter()
                .map(|r| SourceSearchResult {
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
        tracing::info!("Running scheduled source scrape");

        let pool = &self.deps.db_pool;
        let sources = Source::find_due_for_scraping(pool)
            .await
            .map_err(|e| HandlerError::from(e.to_string()))?;

        if sources.is_empty() {
            tracing::info!("No sources due for scraping");
            ctx.service_client::<SourcesServiceClient>()
                .run_scheduled_scrape(EmptyRequest {})
                .send_after(Duration::from_secs(3600));

            return Ok(ScheduledScrapeResult {
                sources_scraped: 0,
                posts_created: 0,
                status: "no_sources_due".to_string(),
            });
        }

        tracing::info!("Found {} sources due for scraping", sources.len());
        let mut scraped_count = 0i32;
        let posts_created = 0i32;

        for source in &sources {
            let source_id = source.id.into_uuid();

            match source.source_type.as_str() {
                "website" => {
                    let result = ctx
                        .run(|| {
                            let deps = &self.deps;
                            async move {
                                ingest_website(
                                    source_id,
                                    MemberId::nil().into_uuid(),
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
                                source_id = %source_id,
                                job_id = %ingested.job_id,
                                "Ingested website source"
                            );
                            scraped_count += 1;
                        }
                        Err(e) => {
                            tracing::error!(
                                source_id = %source_id,
                                error = %e,
                                "Failed to ingest website source"
                            );
                        }
                    }
                }
                "instagram" => {
                    // TODO: Update to use new Source + SocialSource models
                    // once ingest_instagram is migrated
                    tracing::warn!(
                        source_id = %source_id,
                        "Instagram scraping not yet migrated to source model"
                    );
                }
                platform => {
                    tracing::warn!(
                        platform,
                        source_id = %source_id,
                        "Unsupported source type, skipping"
                    );
                }
            }
        }

        // Schedule next run
        ctx.service_client::<SourcesServiceClient>()
            .run_scheduled_scrape(EmptyRequest {})
            .send_after(Duration::from_secs(3600));

        Ok(ScheduledScrapeResult {
            sources_scraped: scraped_count,
            posts_created,
            status: "completed".to_string(),
        })
    }

    // =========================================================================
    // Search query CRUD (delegated to website activities, unchanged)
    // =========================================================================

    async fn list_search_queries(
        &self,
        ctx: Context<'_>,
        _req: EmptyRequest,
    ) -> Result<SearchQueryListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let queries = SearchQuery::find_all(&self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;
        Ok(SearchQueryListResult {
            queries: queries.into_iter().map(search_query_result).collect(),
        })
    }

    async fn create_search_query(
        &self,
        ctx: Context<'_>,
        req: CreateSearchQueryRequest,
    ) -> Result<SearchQueryResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let result = ctx
            .run(|| async {
                let q = SearchQuery::create(&req.query_text, req.sort_order.unwrap_or(0), &self.deps.db_pool)
                    .await
                    .map_err(|e| TerminalError::new(e.to_string()))?;
                Ok(search_query_result(q))
            })
            .await?;
        Ok(result)
    }

    async fn update_search_query(
        &self,
        ctx: Context<'_>,
        req: UpdateSearchQueryRequest,
    ) -> Result<SearchQueryResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let result = ctx
            .run(|| async {
                let q = SearchQuery::update(req.id, &req.query_text, req.sort_order.unwrap_or(0), &self.deps.db_pool)
                    .await
                    .map_err(|e| TerminalError::new(e.to_string()))?;
                Ok(search_query_result(q))
            })
            .await?;
        Ok(result)
    }

    async fn delete_search_query(
        &self,
        ctx: Context<'_>,
        req: DeleteSearchQueryRequest,
    ) -> Result<EmptyResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        ctx.run(|| async {
            SearchQuery::delete(req.id, &self.deps.db_pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;
            Ok(())
        })
        .await?;
        Ok(EmptyResult {})
    }

    async fn toggle_search_query(
        &self,
        ctx: Context<'_>,
        req: ToggleSearchQueryRequest,
    ) -> Result<SearchQueryResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let result = ctx
            .run(|| async {
                let q = SearchQuery::toggle_active(req.id, &self.deps.db_pool)
                    .await
                    .map_err(|e| TerminalError::new(e.to_string()))?;
                Ok(search_query_result(q))
            })
            .await?;
        Ok(result)
    }

    // =========================================================================
    // Discovery
    // =========================================================================

    async fn run_scheduled_discovery(
        &self,
        ctx: Context<'_>,
        _req: EmptyRequest,
    ) -> Result<ScheduledDiscoveryResult, HandlerError> {
        tracing::info!("Running scheduled source discovery");

        let result = ctx
            .run(|| async {
                activities::discover::run_discovery(&self.deps)
                    .await
                    .map_err(|e| TerminalError::new(e.to_string()).into())
            })
            .await?;

        ctx.service_client::<SourcesServiceClient>()
            .run_scheduled_discovery(EmptyRequest {})
            .send_after(Duration::from_secs(86400));

        Ok(ScheduledDiscoveryResult {
            queries_executed: result.queries_executed,
            total_results: result.total_results,
            websites_created: result.websites_created,
            websites_skipped: result.websites_skipped,
            status: "completed".to_string(),
        })
    }
}
