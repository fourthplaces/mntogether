//! Providers service (stateless)
//!
//! Cross-provider operations: list, submit.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::common::auth::restate_auth::{optional_auth, require_admin};
use crate::common::PaginationArgs;
use crate::domains::providers::activities;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

use crate::domains::providers::restate::virtual_objects::provider::ProviderResult;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProvidersRequest {
    pub status: Option<String>,
    pub first: Option<i32>,
    pub after: Option<String>,
    pub last: Option<i32>,
    pub before: Option<String>,
}

impl_restate_serde!(ListProvidersRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitProviderRequest {
    pub name: String,
    pub bio: Option<String>,
    pub headline: Option<String>,
    pub location: Option<String>,
}

impl_restate_serde!(SubmitProviderRequest);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderListResult {
    pub providers: Vec<ProviderResult>,
    pub total_count: i32,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

impl_restate_serde!(ProviderListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingProvidersResult {
    pub providers: Vec<ProviderResult>,
}

impl_restate_serde!(PendingProvidersResult);

// =============================================================================
// Service definition
// =============================================================================

#[restate_sdk::service]
#[name = "Providers"]
pub trait ProvidersService {
    async fn list(req: ListProvidersRequest) -> Result<ProviderListResult, HandlerError>;
    async fn list_pending(
        req: ListProvidersRequest,
    ) -> Result<PendingProvidersResult, HandlerError>;
    async fn submit(req: SubmitProviderRequest) -> Result<ProviderResult, HandlerError>;
}

pub struct ProvidersServiceImpl {
    deps: Arc<ServerDeps>,
}

impl ProvidersServiceImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl ProvidersService for ProvidersServiceImpl {
    async fn list(
        &self,
        ctx: Context<'_>,
        req: ListProvidersRequest,
    ) -> Result<ProviderListResult, HandlerError> {
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
            activities::get_providers_paginated(req.status.as_deref(), &validated, &self.deps)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(ProviderListResult {
            providers: connection
                .edges
                .into_iter()
                .filter_map(|e| {
                    uuid::Uuid::parse_str(&e.node.id)
                        .ok()
                        .map(|id| ProviderResult {
                            id,
                            name: e.node.name,
                            status: e.node.status,
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
        _req: ListProvidersRequest,
    ) -> Result<PendingProvidersResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let providers = activities::get_pending_providers(&self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(PendingProvidersResult {
            providers: providers.into_iter().map(ProviderResult::from).collect(),
        })
    }

    async fn submit(
        &self,
        ctx: Context<'_>,
        req: SubmitProviderRequest,
    ) -> Result<ProviderResult, HandlerError> {
        let user = optional_auth(ctx.headers(), &self.deps.jwt_service);

        use crate::domains::providers::data::SubmitProviderInput;
        let input = SubmitProviderInput {
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

        let id = ctx
            .run(|| async {
                activities::submit_provider(
                    input.clone(),
                    user.as_ref().map(|u| u.member_id.into_uuid()),
                    &self.deps,
                )
                .await
                .map_err(Into::into)
            })
            .await?;

        let provider = activities::get_provider(id.to_string(), &self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
            .ok_or_else(|| TerminalError::new("Provider not found after submit"))?;

        Ok(ProviderResult::from(provider))
    }
}
