//! Provider virtual object
//!
//! Keyed by provider_id. Per-provider serialized writes, concurrent reads.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::common::auth::restate_auth::require_admin;
use crate::common::EmptyRequest;
use crate::domains::providers::activities;
use crate::domains::providers::models::Provider;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProviderRequest {
    pub name: Option<String>,
    pub bio: Option<String>,
    pub headline: Option<String>,
    pub location: Option<String>,
}

impl_restate_serde!(UpdateProviderRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectProviderRequest {
    pub reason: String,
}

impl_restate_serde!(RejectProviderRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagRequest {
    pub tag_kind: String,
    pub tag_value: String,
    pub display_name: Option<String>,
}

impl_restate_serde!(TagRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveTagRequest {
    pub tag_id: String,
}

impl_restate_serde!(RemoveTagRequest);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResult {
    pub id: Uuid,
    pub name: String,
    pub status: String,
}

impl_restate_serde!(ProviderResult);

impl From<Provider> for ProviderResult {
    fn from(p: Provider) -> Self {
        Self {
            id: p.id.into_uuid(),
            name: p.name,
            status: p.status,
        }
    }
}

// =============================================================================
// Virtual object definition
// =============================================================================

#[restate_sdk::object]
#[name = "Provider"]
pub trait ProviderObject {
    async fn update(req: UpdateProviderRequest) -> Result<ProviderResult, HandlerError>;
    async fn approve(req: EmptyRequest) -> Result<ProviderResult, HandlerError>;
    async fn reject(req: RejectProviderRequest) -> Result<ProviderResult, HandlerError>;
    async fn delete(req: EmptyRequest) -> Result<(), HandlerError>;
    async fn add_tag(req: TagRequest) -> Result<(), HandlerError>;
    async fn remove_tag(req: RemoveTagRequest) -> Result<(), HandlerError>;

    #[shared]
    async fn get(req: EmptyRequest) -> Result<ProviderResult, HandlerError>;
}

pub struct ProviderObjectImpl {
    deps: Arc<ServerDeps>,
}

impl ProviderObjectImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl ProviderObject for ProviderObjectImpl {
    async fn update(
        &self,
        ctx: ObjectContext<'_>,
        req: UpdateProviderRequest,
    ) -> Result<ProviderResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let provider_id = ctx.key().to_string();

        use crate::domains::providers::data::UpdateProviderInput;
        let input = UpdateProviderInput {
            name: req.name,
            bio: req.bio,
            why_statement: None,
            headline: req.headline,
            profile_image_url: None,
            location: req.location,
            latitude: None,
            longitude: None,
            service_radius_km: None,
            offers_in_person: None,
            offers_remote: None,
            accepting_clients: None,
        };

        ctx.run(|| async {
            activities::update_provider(provider_id.clone(), input.clone(), &self.deps)
                .await
                .map(|_| ())
                .map_err(Into::into)
        })
        .await?;

        let provider = activities::get_provider(provider_id, &self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
            .ok_or_else(|| TerminalError::new("Provider not found after update"))?;

        Ok(ProviderResult::from(provider))
    }

    async fn approve(
        &self,
        ctx: ObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<ProviderResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let provider_id = ctx.key().to_string();

        ctx.run(|| async {
            activities::approve_provider(
                provider_id.clone(),
                user.member_id.into_uuid(),
                &self.deps,
            )
            .await
            .map(|_| ())
            .map_err(Into::into)
        })
        .await?;

        let provider = activities::get_provider(provider_id, &self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
            .ok_or_else(|| TerminalError::new("Provider not found"))?;

        Ok(ProviderResult::from(provider))
    }

    async fn reject(
        &self,
        ctx: ObjectContext<'_>,
        req: RejectProviderRequest,
    ) -> Result<ProviderResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let provider_id = ctx.key().to_string();

        ctx.run(|| async {
            activities::reject_provider(
                provider_id.clone(),
                req.reason.clone(),
                user.member_id.into_uuid(),
                &self.deps,
            )
            .await
            .map(|_| ())
            .map_err(Into::into)
        })
        .await?;

        let provider = activities::get_provider(provider_id, &self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
            .ok_or_else(|| TerminalError::new("Provider not found"))?;

        Ok(ProviderResult::from(provider))
    }

    async fn delete(
        &self,
        ctx: ObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<(), HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let provider_id = ctx.key().to_string();

        ctx.run(|| async {
            activities::delete_provider(provider_id.clone(), &self.deps)
                .await
                .map_err(Into::into)
        })
        .await?;

        Ok(())
    }

    async fn add_tag(
        &self,
        ctx: ObjectContext<'_>,
        req: TagRequest,
    ) -> Result<(), HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let provider_id = ctx.key().to_string();

        ctx.run(|| async {
            activities::add_provider_tag(
                provider_id.clone(),
                req.tag_kind.clone(),
                req.tag_value.clone(),
                req.display_name.clone(),
                &self.deps,
            )
            .await
            .map(|_| ())
            .map_err(Into::into)
        })
        .await?;

        Ok(())
    }

    async fn remove_tag(
        &self,
        ctx: ObjectContext<'_>,
        req: RemoveTagRequest,
    ) -> Result<(), HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let provider_id = ctx.key().to_string();

        ctx.run(|| async {
            activities::remove_provider_tag(
                provider_id.clone(),
                req.tag_id.clone(),
                &self.deps,
            )
            .await
            .map_err(Into::into)
        })
        .await?;

        Ok(())
    }

    async fn get(
        &self,
        ctx: SharedObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<ProviderResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let provider_id = ctx.key().to_string();

        let provider = activities::get_provider(provider_id, &self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
            .ok_or_else(|| TerminalError::new("Provider not found"))?;

        Ok(ProviderResult::from(provider))
    }
}
