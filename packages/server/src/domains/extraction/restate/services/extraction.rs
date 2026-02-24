//! Extraction service (stateless)
//!
//! URL submission, extraction queries, site ingestion.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::common::auth::restate_auth::require_admin;
use crate::domains::extraction::activities as extraction_activities;
use crate::domains::extraction::data::ExtractionPageData;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitUrlRequest {
    pub url: String,
    pub query: Option<String>,
}

impl_restate_serde!(SubmitUrlRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerRequest {
    pub query: String,
    pub site: Option<String>,
}

impl_restate_serde!(TriggerRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRequest {
    pub site_url: String,
    pub max_pages: Option<i32>,
}

impl_restate_serde!(IngestRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPageRequest {
    pub url: String,
}

impl_restate_serde!(GetPageRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPagesRequest {
    pub domain: String,
    pub limit: Option<i32>,
}

impl_restate_serde!(ListPagesRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountRequest {
    pub domain: String,
}

impl_restate_serde!(CountRequest);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitUrlResult {
    pub extractions_count: i32,
}

impl_restate_serde!(SubmitUrlResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerResult {
    pub extractions_count: i32,
}

impl_restate_serde!(TriggerResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResult {
    pub pages_ingested: i32,
}

impl_restate_serde!(IngestResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountResult {
    pub count: i64,
}

impl_restate_serde!(CountResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageResult {
    pub url: String,
    pub content: Option<String>,
}

impl_restate_serde!(PageResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageListResult {
    pub pages: Vec<PageResult>,
}

impl_restate_serde!(PageListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionalPageResult {
    pub page: Option<PageResult>,
}

impl_restate_serde!(OptionalPageResult);

// =============================================================================
// Service definition
// =============================================================================

#[restate_sdk::service]
#[name = "Extraction"]
pub trait ExtractionService {
    async fn submit_url(req: SubmitUrlRequest) -> Result<SubmitUrlResult, HandlerError>;
    async fn trigger(req: TriggerRequest) -> Result<TriggerResult, HandlerError>;
    async fn ingest_site(req: IngestRequest) -> Result<IngestResult, HandlerError>;
    async fn get_page(req: GetPageRequest) -> Result<OptionalPageResult, HandlerError>;
    async fn list_pages(req: ListPagesRequest) -> Result<PageListResult, HandlerError>;
    async fn count_pages(req: CountRequest) -> Result<CountResult, HandlerError>;
}

pub struct ExtractionServiceImpl {
    deps: Arc<ServerDeps>,
}

impl ExtractionServiceImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl ExtractionService for ExtractionServiceImpl {
    async fn submit_url(
        &self,
        ctx: Context<'_>,
        req: SubmitUrlRequest,
    ) -> Result<SubmitUrlResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let results = extraction_activities::submit_url(&req.url, req.query.as_deref(), &self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(SubmitUrlResult {
            extractions_count: results.len() as i32,
        })
    }

    async fn trigger(
        &self,
        ctx: Context<'_>,
        req: TriggerRequest,
    ) -> Result<TriggerResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let results =
            extraction_activities::trigger_extraction(&req.query, req.site.as_deref(), &self.deps)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(TriggerResult {
            extractions_count: results.len() as i32,
        })
    }

    async fn ingest_site(
        &self,
        ctx: Context<'_>,
        req: IngestRequest,
    ) -> Result<IngestResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let result = ctx
            .run(|| async {
                extraction_activities::ingest_site(&req.site_url, req.max_pages, &self.deps)
                    .await
                    .map_err(Into::into)
            })
            .await?;

        Ok(IngestResult {
            pages_ingested: result.pages_crawled,
        })
    }

    async fn get_page(
        &self,
        ctx: Context<'_>,
        req: GetPageRequest,
    ) -> Result<OptionalPageResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let page = ExtractionPageData::find_by_url(&req.url, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(OptionalPageResult {
            page: page.map(|p| PageResult {
                url: p.url,
                content: Some(p.content.clone()),
            }),
        })
    }

    async fn list_pages(
        &self,
        ctx: Context<'_>,
        req: ListPagesRequest,
    ) -> Result<PageListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let limit = req.limit.unwrap_or(50);
        let pages = ExtractionPageData::find_by_domain(&req.domain, limit, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(PageListResult {
            pages: pages
                .into_iter()
                .map(|p| PageResult {
                    url: p.url,
                    content: Some(p.content.clone()),
                })
                .collect(),
        })
    }

    async fn count_pages(
        &self,
        ctx: Context<'_>,
        req: CountRequest,
    ) -> Result<CountResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let count = ExtractionPageData::count_by_domain(&req.domain, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(CountResult {
            count: count as i64,
        })
    }
}
