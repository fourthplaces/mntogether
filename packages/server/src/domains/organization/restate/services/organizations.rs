use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::common::auth::restate_auth::require_admin;
use crate::common::{EmptyRequest, OrganizationId};
use crate::domains::organization::data::OrganizationData;
use crate::domains::organization::models::Organization;
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
}
