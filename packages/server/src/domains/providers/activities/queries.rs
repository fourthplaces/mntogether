//! Provider query actions
//!
//! Query actions return data directly and are called without process().
//! Auth checks are done at the API layer.

use anyhow::{Context, Result};
use tracing::info;

use crate::common::{build_page_info, Cursor, ProviderId, ValidatedPaginationArgs};
use crate::domains::providers::data::{ProviderConnection, ProviderData, ProviderEdge};
use crate::domains::providers::models::Provider;
use crate::kernel::ServerDeps;

/// Get a single provider by ID
/// Note: Admin auth is checked at the API layer
pub async fn get_provider(provider_id: String, deps: &ServerDeps) -> Result<Option<Provider>> {
    let id = ProviderId::parse(&provider_id).context("Invalid provider ID")?;

    info!(provider_id = %id, "Getting provider");

    Provider::find_by_id_optional(id, &deps.db_pool).await
}

/// Get all providers with optional filters
/// Note: Admin auth is checked at the API layer
pub async fn get_providers(
    status: Option<String>,
    accepting_clients: Option<bool>,
    limit: Option<i32>,
    offset: Option<i32>,
    deps: &ServerDeps,
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
        &deps.db_pool,
    )
    .await
}

/// Get all pending providers (for admin approval queue)
/// Note: Admin auth is checked at the API layer
pub async fn get_pending_providers(deps: &ServerDeps) -> Result<Vec<Provider>> {
    info!("Getting pending providers");

    Provider::find_pending(&deps.db_pool).await
}

/// Get all approved providers
pub async fn get_approved_providers(deps: &ServerDeps) -> Result<Vec<Provider>> {
    info!("Getting approved providers");

    Provider::find_approved(&deps.db_pool).await
}

/// Get paginated providers with cursor-based pagination (Relay spec)
/// Note: Admin auth is checked at the API layer
pub async fn get_providers_paginated(
    status: Option<&str>,
    args: &ValidatedPaginationArgs,
    deps: &ServerDeps,
) -> Result<ProviderConnection> {
    let pool = &deps.db_pool;

    let (providers, has_more) = Provider::find_paginated(status, args, pool).await?;
    let total_count = Provider::count_with_filters(status, pool).await? as i32;

    let edges: Vec<ProviderEdge> = providers
        .into_iter()
        .map(|provider| {
            let cursor = Cursor::encode_uuid(provider.id.into_uuid());
            ProviderEdge {
                node: ProviderData::from(provider),
                cursor,
            }
        })
        .collect();

    let page_info = build_page_info(
        has_more,
        args,
        edges.first().map(|e| e.cursor.clone()),
        edges.last().map(|e| e.cursor.clone()),
    );

    Ok(ProviderConnection {
        edges,
        page_info,
        total_count,
    })
}
