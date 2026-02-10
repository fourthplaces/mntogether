//! Source virtual object
//!
//! Keyed by source_id. Per-source serialized writes, concurrent reads.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::common::auth::restate_auth::require_admin;
use crate::common::{EmptyRequest, OrganizationId, SourceId, WebsiteId};
use crate::domains::source::models::{get_source_identifier, Source, WebsiteSource};
use crate::domains::website::activities;
use crate::domains::website::models::WebsiteAssessment;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectSourceRequest {
    pub reason: String,
}

impl_restate_serde!(RejectSourceRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspendSourceRequest {
    pub reason: String,
}

impl_restate_serde!(SuspendSourceRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignOrganizationRequest {
    pub organization_id: Uuid,
}

impl_restate_serde!(AssignOrganizationRequest);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceObjectResult {
    pub id: Uuid,
    pub source_type: String,
    pub identifier: String,
    pub url: Option<String>,
    pub status: String,
    pub active: bool,
    pub created_at: Option<String>,
    pub last_scraped_at: Option<String>,
    pub organization_id: Option<String>,
}

impl_restate_serde!(SourceObjectResult);

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
#[name = "Source"]
pub trait SourceObject {
    async fn approve(req: EmptyRequest) -> Result<SourceObjectResult, HandlerError>;
    async fn reject(req: RejectSourceRequest) -> Result<SourceObjectResult, HandlerError>;
    async fn suspend(req: SuspendSourceRequest) -> Result<SourceObjectResult, HandlerError>;
    async fn assign_organization(
        req: AssignOrganizationRequest,
    ) -> Result<SourceObjectResult, HandlerError>;
    async fn unassign_organization(
        req: EmptyRequest,
    ) -> Result<SourceObjectResult, HandlerError>;
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

    #[shared]
    async fn get(req: EmptyRequest) -> Result<SourceObjectResult, HandlerError>;

    #[shared]
    async fn get_assessment(req: EmptyRequest) -> Result<OptionalAssessmentResult, HandlerError>;
}

pub struct SourceObjectImpl {
    deps: Arc<ServerDeps>,
}

impl SourceObjectImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }

    fn parse_source_id(key: &str) -> Result<Uuid, HandlerError> {
        Uuid::parse_str(key)
            .map_err(|e| TerminalError::new(format!("Invalid source ID: {}", e)).into())
    }

    async fn build_result(source: Source, pool: &sqlx::PgPool) -> Result<SourceObjectResult, HandlerError> {
        let identifier = get_source_identifier(source.id, pool)
            .await
            .unwrap_or_else(|_| "unknown".to_string());

        Ok(SourceObjectResult {
            id: source.id.into_uuid(),
            source_type: source.source_type,
            identifier,
            url: source.url,
            status: source.status,
            active: source.active,
            created_at: Some(source.created_at.to_rfc3339()),
            last_scraped_at: source.last_scraped_at.map(|dt| dt.to_rfc3339()),
            organization_id: source.organization_id.map(|id| id.to_string()),
        })
    }
}

