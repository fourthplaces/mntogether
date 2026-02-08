//! Discovery service (stateless)
//!
//! Admin-managed search queries, filter rules, and discovery runs.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::common::auth::restate_auth::require_admin;
use crate::common::EmptyRequest;
use crate::domains::discovery::activities as discovery_activities;
use crate::domains::discovery::models::{
    DiscoveryFilterRule, DiscoveryQuery, DiscoveryRun, DiscoveryRunResult,
};
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListQueriesRequest {
    pub include_inactive: Option<bool>,
}

impl_restate_serde!(ListQueriesRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateQueryRequest {
    pub query_text: String,
    pub category: Option<String>,
}

impl_restate_serde!(CreateQueryRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateQueryRequest {
    pub id: Uuid,
    pub query_text: Option<String>,
    pub category: Option<String>,
}

impl_restate_serde!(UpdateQueryRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToggleQueryRequest {
    pub id: Uuid,
    pub is_active: bool,
}

impl_restate_serde!(ToggleQueryRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRequest {
    pub id: Uuid,
}

impl_restate_serde!(DeleteRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListFilterRulesRequest {
    pub query_id: Option<Uuid>,
}

impl_restate_serde!(ListFilterRulesRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFilterRuleRequest {
    pub query_id: Option<Uuid>,
    pub rule_text: String,
}

impl_restate_serde!(CreateFilterRuleRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFilterRuleRequest {
    pub id: Uuid,
    pub rule_text: String,
}

impl_restate_serde!(UpdateFilterRuleRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRunsRequest {
    pub limit: Option<i32>,
}

impl_restate_serde!(ListRunsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResultsRequest {
    pub run_id: Uuid,
}

impl_restate_serde!(RunResultsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteSourcesRequest {
    pub website_id: Uuid,
}

impl_restate_serde!(WebsiteSourcesRequest);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub id: Uuid,
    pub query_text: String,
    pub category: Option<String>,
    pub is_active: bool,
}

impl_restate_serde!(QueryResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryListResult {
    pub queries: Vec<QueryResult>,
}

impl_restate_serde!(QueryListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRuleResult {
    pub id: Uuid,
    pub query_id: Option<Uuid>,
    pub rule_text: String,
    pub sort_order: i32,
    pub is_active: bool,
}

impl_restate_serde!(FilterRuleResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRuleListResult {
    pub rules: Vec<FilterRuleResult>,
}

impl_restate_serde!(FilterRuleListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    pub id: Uuid,
    pub queries_executed: i32,
    pub total_results: i32,
    pub websites_created: i32,
    pub websites_filtered: i32,
}

impl_restate_serde!(RunResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunListResult {
    pub runs: Vec<RunResult>,
}

impl_restate_serde!(RunListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResultDetail {
    pub id: Uuid,
    pub run_id: Uuid,
    pub query_id: Uuid,
    pub domain: String,
    pub url: String,
    pub title: Option<String>,
    pub snippet: Option<String>,
    pub relevance_score: Option<f64>,
    pub filter_result: String,
    pub filter_reason: Option<String>,
    pub website_id: Option<Uuid>,
    pub discovered_at: String,
}

impl_restate_serde!(RunResultDetail);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResultDetailList {
    pub results: Vec<RunResultDetail>,
}

impl_restate_serde!(RunResultDetailList);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverySearchResult {
    pub queries_run: i32,
    pub total_results: i32,
    pub websites_created: i32,
    pub websites_filtered: i32,
    pub run_id: Uuid,
}

impl_restate_serde!(DiscoverySearchResult);

// =============================================================================
// Service definition
// =============================================================================

#[restate_sdk::service]
#[name = "Discovery"]
pub trait DiscoveryService {
    async fn list_queries(req: ListQueriesRequest) -> Result<QueryListResult, HandlerError>;
    async fn create_query(req: CreateQueryRequest) -> Result<QueryResult, HandlerError>;
    async fn update_query(req: UpdateQueryRequest) -> Result<QueryResult, HandlerError>;
    async fn toggle_query(req: ToggleQueryRequest) -> Result<QueryResult, HandlerError>;
    async fn delete_query(req: DeleteRequest) -> Result<(), HandlerError>;
    async fn list_filter_rules(
        req: ListFilterRulesRequest,
    ) -> Result<FilterRuleListResult, HandlerError>;
    async fn create_filter_rule(
        req: CreateFilterRuleRequest,
    ) -> Result<FilterRuleResult, HandlerError>;
    async fn update_filter_rule(
        req: UpdateFilterRuleRequest,
    ) -> Result<FilterRuleResult, HandlerError>;
    async fn delete_filter_rule(req: DeleteRequest) -> Result<(), HandlerError>;
    async fn list_runs(req: ListRunsRequest) -> Result<RunListResult, HandlerError>;
    async fn get_run_results(req: RunResultsRequest) -> Result<RunResultDetailList, HandlerError>;
    async fn get_website_sources(
        req: WebsiteSourcesRequest,
    ) -> Result<RunResultDetailList, HandlerError>;
    async fn run_discovery(req: EmptyRequest) -> Result<DiscoverySearchResult, HandlerError>;
}

pub struct DiscoveryServiceImpl {
    deps: Arc<ServerDeps>,
}

impl DiscoveryServiceImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl DiscoveryService for DiscoveryServiceImpl {
    async fn list_queries(
        &self,
        ctx: Context<'_>,
        req: ListQueriesRequest,
    ) -> Result<QueryListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let queries = if req.include_inactive.unwrap_or(false) {
            DiscoveryQuery::find_all(&self.deps.db_pool).await
        } else {
            DiscoveryQuery::find_active(&self.deps.db_pool).await
        }
        .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(QueryListResult {
            queries: queries
                .into_iter()
                .map(|q| QueryResult {
                    id: q.id,
                    query_text: q.query_text,
                    category: q.category,
                    is_active: q.is_active,
                })
                .collect(),
        })
    }

    async fn create_query(
        &self,
        ctx: Context<'_>,
        req: CreateQueryRequest,
    ) -> Result<QueryResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let query = ctx
            .run(|| async {
                DiscoveryQuery::create(
                    req.query_text.clone(),
                    req.category.clone(),
                    Some(user.member_id.into_uuid()),
                    &self.deps.db_pool,
                )
                .await
                .map_err(Into::into)
            })
            .await?;

        Ok(QueryResult {
            id: query.id,
            query_text: query.query_text,
            category: query.category,
            is_active: query.is_active,
        })
    }

    async fn update_query(
        &self,
        ctx: Context<'_>,
        req: UpdateQueryRequest,
    ) -> Result<QueryResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let query_id = req.id;
        let query_text = req.query_text.unwrap_or_default();
        let query = ctx
            .run(|| async {
                DiscoveryQuery::update(
                    query_id,
                    query_text.clone(),
                    req.category.clone(),
                    &self.deps.db_pool,
                )
                .await
                .map_err(Into::into)
            })
            .await?;

        Ok(QueryResult {
            id: query.id,
            query_text: query.query_text,
            category: query.category,
            is_active: query.is_active,
        })
    }

    async fn toggle_query(
        &self,
        ctx: Context<'_>,
        req: ToggleQueryRequest,
    ) -> Result<QueryResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let query = ctx
            .run(|| async {
                DiscoveryQuery::toggle_active(req.id, req.is_active, &self.deps.db_pool)
                    .await
                    .map_err(Into::into)
            })
            .await?;

        Ok(QueryResult {
            id: query.id,
            query_text: query.query_text,
            category: query.category,
            is_active: query.is_active,
        })
    }

    async fn delete_query(
        &self,
        ctx: Context<'_>,
        req: DeleteRequest,
    ) -> Result<(), HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        ctx.run(|| async {
            DiscoveryQuery::delete(req.id, &self.deps.db_pool)
                .await
                .map_err(Into::into)
        })
        .await?;

        Ok(())
    }

    async fn list_filter_rules(
        &self,
        ctx: Context<'_>,
        req: ListFilterRulesRequest,
    ) -> Result<FilterRuleListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let rules =
            DiscoveryFilterRule::find_all_for_query(req.query_id, &self.deps.db_pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(FilterRuleListResult {
            rules: rules
                .into_iter()
                .map(|r| FilterRuleResult {
                    id: r.id,
                    query_id: r.query_id,
                    rule_text: r.rule_text,
                    sort_order: r.sort_order,
                    is_active: r.is_active,
                })
                .collect(),
        })
    }

    async fn create_filter_rule(
        &self,
        ctx: Context<'_>,
        req: CreateFilterRuleRequest,
    ) -> Result<FilterRuleResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let rule = ctx
            .run(|| async {
                DiscoveryFilterRule::create(
                    req.query_id,
                    req.rule_text.clone(),
                    Some(user.member_id.into_uuid()),
                    &self.deps.db_pool,
                )
                .await
                .map_err(Into::into)
            })
            .await?;

        Ok(FilterRuleResult {
            id: rule.id,
            query_id: rule.query_id,
            rule_text: rule.rule_text,
            sort_order: rule.sort_order,
            is_active: rule.is_active,
        })
    }

    async fn update_filter_rule(
        &self,
        ctx: Context<'_>,
        req: UpdateFilterRuleRequest,
    ) -> Result<FilterRuleResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let rule = ctx
            .run(|| async {
                DiscoveryFilterRule::update(req.id, req.rule_text.clone(), &self.deps.db_pool)
                    .await
                    .map_err(Into::into)
            })
            .await?;

        Ok(FilterRuleResult {
            id: rule.id,
            query_id: rule.query_id,
            rule_text: rule.rule_text,
            sort_order: rule.sort_order,
            is_active: rule.is_active,
        })
    }

    async fn delete_filter_rule(
        &self,
        ctx: Context<'_>,
        req: DeleteRequest,
    ) -> Result<(), HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        ctx.run(|| async {
            DiscoveryFilterRule::delete(req.id, &self.deps.db_pool)
                .await
                .map_err(Into::into)
        })
        .await?;

        Ok(())
    }

    async fn list_runs(
        &self,
        ctx: Context<'_>,
        req: ListRunsRequest,
    ) -> Result<RunListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let limit = req.limit.unwrap_or(20);
        let runs = DiscoveryRun::find_recent(limit, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(RunListResult {
            runs: runs
                .into_iter()
                .map(|r| RunResult {
                    id: r.id,
                    queries_executed: r.queries_executed,
                    total_results: r.total_results,
                    websites_created: r.websites_created,
                    websites_filtered: r.websites_filtered,
                })
                .collect(),
        })
    }

    async fn get_run_results(
        &self,
        ctx: Context<'_>,
        req: RunResultsRequest,
    ) -> Result<RunResultDetailList, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let results = DiscoveryRunResult::find_by_run(req.run_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(RunResultDetailList {
            results: results.into_iter().map(run_result_to_detail).collect(),
        })
    }

    async fn get_website_sources(
        &self,
        ctx: Context<'_>,
        req: WebsiteSourcesRequest,
    ) -> Result<RunResultDetailList, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let results = DiscoveryRunResult::find_by_website(req.website_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(RunResultDetailList {
            results: results.into_iter().map(run_result_to_detail).collect(),
        })
    }

    async fn run_discovery(
        &self,
        ctx: Context<'_>,
        _req: EmptyRequest,
    ) -> Result<DiscoverySearchResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let stats = ctx
            .run(|| async {
                discovery_activities::run_discovery("manual", &self.deps)
                    .await
                    .map_err(Into::into)
            })
            .await?;

        Ok(DiscoverySearchResult {
            queries_run: stats.queries_executed as i32,
            total_results: stats.total_results as i32,
            websites_created: stats.websites_created as i32,
            websites_filtered: stats.websites_filtered as i32,
            run_id: stats.run_id,
        })
    }
}

fn run_result_to_detail(r: DiscoveryRunResult) -> RunResultDetail {
    RunResultDetail {
        id: r.id,
        run_id: r.run_id,
        query_id: r.query_id,
        domain: r.domain,
        url: r.url,
        title: r.title,
        snippet: r.snippet,
        relevance_score: r.relevance_score,
        filter_result: r.filter_result,
        filter_reason: r.filter_reason,
        website_id: r.website_id,
        discovered_at: r.discovered_at.to_rfc3339(),
    }
}
