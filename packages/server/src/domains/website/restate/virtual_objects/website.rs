//! Website virtual object
//!
//! Keyed by website_id. Per-website serialized writes, concurrent reads.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::common::auth::restate_auth::require_admin;
use crate::common::EmptyRequest;
use crate::common::{OrganizationId, WebsiteId};
use crate::domains::website::activities;
use crate::domains::website::models::{Website, WebsiteAssessment};
use crate::domains::website::restate::WebsiteResearchRequest;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectWebsiteRequest {
    pub reason: String,
}

impl_restate_serde!(RejectWebsiteRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspendWebsiteRequest {
    pub reason: String,
}

impl_restate_serde!(SuspendWebsiteRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignOrganizationRequest {
    pub organization_id: Uuid,
}

impl_restate_serde!(AssignOrganizationRequest);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteResult {
    pub id: Uuid,
    pub domain: String,
    pub status: String,
    pub active: bool,
    pub created_at: Option<String>,
    pub last_crawled_at: Option<String>,
    pub post_count: Option<i64>,
    pub organization_id: Option<String>,
}

impl_restate_serde!(WebsiteResult);

impl From<Website> for WebsiteResult {
    fn from(w: Website) -> Self {
        Self {
            id: w.id.into_uuid(),
            domain: w.domain,
            status: w.status,
            active: w.active,
            created_at: Some(w.created_at.to_rfc3339()),
            last_crawled_at: w.last_scraped_at.map(|dt| dt.to_rfc3339()),
            post_count: None,
            organization_id: w.organization_id.map(|id| id.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentResult {
    pub id: Uuid,
    pub website_id: Uuid,
    pub assessment_markdown: String,
    pub confidence_score: Option<f64>,
}

impl_restate_serde!(AssessmentResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionalAssessmentResult {
    pub assessment: Option<AssessmentResult>,
}

impl_restate_serde!(OptionalAssessmentResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateAssessmentResult {
    pub job_id: String,
    pub status: String,
    pub assessment_id: Option<String>,
}

impl_restate_serde!(GenerateAssessmentResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegeneratePostsResult {
    pub posts_created: i32,
    pub status: String,
}

impl_restate_serde!(RegeneratePostsResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicatePostsResult {
    pub status: String,
}

impl_restate_serde!(DeduplicatePostsResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractOrganizationResult {
    pub organization_id: Option<String>,
    pub status: String,
}

impl_restate_serde!(ExtractOrganizationResult);

// =============================================================================
// Virtual object definition
// =============================================================================

#[restate_sdk::object]
#[name = "Website"]
pub trait WebsiteObject {
    async fn approve(req: EmptyRequest) -> Result<WebsiteResult, HandlerError>;
    async fn reject(req: RejectWebsiteRequest) -> Result<WebsiteResult, HandlerError>;
    async fn suspend(req: SuspendWebsiteRequest) -> Result<WebsiteResult, HandlerError>;
    async fn generate_assessment(
        req: EmptyRequest,
    ) -> Result<GenerateAssessmentResult, HandlerError>;
    async fn regenerate_posts(
        req: EmptyRequest,
    ) -> Result<RegeneratePostsResult, HandlerError>;
    async fn deduplicate_posts(
        req: EmptyRequest,
    ) -> Result<DeduplicatePostsResult, HandlerError>;
    async fn extract_organization(
        req: EmptyRequest,
    ) -> Result<ExtractOrganizationResult, HandlerError>;
    async fn assign_organization(
        req: AssignOrganizationRequest,
    ) -> Result<WebsiteResult, HandlerError>;
    async fn unassign_organization(
        req: EmptyRequest,
    ) -> Result<WebsiteResult, HandlerError>;

    #[shared]
    async fn get(req: EmptyRequest) -> Result<WebsiteResult, HandlerError>;

    #[shared]
    async fn get_assessment(req: EmptyRequest) -> Result<OptionalAssessmentResult, HandlerError>;
}

pub struct WebsiteObjectImpl {
    deps: Arc<ServerDeps>,
}

impl WebsiteObjectImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }

    fn parse_website_id(key: &str) -> Result<Uuid, HandlerError> {
        Uuid::parse_str(key)
            .map_err(|e| TerminalError::new(format!("Invalid website ID: {}", e)).into())
    }
}

impl WebsiteObject for WebsiteObjectImpl {
    async fn approve(
        &self,
        ctx: ObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<WebsiteResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let website_id = Self::parse_website_id(ctx.key())?;

        ctx.run(|| async {
            activities::approve_website(
                WebsiteId::from_uuid(website_id),
                user.member_id,
                &self.deps,
            )
            .await
            .map(|_| ())
            .map_err(Into::into)
        })
        .await?;

        let website = Website::find_by_id(WebsiteId::from_uuid(website_id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(WebsiteResult::from(website))
    }

    async fn reject(
        &self,
        ctx: ObjectContext<'_>,
        req: RejectWebsiteRequest,
    ) -> Result<WebsiteResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let website_id = Self::parse_website_id(ctx.key())?;

        let id = ctx
            .run(|| async {
                activities::reject_website(
                    WebsiteId::from_uuid(website_id),
                    req.reason.clone(),
                    user.member_id,
                    &self.deps,
                )
                .await
                .map_err(Into::into)
            })
            .await?;

        let website = Website::find_by_id(id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(WebsiteResult::from(website))
    }

    async fn suspend(
        &self,
        ctx: ObjectContext<'_>,
        req: SuspendWebsiteRequest,
    ) -> Result<WebsiteResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let website_id = Self::parse_website_id(ctx.key())?;

        let id = ctx
            .run(|| async {
                activities::suspend_website(
                    WebsiteId::from_uuid(website_id),
                    req.reason.clone(),
                    user.member_id,
                    &self.deps,
                )
                .await
                .map_err(Into::into)
            })
            .await?;

        let website = Website::find_by_id(id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(WebsiteResult::from(website))
    }

    async fn generate_assessment(
        &self,
        ctx: ObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<GenerateAssessmentResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let website_id = Self::parse_website_id(ctx.key())?;

        let result = activities::approval::assess_website(
            website_id,
            user.member_id.into_uuid(),
            &self.deps,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        if result.status == "completed" {
            Ok(GenerateAssessmentResult {
                job_id: result.job_id.to_string(),
                status: "completed".to_string(),
                assessment_id: result.assessment_id.map(|id| id.to_string()),
            })
        } else {
            // Fire-and-forget the research workflow via Restate workflow client
            if let Some(ref message) = result.message {
                if let Some(research_id_str) = message.strip_prefix("research_id:") {
                    if let Ok(research_id) = Uuid::parse_str(research_id_str) {
                        let _ = ctx
                            .workflow_client::<crate::domains::website::restate::workflows::research::WebsiteResearchWorkflowClient>(
                                research_id.to_string(),
                            )
                            .run(WebsiteResearchRequest {
                                research_id,
                                website_id,
                                requested_by: user.member_id.into_uuid(),
                            })
                            .call()
                            .await;
                    }
                }
            }
            Ok(GenerateAssessmentResult {
                job_id: result.job_id.to_string(),
                status: "research_started".to_string(),
                assessment_id: None,
            })
        }
    }

    async fn get(
        &self,
        ctx: SharedObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<WebsiteResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let website_id = Uuid::parse_str(ctx.key())
            .map_err(|e| TerminalError::new(format!("Invalid website ID: {}", e)))?;

        let website = Website::find_by_id(WebsiteId::from_uuid(website_id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(WebsiteResult::from(website))
    }

    async fn get_assessment(
        &self,
        ctx: SharedObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<OptionalAssessmentResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let website_id = Uuid::parse_str(ctx.key())
            .map_err(|e| TerminalError::new(format!("Invalid website ID: {}", e)))?;

        let assessment = WebsiteAssessment::find_latest_by_website_id(
            website_id,
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(OptionalAssessmentResult {
            assessment: assessment.map(|a| AssessmentResult {
                id: a.id,
                website_id: a.website_id,
                assessment_markdown: a.assessment_markdown,
                confidence_score: a.confidence_score,
            }),
        })
    }

    async fn regenerate_posts(
        &self,
        ctx: ObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<RegeneratePostsResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let website_id = Self::parse_website_id(ctx.key())?;

        let workflow_id = format!("regen-{}-{}", website_id, chrono::Utc::now().timestamp());
        let _ = ctx
            .workflow_client::<crate::domains::website::restate::workflows::regenerate_posts::RegeneratePostsWorkflowClient>(
                workflow_id.clone(),
            )
            .run(crate::domains::website::restate::workflows::regenerate_posts::RegeneratePostsRequest { website_id })
            .send();

        Ok(RegeneratePostsResult {
            posts_created: 0,
            status: format!("started:{}", workflow_id),
        })
    }

    async fn deduplicate_posts(
        &self,
        ctx: ObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<DeduplicatePostsResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let website_id = Self::parse_website_id(ctx.key())?;

        let workflow_id = format!("dedup-{}-{}", website_id, chrono::Utc::now().timestamp());
        let _ = ctx
            .workflow_client::<crate::domains::posts::restate::workflows::deduplicate_posts::DeduplicatePostsWorkflowClient>(
                workflow_id.clone(),
            )
            .run(crate::domains::posts::restate::workflows::deduplicate_posts::DeduplicatePostsRequest {
                website_id,
            })
            .send();

        Ok(DeduplicatePostsResult {
            status: format!("started:{}", workflow_id),
        })
    }

    async fn extract_organization(
        &self,
        ctx: ObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<ExtractOrganizationResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let website_id = Self::parse_website_id(ctx.key())?;

        match crate::domains::crawling::activities::extract_and_create_organization(
            WebsiteId::from_uuid(website_id),
            &self.deps,
        )
        .await
        {
            Ok(org_id) => Ok(ExtractOrganizationResult {
                organization_id: Some(org_id.into_uuid().to_string()),
                status: "completed".to_string(),
            }),
            Err(e) => Err(
                TerminalError::new(format!("Organization extraction failed: {}", e)).into(),
            ),
        }
    }

    async fn assign_organization(
        &self,
        ctx: ObjectContext<'_>,
        req: AssignOrganizationRequest,
    ) -> Result<WebsiteResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let website_id = Self::parse_website_id(ctx.key())?;

        let website = Website::set_organization_id(
            WebsiteId::from_uuid(website_id),
            OrganizationId::from(req.organization_id),
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(WebsiteResult::from(website))
    }

    async fn unassign_organization(
        &self,
        ctx: ObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<WebsiteResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let website_id = Self::parse_website_id(ctx.key())?;

        let website = Website::unset_organization_id(
            WebsiteId::from_uuid(website_id),
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(WebsiteResult::from(website))
    }
}
