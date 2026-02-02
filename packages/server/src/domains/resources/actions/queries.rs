//! Resource query actions
//!
//! All resource read operations go through these actions via `engine.activate().process()`.
//! Actions are self-contained: they handle ID parsing and return final models.

use anyhow::{Context, Result};
use seesaw_core::EffectContext;
use tracing::info;

use crate::common::{AppState, ResourceId};
use crate::domains::resources::data::ResourceStatusData;
use crate::domains::resources::models::{Resource, ResourceStatus};
use crate::kernel::ServerDeps;

/// Get a single resource by ID
pub async fn get_resource(
    resource_id: String,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Option<Resource>> {
    let id = ResourceId::parse(&resource_id).context("Invalid resource ID")?;

    info!(resource_id = %id, "Getting resource");

    Resource::find_by_id_optional(id, &ctx.deps().db_pool).await
}

/// Get resources with pagination and optional status filter
pub async fn get_resources(
    status: Option<ResourceStatusData>,
    limit: i64,
    offset: i64,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Vec<Resource>> {
    info!(status = ?status, "Getting resources with filters");

    let status_filter = status.map(|s| match s {
        ResourceStatusData::PendingApproval => ResourceStatus::PendingApproval,
        ResourceStatusData::Active => ResourceStatus::Active,
        ResourceStatusData::Rejected => ResourceStatus::Rejected,
        ResourceStatusData::Expired => ResourceStatus::Expired,
    });

    Resource::find_with_filters(status_filter, limit, offset, &ctx.deps().db_pool).await
}

/// Count resources with optional status filter
pub async fn count_resources(
    status: Option<ResourceStatusData>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<i64> {
    let status_filter = status.map(|s| match s {
        ResourceStatusData::PendingApproval => ResourceStatus::PendingApproval,
        ResourceStatusData::Active => ResourceStatus::Active,
        ResourceStatusData::Rejected => ResourceStatus::Rejected,
        ResourceStatusData::Expired => ResourceStatus::Expired,
    });

    Resource::count_with_filters(status_filter, &ctx.deps().db_pool).await
}

/// Get pending resources (for admin approval queue)
pub async fn get_pending_resources(
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Vec<Resource>> {
    info!("Getting pending resources");

    Resource::find_pending(&ctx.deps().db_pool).await
}

/// Get active resources
pub async fn get_active_resources(
    limit: i64,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Vec<Resource>> {
    info!("Getting active resources");

    Resource::find_active(limit, &ctx.deps().db_pool).await
}
