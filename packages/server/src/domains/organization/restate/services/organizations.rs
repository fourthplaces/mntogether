use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::auth::restate_auth::require_admin;
use crate::common::{EmptyRequest, OrganizationId, WebsiteId};
use crate::domains::crawling::activities::extract_and_create_organization;
use crate::domains::organization::models::Organization;
use crate::domains::website::models::Website;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrganizationRequest {
    pub name: String,
    pub description: Option<String>,
}

impl_restate_serde!(CreateOrganizationRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOrganizationRequest {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

impl_restate_serde!(UpdateOrganizationRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetOrganizationRequest {
    pub id: Uuid,
}

impl_restate_serde!(GetOrganizationRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteOrganizationRequest {
    pub id: Uuid,
}

impl_restate_serde!(DeleteOrganizationRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegenerateOrganizationRequest {
    pub id: Uuid,
}

impl_restate_serde!(RegenerateOrganizationRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegenerateOrganizationResult {
    pub organization_id: Option<String>,
    pub websites_processed: i64,
    pub status: String,
}

impl_restate_serde!(RegenerateOrganizationResult);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationResult {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub website_count: i64,
    pub social_profile_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl_restate_serde!(OrganizationResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationListResult {
    pub organizations: Vec<OrganizationResult>,
}

impl_restate_serde!(OrganizationListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillResult {
    pub processed: i64,
    pub succeeded: i64,
    pub failed: i64,
}

impl_restate_serde!(BackfillResult);

// =============================================================================
// Service definition
// =============================================================================

#[restate_sdk::service]
#[name = "Organizations"]
pub trait OrganizationsService {
    async fn list(req: EmptyRequest) -> Result<OrganizationListResult, HandlerError>;
    async fn get(req: GetOrganizationRequest) -> Result<OrganizationResult, HandlerError>;
    async fn create(req: CreateOrganizationRequest) -> Result<OrganizationResult, HandlerError>;
    async fn update(req: UpdateOrganizationRequest) -> Result<OrganizationResult, HandlerError>;
    async fn delete(req: DeleteOrganizationRequest) -> Result<EmptyRequest, HandlerError>;
    async fn regenerate(
        req: RegenerateOrganizationRequest,
    ) -> Result<RegenerateOrganizationResult, HandlerError>;
    async fn backfill_organizations(req: EmptyRequest) -> Result<BackfillResult, HandlerError>;
}

pub struct OrganizationsServiceImpl {
    deps: Arc<ServerDeps>,
}

impl OrganizationsServiceImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl OrganizationsService for OrganizationsServiceImpl {
    async fn list(
        &self,
        ctx: Context<'_>,
        _req: EmptyRequest,
    ) -> Result<OrganizationListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let orgs = Organization::list(&self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let mut results = Vec::with_capacity(orgs.len());
        for org in orgs {
            let website_count = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM websites WHERE organization_id = $1",
            )
            .bind(org.id)
            .fetch_one(&self.deps.db_pool)
            .await
            .unwrap_or(0);

            let social_profile_count = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM social_profiles WHERE organization_id = $1",
            )
            .bind(org.id)
            .fetch_one(&self.deps.db_pool)
            .await
            .unwrap_or(0);

            results.push(OrganizationResult {
                id: org.id.to_string(),
                name: org.name,
                description: org.description,
                website_count,
                social_profile_count,
                created_at: org.created_at.to_rfc3339(),
                updated_at: org.updated_at.to_rfc3339(),
            });
        }

        Ok(OrganizationListResult {
            organizations: results,
        })
    }

    async fn get(
        &self,
        ctx: Context<'_>,
        req: GetOrganizationRequest,
    ) -> Result<OrganizationResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let org = Organization::find_by_id(OrganizationId::from(req.id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let website_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM websites WHERE organization_id = $1",
        )
        .bind(org.id)
        .fetch_one(&self.deps.db_pool)
        .await
        .unwrap_or(0);

        let social_profile_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM social_profiles WHERE organization_id = $1",
        )
        .bind(org.id)
        .fetch_one(&self.deps.db_pool)
        .await
        .unwrap_or(0);

        Ok(OrganizationResult {
            id: org.id.to_string(),
            name: org.name,
            description: org.description,
            website_count,
            social_profile_count,
            created_at: org.created_at.to_rfc3339(),
            updated_at: org.updated_at.to_rfc3339(),
        })
    }

    async fn create(
        &self,
        ctx: Context<'_>,
        req: CreateOrganizationRequest,
    ) -> Result<OrganizationResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let org = Organization::create(&req.name, req.description.as_deref(), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(OrganizationResult {
            id: org.id.to_string(),
            name: org.name,
            description: org.description,
            website_count: 0,
            social_profile_count: 0,
            created_at: org.created_at.to_rfc3339(),
            updated_at: org.updated_at.to_rfc3339(),
        })
    }

    async fn update(
        &self,
        ctx: Context<'_>,
        req: UpdateOrganizationRequest,
    ) -> Result<OrganizationResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let org = Organization::update(
            OrganizationId::from(req.id),
            &req.name,
            req.description.as_deref(),
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        let website_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM websites WHERE organization_id = $1",
        )
        .bind(org.id)
        .fetch_one(&self.deps.db_pool)
        .await
        .unwrap_or(0);

        let social_profile_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM social_profiles WHERE organization_id = $1",
        )
        .bind(org.id)
        .fetch_one(&self.deps.db_pool)
        .await
        .unwrap_or(0);

        Ok(OrganizationResult {
            id: org.id.to_string(),
            name: org.name,
            description: org.description,
            website_count,
            social_profile_count,
            created_at: org.created_at.to_rfc3339(),
            updated_at: org.updated_at.to_rfc3339(),
        })
    }

    async fn delete(
        &self,
        ctx: Context<'_>,
        req: DeleteOrganizationRequest,
    ) -> Result<EmptyRequest, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        Organization::delete(OrganizationId::from(req.id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(EmptyRequest {})
    }

    async fn regenerate(
        &self,
        ctx: Context<'_>,
        req: RegenerateOrganizationRequest,
    ) -> Result<RegenerateOrganizationResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let org_id = OrganizationId::from(req.id);
        let pool = &self.deps.db_pool;

        // Find all websites linked to this org before deleting
        let website_ids: Vec<WebsiteId> = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM websites WHERE organization_id = $1",
        )
        .bind(req.id)
        .fetch_all(pool)
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?
        .into_iter()
        .map(WebsiteId::from_uuid)
        .collect();

        let websites_processed = website_ids.len() as i64;

        if website_ids.is_empty() {
            return Err(TerminalError::new("No websites linked to this organization").into());
        }

        info!(org_id = %org_id, websites = websites_processed, "Regenerating organization: deleting and re-extracting");

        // Delete org — cascades: websites get org_id=NULL, social_profiles deleted
        Organization::delete(org_id, pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        // Re-run extraction on each website
        let mut new_org_id: Option<String> = None;
        for website_id in &website_ids {
            match extract_and_create_organization(*website_id, &self.deps).await {
                Ok(oid) => {
                    info!(website_id = %website_id, org_id = %oid, "Re-extraction succeeded");
                    new_org_id = Some(oid.into_uuid().to_string());
                }
                Err(e) => {
                    warn!(website_id = %website_id, error = %e, "Re-extraction failed for website");
                }
            }
        }

        Ok(RegenerateOrganizationResult {
            organization_id: new_org_id,
            websites_processed,
            status: "completed".to_string(),
        })
    }

    async fn backfill_organizations(
        &self,
        ctx: Context<'_>,
        _req: EmptyRequest,
    ) -> Result<BackfillResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let websites = Website::find_without_organization(&self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let processed = websites.len() as i64;
        let mut succeeded: i64 = 0;
        let mut failed: i64 = 0;

        info!(count = processed, "Starting organization backfill");

        for website in &websites {
            let website_id = website.id;
            match extract_and_create_organization(website_id, &self.deps).await {
                Ok(org_id) => {
                    info!(website_id = %website_id, org_id = %org_id, "Backfill: organization created");
                    succeeded += 1;
                }
                Err(e) => {
                    warn!(website_id = %website_id, error = %e, "Backfill: organization extraction failed");
                    failed += 1;
                }
            }
        }

        info!(processed, succeeded, failed, "Organization backfill complete");

        Ok(BackfillResult {
            processed,
            succeeded,
            failed,
        })
    }
}