impl SourceObject for SourceObjectImpl {
    async fn approve(
        &self,
        ctx: ObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<SourceObjectResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let source_id = Self::parse_source_id(ctx.key())?;

        ctx.run(|| async {
            Source::approve(
                SourceId::from_uuid(source_id),
                user.member_id,
                &self.deps.db_pool,
            )
            .await
            .map(|_| ())
            .map_err(Into::into)
        })
        .await?;

        let source = Source::find_by_id(SourceId::from_uuid(source_id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Self::build_result(source, &self.deps.db_pool).await
    }

    async fn reject(
        &self,
        ctx: ObjectContext<'_>,
        req: RejectSourceRequest,
    ) -> Result<SourceObjectResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let source_id = Self::parse_source_id(ctx.key())?;

        ctx.run(|| async {
            Source::reject(
                SourceId::from_uuid(source_id),
                user.member_id,
                req.reason.clone(),
                &self.deps.db_pool,
            )
            .await
            .map(|_| ())
            .map_err(Into::into)
        })
        .await?;

        let source = Source::find_by_id(SourceId::from_uuid(source_id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Self::build_result(source, &self.deps.db_pool).await
    }

    async fn suspend(
        &self,
        ctx: ObjectContext<'_>,
        req: SuspendSourceRequest,
    ) -> Result<SourceObjectResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let source_id = Self::parse_source_id(ctx.key())?;

        ctx.run(|| async {
            Source::suspend(
                SourceId::from_uuid(source_id),
                user.member_id,
                req.reason.clone(),
                &self.deps.db_pool,
            )
            .await
            .map(|_| ())
            .map_err(Into::into)
        })
        .await?;

        let source = Source::find_by_id(SourceId::from_uuid(source_id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Self::build_result(source, &self.deps.db_pool).await
    }

    async fn assign_organization(
        &self,
        ctx: ObjectContext<'_>,
        req: AssignOrganizationRequest,
    ) -> Result<SourceObjectResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let source_id = Self::parse_source_id(ctx.key())?;

        let source = Source::set_organization_id(
            SourceId::from_uuid(source_id),
            OrganizationId::from(req.organization_id),
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        Self::build_result(source, &self.deps.db_pool).await
    }

    async fn unassign_organization(
        &self,
        ctx: ObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<SourceObjectResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let source_id = Self::parse_source_id(ctx.key())?;

        let source = Source::unset_organization_id(
            SourceId::from_uuid(source_id),
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        Self::build_result(source, &self.deps.db_pool).await
    }

    async fn generate_assessment(
        &self,
        ctx: ObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<GenerateAssessmentResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let source_id = Self::parse_source_id(ctx.key())?;

        // Only websites have assessments
        let _ws = WebsiteSource::find_by_source_id(SourceId::from_uuid(source_id), &self.deps.db_pool)
            .await
            .map_err(|_| TerminalError::new("Assessments are only available for website sources"))?;

        let result = activities::approval::assess_website(
            source_id,
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
            if let Some(ref message) = result.message {
                if let Some(research_id_str) = message.strip_prefix("research_id:") {
                    if let Ok(research_id) = Uuid::parse_str(research_id_str) {
                        let _ = ctx
                            .workflow_client::<crate::domains::website::restate::workflows::research::WebsiteResearchWorkflowClient>(
                                research_id.to_string(),
                            )
                            .run(crate::domains::website::restate::WebsiteResearchRequest {
                                research_id,
                                website_id: source_id,
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

    async fn regenerate_posts(
        &self,
        ctx: ObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<RegeneratePostsResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let source_id = Self::parse_source_id(ctx.key())?;

        let source = Source::find_by_id(SourceId::from_uuid(source_id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let workflow_id = format!("regen-{}-{}", source_id, chrono::Utc::now().timestamp());

        match source.source_type.as_str() {
            "website" => {
                let _ = ctx
                    .workflow_client::<crate::domains::website::restate::workflows::regenerate_posts::RegeneratePostsWorkflowClient>(
                        workflow_id.clone(),
                    )
                    .run(crate::domains::website::restate::workflows::regenerate_posts::RegeneratePostsRequest { website_id: source_id })
                    .send();
            }
            "instagram" | "facebook" | "tiktok" => {
                let _ = ctx
                    .workflow_client::<crate::domains::source::restate::workflows::regenerate_social_posts::RegenerateSocialPostsWorkflowClient>(
                        workflow_id.clone(),
                    )
                    .run(crate::domains::source::restate::workflows::regenerate_social_posts::RegenerateSocialPostsRequest { source_id })
                    .send();
            }
            other => {
                return Err(TerminalError::new(format!(
                    "Post regeneration is not supported for source type '{}'",
                    other
                ))
                .into());
            }
        }

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
        let source_id = Self::parse_source_id(ctx.key())?;

        // Determine source type for dedup
        let source = Source::find_by_id(SourceId::from_uuid(source_id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let workflow_id = format!("dedup-{}-{}", source_id, chrono::Utc::now().timestamp());
        let _ = ctx
            .workflow_client::<crate::domains::posts::restate::workflows::deduplicate_posts::DeduplicatePostsWorkflowClient>(
                workflow_id.clone(),
            )
            .run(crate::domains::posts::restate::workflows::deduplicate_posts::DeduplicatePostsRequest {
                source_type: source.source_type,
                source_id,
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
        let source_id = Self::parse_source_id(ctx.key())?;

        // Only websites support org extraction
        let _ws = WebsiteSource::find_by_source_id(SourceId::from_uuid(source_id), &self.deps.db_pool)
            .await
            .map_err(|_| TerminalError::new("Organization extraction is only available for website sources"))?;

        match crate::domains::crawling::activities::extract_and_create_organization(
            WebsiteId::from_uuid(source_id),
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

    async fn get(
        &self,
        ctx: SharedObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<SourceObjectResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let source_id = Uuid::parse_str(ctx.key())
            .map_err(|e| TerminalError::new(format!("Invalid source ID: {}", e)))?;

        let source = Source::find_by_id(SourceId::from_uuid(source_id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Self::build_result(source, &self.deps.db_pool).await
    }

    async fn get_assessment(
        &self,
        ctx: SharedObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<OptionalAssessmentResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let source_id = Uuid::parse_str(ctx.key())
            .map_err(|e| TerminalError::new(format!("Invalid source ID: {}", e)))?;

        let assessment = WebsiteAssessment::find_latest_by_website_id(
            source_id,
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
}
