//! Provider query actions
//!
//! All provider read operations go through these actions via `engine.activate().process()`.
//! Actions are self-contained: they handle ID parsing and return final models.

use anyhow::{Context, Result};
use seesaw_core::EffectContext;
use tracing::info;

use crate::common::{AppState, ProviderId};
use crate::domains::providers::models::Provider;
use crate::kernel::ServerDeps;

/// Get a single provider by ID
pub async fn get_provider(
    provider_id: String,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Option<Provider>> {
    let id = ProviderId::parse(&provider_id).context("Invalid provider ID")?;

    info!(provider_id = %id, "Getting provider");

    Provider::find_by_id_optional(id, &ctx.deps().db_pool).await
}

/// Get all providers with optional filters
pub async fn get_providers(
    status: Option<String>,
    accepting_clients: Option<bool>,
    limit: Option<i32>,
    offset: Option<i32>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Vec<Provider>> {
    info!(
        status = ?status,
        accepting_clients = ?accepting_clients,
        "Getting providers with filters"
    );

    Provider::find_with_filters(
        status.as_deref(),
        accepting_clients,
        limit,
        offset,
        &ctx.deps().db_pool,
    )
    .await
}

/// Get all pending providers (for admin approval queue)
pub async fn get_pending_providers(
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Vec<Provider>> {
    info!("Getting pending providers");

    Provider::find_pending(&ctx.deps().db_pool).await
}

/// Get all approved providers
pub async fn get_approved_providers(
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Vec<Provider>> {
    info!("Getting approved providers");

    Provider::find_approved(&ctx.deps().db_pool).await
}
