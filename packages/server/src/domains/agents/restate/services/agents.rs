//! Agents service (stateless)
//!
//! CRUD for agents and their configs, plus pipeline triggers for curators.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::common::auth::restate_auth::require_admin;
use crate::domains::agents::activities::{discover, enrich, extract, monitor};
use crate::domains::agents::models::{
    Agent, AgentAssistantConfig, AgentCuratorConfig, AgentFilterRule, AgentRequiredTagKind,
    AgentRun, AgentRunStat, AgentSearchQuery, AgentWebsite,
};
use crate::domains::tag::models::tag_kind_config::TagKindConfig;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListAgentsRequest {
    pub role: Option<String>,
}
impl_restate_serde!(ListAgentsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdRequest {
    pub agent_id: Uuid,
}
impl_restate_serde!(AgentIdRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentRequest {
    pub display_name: String,
    pub role: String,
    /// For assistant: initial preamble
    pub preamble: Option<String>,
    /// For assistant: config_name (e.g., "admin", "public")
    pub config_name: Option<String>,
    /// For curator: purpose text
    pub purpose: Option<String>,
}
impl_restate_serde!(CreateAgentRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAgentRequest {
    pub agent_id: Uuid,
    pub display_name: Option<String>,
}
impl_restate_serde!(UpdateAgentRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetAgentStatusRequest {
    pub agent_id: Uuid,
    pub status: String,
}
impl_restate_serde!(SetAgentStatusRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCuratorConfigRequest {
    pub agent_id: Uuid,
    pub purpose: String,
    pub audience_roles: Vec<String>,
    pub schedule_discover: Option<String>,
    pub schedule_monitor: Option<String>,
}
impl_restate_serde!(UpdateCuratorConfigRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSearchQueryRequest {
    pub agent_id: Uuid,
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
pub struct DeleteRequest {
    pub id: Uuid,
}
impl_restate_serde!(DeleteRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFilterRuleRequest {
    pub agent_id: Uuid,
    pub rule_text: String,
    pub sort_order: Option<i32>,
}
impl_restate_serde!(CreateFilterRuleRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFilterRuleRequest {
    pub id: Uuid,
    pub rule_text: String,
    pub sort_order: Option<i32>,
}
impl_restate_serde!(UpdateFilterRuleRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetRequiredTagKindsRequest {
    pub agent_id: Uuid,
    pub tag_kind_ids: Vec<Uuid>,
}
impl_restate_serde!(SetRequiredTagKindsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunAgentStepRequest {
    pub agent_id: Uuid,
    pub step: String, // "discover", "extract", "enrich", "monitor"
}
impl_restate_serde!(RunAgentStepRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRunsRequest {
    pub agent_id: Uuid,
    pub limit: Option<i64>,
}
impl_restate_serde!(ListRunsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetRunRequest {
    pub run_id: Uuid,
}
impl_restate_serde!(GetRunRequest);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub id: Uuid,
    pub display_name: String,
    pub role: String,
    pub status: String,
    pub created_at: String,
}
impl_restate_serde!(AgentResponse);

impl From<Agent> for AgentResponse {
    fn from(a: Agent) -> Self {
        Self {
            id: a.id,
            display_name: a.display_name,
            role: a.role,
            status: a.status,
            created_at: a.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentListResponse {
    pub agents: Vec<AgentResponse>,
}
impl_restate_serde!(AgentListResponse);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDetailResponse {
    pub agent: AgentResponse,
    pub assistant_config: Option<AssistantConfigResponse>,
    pub curator_config: Option<CuratorConfigResponse>,
    pub search_queries: Vec<SearchQueryResponse>,
    pub filter_rules: Vec<FilterRuleResponse>,
    pub required_tag_kinds: Vec<TagKindResponse>,
    pub websites: Vec<AgentWebsiteResponse>,
}
impl_restate_serde!(AgentDetailResponse);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantConfigResponse {
    pub preamble: String,
    pub config_name: String,
}
impl_restate_serde!(AssistantConfigResponse);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuratorConfigResponse {
    pub purpose: String,
    pub audience_roles: Vec<String>,
    pub schedule_discover: Option<String>,
    pub schedule_monitor: Option<String>,
}
impl_restate_serde!(CuratorConfigResponse);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQueryResponse {
    pub id: Uuid,
    pub query_text: String,
    pub is_active: bool,
    pub sort_order: i32,
}
impl_restate_serde!(SearchQueryResponse);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRuleResponse {
    pub id: Uuid,
    pub rule_text: String,
    pub is_active: bool,
    pub sort_order: i32,
}
impl_restate_serde!(FilterRuleResponse);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagKindResponse {
    pub id: Uuid,
    pub slug: String,
    pub display_name: String,
}
impl_restate_serde!(TagKindResponse);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentWebsiteResponse {
    pub website_id: Uuid,
    pub domain: Option<String>,
    pub discovered_at: String,
}
impl_restate_serde!(AgentWebsiteResponse);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunResponse {
    pub id: Uuid,
    pub step: String,
    pub trigger_type: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub stats: Vec<RunStatResponse>,
}
impl_restate_serde!(AgentRunResponse);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunStatResponse {
    pub stat_key: String,
    pub stat_value: i32,
}
impl_restate_serde!(RunStatResponse);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmptyResponse {}
impl_restate_serde!(EmptyResponse);

// Wrapper types for Vec returns (Restate serde doesn't impl for Vec<T> directly)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQueryListResponse {
    pub queries: Vec<SearchQueryResponse>,
}
impl_restate_serde!(SearchQueryListResponse);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRuleListResponse {
    pub rules: Vec<FilterRuleResponse>,
}
impl_restate_serde!(FilterRuleListResponse);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagKindListResponse {
    pub tag_kinds: Vec<TagKindResponse>,
}
impl_restate_serde!(TagKindListResponse);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunListResponse {
    pub runs: Vec<AgentRunResponse>,
}
impl_restate_serde!(AgentRunListResponse);

// =============================================================================
// Service definition
// =============================================================================

#[restate_sdk::service]
#[name = "Agents"]
pub trait AgentsService {
    // CRUD - Agents
    async fn list_agents(req: ListAgentsRequest) -> Result<AgentListResponse, HandlerError>;
    async fn get_agent(req: AgentIdRequest) -> Result<AgentDetailResponse, HandlerError>;
    async fn create_agent(req: CreateAgentRequest) -> Result<AgentResponse, HandlerError>;
    async fn update_agent(req: UpdateAgentRequest) -> Result<AgentResponse, HandlerError>;
    async fn set_agent_status(req: SetAgentStatusRequest) -> Result<AgentResponse, HandlerError>;

    // Curator config
    async fn update_curator_config(
        req: UpdateCuratorConfigRequest,
    ) -> Result<CuratorConfigResponse, HandlerError>;

    // Search queries
    async fn list_search_queries(
        req: AgentIdRequest,
    ) -> Result<SearchQueryListResponse, HandlerError>;
    async fn create_search_query(
        req: CreateSearchQueryRequest,
    ) -> Result<SearchQueryResponse, HandlerError>;
    async fn update_search_query(
        req: UpdateSearchQueryRequest,
    ) -> Result<SearchQueryResponse, HandlerError>;
    async fn delete_search_query(req: DeleteRequest) -> Result<EmptyResponse, HandlerError>;

    // Filter rules
    async fn list_filter_rules(
        req: AgentIdRequest,
    ) -> Result<FilterRuleListResponse, HandlerError>;
    async fn create_filter_rule(
        req: CreateFilterRuleRequest,
    ) -> Result<FilterRuleResponse, HandlerError>;
    async fn update_filter_rule(
        req: UpdateFilterRuleRequest,
    ) -> Result<FilterRuleResponse, HandlerError>;
    async fn delete_filter_rule(req: DeleteRequest) -> Result<EmptyResponse, HandlerError>;

    // Required tag kinds
    async fn set_required_tag_kinds(
        req: SetRequiredTagKindsRequest,
    ) -> Result<TagKindListResponse, HandlerError>;

    // Pipeline triggers
    async fn run_agent_step(req: RunAgentStepRequest) -> Result<AgentRunResponse, HandlerError>;

    // Runs
    async fn list_runs(req: ListRunsRequest) -> Result<AgentRunListResponse, HandlerError>;
}

pub struct AgentsServiceImpl {
    deps: Arc<ServerDeps>,
}

impl AgentsServiceImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl AgentsService for AgentsServiceImpl {
    // =========================================================================
    // CRUD - Agents
    // =========================================================================

    async fn list_agents(
        &self,
        ctx: Context<'_>,
        req: ListAgentsRequest,
    ) -> Result<AgentListResponse, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let agents = if let Some(role) = &req.role {
            Agent::find_by_role(role, &self.deps.db_pool).await
        } else {
            Agent::find_all(&self.deps.db_pool).await
        }
        .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(AgentListResponse {
            agents: agents.into_iter().map(AgentResponse::from).collect(),
        })
    }

    async fn get_agent(
        &self,
        ctx: Context<'_>,
        req: AgentIdRequest,
    ) -> Result<AgentDetailResponse, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let pool = &self.deps.db_pool;

        let agent = Agent::find_by_id(req.agent_id, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let assistant_config = if agent.role == "assistant" {
            AgentAssistantConfig::find_by_agent(agent.id, pool)
                .await
                .ok()
                .map(|c| AssistantConfigResponse {
                    preamble: c.preamble,
                    config_name: c.config_name,
                })
        } else {
            None
        };

        let curator_config = if agent.role == "curator" {
            AgentCuratorConfig::find_by_agent(agent.id, pool)
                .await
                .ok()
                .map(|c| CuratorConfigResponse {
                    purpose: c.purpose,
                    audience_roles: c.audience_roles,
                    schedule_discover: c.schedule_discover,
                    schedule_monitor: c.schedule_monitor,
                })
        } else {
            None
        };

        let search_queries = AgentSearchQuery::find_by_agent(agent.id, pool)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|q| SearchQueryResponse {
                id: q.id,
                query_text: q.query_text,
                is_active: q.is_active,
                sort_order: q.sort_order,
            })
            .collect();

        let filter_rules = AgentFilterRule::find_by_agent(agent.id, pool)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|r| FilterRuleResponse {
                id: r.id,
                rule_text: r.rule_text,
                is_active: r.is_active,
                sort_order: r.sort_order,
            })
            .collect();

        let required = AgentRequiredTagKind::find_by_agent(agent.id, pool)
            .await
            .unwrap_or_default();
        let mut required_tag_kinds = Vec::new();
        for req_tk in &required {
            if let Ok(kind) = TagKindConfig::find_by_id(req_tk.tag_kind_id, pool).await {
                required_tag_kinds.push(TagKindResponse {
                    id: kind.id,
                    slug: kind.slug,
                    display_name: kind.display_name,
                });
            }
        }

        let agent_websites_raw = AgentWebsite::find_by_agent(agent.id, pool)
            .await
            .unwrap_or_default();
        let mut websites = Vec::new();
        for aw in &agent_websites_raw {
            use crate::domains::website::models::Website;
            let domain = Website::find_by_id(aw.website_id.into(), pool)
                .await
                .ok()
                .map(|w| w.domain);
            websites.push(AgentWebsiteResponse {
                website_id: aw.website_id,
                domain,
                discovered_at: aw.discovered_at.to_rfc3339(),
            });
        }

        Ok(AgentDetailResponse {
            agent: AgentResponse::from(agent),
            assistant_config,
            curator_config,
            search_queries,
            filter_rules,
            required_tag_kinds,
            websites,
        })
    }

    async fn create_agent(
        &self,
        ctx: Context<'_>,
        req: CreateAgentRequest,
    ) -> Result<AgentResponse, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let pool = &self.deps.db_pool;

        let response = ctx
            .run(|| async {
                let agent = Agent::create(&req.display_name, &req.role, pool)
                    .await
                    .map_err(|e| TerminalError::new(e.to_string()))?;

                // Create role-specific config
                match req.role.as_str() {
                    "assistant" => {
                        let preamble = req.preamble.as_deref().unwrap_or("");
                        let config_name = req.config_name.as_deref().unwrap_or("custom");
                        AgentAssistantConfig::create(agent.id, preamble, config_name, pool)
                            .await
                            .map_err(|e| TerminalError::new(e.to_string()))?;
                    }
                    "curator" => {
                        let purpose = req.purpose.as_deref().unwrap_or("");
                        AgentCuratorConfig::create(agent.id, purpose, pool)
                            .await
                            .map_err(|e| TerminalError::new(e.to_string()))?;
                    }
                    _ => {}
                }

                Ok(AgentResponse::from(agent))
            })
            .await?;

        Ok(response)
    }

    async fn update_agent(
        &self,
        ctx: Context<'_>,
        req: UpdateAgentRequest,
    ) -> Result<AgentResponse, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let response = ctx
            .run(|| async {
                let agent = if let Some(name) = &req.display_name {
                    Agent::update_display_name(req.agent_id, name, &self.deps.db_pool)
                        .await
                        .map_err(|e| TerminalError::new(e.to_string()))?
                } else {
                    Agent::find_by_id(req.agent_id, &self.deps.db_pool)
                        .await
                        .map_err(|e| TerminalError::new(e.to_string()))?
                };
                Ok(AgentResponse::from(agent))
            })
            .await?;

        Ok(response)
    }

    async fn set_agent_status(
        &self,
        ctx: Context<'_>,
        req: SetAgentStatusRequest,
    ) -> Result<AgentResponse, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let response = ctx
            .run(|| async {
                let agent = Agent::set_status(req.agent_id, &req.status, &self.deps.db_pool)
                    .await
                    .map_err(|e| TerminalError::new(e.to_string()))?;
                Ok(AgentResponse::from(agent))
            })
            .await?;

        Ok(response)
    }

    // =========================================================================
    // Curator config
    // =========================================================================

    async fn update_curator_config(
        &self,
        ctx: Context<'_>,
        req: UpdateCuratorConfigRequest,
    ) -> Result<CuratorConfigResponse, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let response = ctx
            .run(|| async {
                let config = AgentCuratorConfig::update(
                    req.agent_id,
                    &req.purpose,
                    &req.audience_roles,
                    req.schedule_discover.as_deref(),
                    req.schedule_monitor.as_deref(),
                    &self.deps.db_pool,
                )
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

                Ok(CuratorConfigResponse {
                    purpose: config.purpose,
                    audience_roles: config.audience_roles,
                    schedule_discover: config.schedule_discover,
                    schedule_monitor: config.schedule_monitor,
                })
            })
            .await?;

        Ok(response)
    }

    // =========================================================================
    // Search queries
    // =========================================================================

    async fn list_search_queries(
        &self,
        ctx: Context<'_>,
        req: AgentIdRequest,
    ) -> Result<SearchQueryListResponse, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let queries = AgentSearchQuery::find_by_agent(req.agent_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(SearchQueryListResponse {
            queries: queries
                .into_iter()
                .map(|q| SearchQueryResponse {
                    id: q.id,
                    query_text: q.query_text,
                    is_active: q.is_active,
                    sort_order: q.sort_order,
                })
                .collect(),
        })
    }

    async fn create_search_query(
        &self,
        ctx: Context<'_>,
        req: CreateSearchQueryRequest,
    ) -> Result<SearchQueryResponse, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let response = ctx
            .run(|| async {
                let q = AgentSearchQuery::create(
                    req.agent_id,
                    &req.query_text,
                    req.sort_order.unwrap_or(0),
                    &self.deps.db_pool,
                )
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

                Ok(SearchQueryResponse {
                    id: q.id,
                    query_text: q.query_text,
                    is_active: q.is_active,
                    sort_order: q.sort_order,
                })
            })
            .await?;

        Ok(response)
    }

    async fn update_search_query(
        &self,
        ctx: Context<'_>,
        req: UpdateSearchQueryRequest,
    ) -> Result<SearchQueryResponse, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let response = ctx
            .run(|| async {
                let q = AgentSearchQuery::update(
                    req.id,
                    &req.query_text,
                    req.sort_order.unwrap_or(0),
                    &self.deps.db_pool,
                )
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

                Ok(SearchQueryResponse {
                    id: q.id,
                    query_text: q.query_text,
                    is_active: q.is_active,
                    sort_order: q.sort_order,
                })
            })
            .await?;

        Ok(response)
    }

    async fn delete_search_query(
        &self,
        ctx: Context<'_>,
        req: DeleteRequest,
    ) -> Result<EmptyResponse, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        ctx.run(|| async {
            AgentSearchQuery::delete(req.id, &self.deps.db_pool)
                .await
                .map_err(Into::into)
        })
        .await?;

        Ok(EmptyResponse {})
    }

    // =========================================================================
    // Filter rules
    // =========================================================================

    async fn list_filter_rules(
        &self,
        ctx: Context<'_>,
        req: AgentIdRequest,
    ) -> Result<FilterRuleListResponse, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let rules = AgentFilterRule::find_by_agent(req.agent_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(FilterRuleListResponse {
            rules: rules
                .into_iter()
                .map(|r| FilterRuleResponse {
                    id: r.id,
                    rule_text: r.rule_text,
                    is_active: r.is_active,
                    sort_order: r.sort_order,
                })
                .collect(),
        })
    }

    async fn create_filter_rule(
        &self,
        ctx: Context<'_>,
        req: CreateFilterRuleRequest,
    ) -> Result<FilterRuleResponse, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let response = ctx
            .run(|| async {
                let r = AgentFilterRule::create(
                    req.agent_id,
                    &req.rule_text,
                    req.sort_order.unwrap_or(0),
                    &self.deps.db_pool,
                )
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

                Ok(FilterRuleResponse {
                    id: r.id,
                    rule_text: r.rule_text,
                    is_active: r.is_active,
                    sort_order: r.sort_order,
                })
            })
            .await?;

        Ok(response)
    }

    async fn update_filter_rule(
        &self,
        ctx: Context<'_>,
        req: UpdateFilterRuleRequest,
    ) -> Result<FilterRuleResponse, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let response = ctx
            .run(|| async {
                let r = AgentFilterRule::update(
                    req.id,
                    &req.rule_text,
                    req.sort_order.unwrap_or(0),
                    &self.deps.db_pool,
                )
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

                Ok(FilterRuleResponse {
                    id: r.id,
                    rule_text: r.rule_text,
                    is_active: r.is_active,
                    sort_order: r.sort_order,
                })
            })
            .await?;

        Ok(response)
    }

    async fn delete_filter_rule(
        &self,
        ctx: Context<'_>,
        req: DeleteRequest,
    ) -> Result<EmptyResponse, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        ctx.run(|| async {
            AgentFilterRule::delete(req.id, &self.deps.db_pool)
                .await
                .map_err(Into::into)
        })
        .await?;

        Ok(EmptyResponse {})
    }

    // =========================================================================
    // Required tag kinds
    // =========================================================================

    async fn set_required_tag_kinds(
        &self,
        ctx: Context<'_>,
        req: SetRequiredTagKindsRequest,
    ) -> Result<TagKindListResponse, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let pool = &self.deps.db_pool;

        ctx.run(|| async {
            AgentRequiredTagKind::set_for_agent(req.agent_id, &req.tag_kind_ids, pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;
            Ok(())
        })
        .await?;

        // Fetch the tag kind details outside ctx.run() (read-only)
        let required = AgentRequiredTagKind::find_by_agent(req.agent_id, pool)
            .await
            .unwrap_or_default();
        let mut tag_kinds = Vec::new();
        for r in &required {
            if let Ok(kind) = TagKindConfig::find_by_id(r.tag_kind_id, pool).await {
                tag_kinds.push(TagKindResponse {
                    id: kind.id,
                    slug: kind.slug,
                    display_name: kind.display_name,
                });
            }
        }

        Ok(TagKindListResponse { tag_kinds })
    }

    // =========================================================================
    // Pipeline triggers
    // =========================================================================

    async fn run_agent_step(
        &self,
        ctx: Context<'_>,
        req: RunAgentStepRequest,
    ) -> Result<AgentRunResponse, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let pool = &self.deps.db_pool;

        info!(agent_id = %req.agent_id, step = %req.step, "Running agent step");

        // Run the pipeline step inside ctx.run(), build response inside the closure
        let response = ctx
            .run(|| async {
                let run = match req.step.as_str() {
                    "discover" => discover::discover(req.agent_id, "manual", &self.deps).await,
                    "extract" => extract::extract(req.agent_id, "manual", &self.deps).await,
                    "enrich" => enrich::enrich(req.agent_id, "manual", &self.deps).await,
                    "monitor" => monitor::monitor(req.agent_id, "manual", &self.deps).await,
                    _ => return Err(TerminalError::new(format!("Unknown step: {}", req.step)).into()),
                }
                .map_err(|e| TerminalError::new(e.to_string()))?;

                let stats = AgentRunStat::find_by_run(run.id, pool)
                    .await
                    .unwrap_or_default();

                Ok(AgentRunResponse {
                    id: run.id,
                    step: run.step,
                    trigger_type: run.trigger_type,
                    status: run.status,
                    started_at: run.started_at.to_rfc3339(),
                    completed_at: run.completed_at.map(|t| t.to_rfc3339()),
                    stats: stats
                        .into_iter()
                        .map(|s| RunStatResponse {
                            stat_key: s.stat_key,
                            stat_value: s.stat_value,
                        })
                        .collect(),
                })
            })
            .await?;

        Ok(response)
    }

    // =========================================================================
    // Runs
    // =========================================================================

    async fn list_runs(
        &self,
        ctx: Context<'_>,
        req: ListRunsRequest,
    ) -> Result<AgentRunListResponse, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let pool = &self.deps.db_pool;

        let runs = AgentRun::find_recent(req.agent_id, req.limit.unwrap_or(20), pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let mut responses = Vec::new();
        for run in runs {
            let stats = AgentRunStat::find_by_run(run.id, pool)
                .await
                .unwrap_or_default();
            responses.push(AgentRunResponse {
                id: run.id,
                step: run.step,
                trigger_type: run.trigger_type,
                status: run.status,
                started_at: run.started_at.to_rfc3339(),
                completed_at: run.completed_at.map(|t| t.to_rfc3339()),
                stats: stats
                    .into_iter()
                    .map(|s| RunStatResponse {
                        stat_key: s.stat_key,
                        stat_value: s.stat_value,
                    })
                    .collect(),
            });
        }

        Ok(AgentRunListResponse { runs: responses })
    }
}
