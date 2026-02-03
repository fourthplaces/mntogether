//! Resource query actions
//!
//! All resource read operations go through these actions via `engine.activate().process()`.
//! Actions are self-contained: they handle ID parsing and return final models.

use anyhow::{Context, Result};
use seesaw_core::EffectContext;
use tracing::info;

use crate::common::{build_page_info, AppState, Cursor, ResourceId, ValidatedPaginationArgs};
use crate::domains::resources::data::{
    ResourceConnection, ResourceData, ResourceEdge, ResourceStatusData,
};
use crate::domains::resources::models::{Resource, ResourceStatus};
use crate::kernel::ServerDeps;

/// Get a single resource by ID (admin only)
pub async fn get_resource(
    resource_id: String,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Option<Resource>> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

    let id = ResourceId::parse(&resource_id).context("Invalid resource ID")?;

    info!(resource_id = %id, "Getting resource");

    Resource::find_by_id_optional(id, &ctx.deps().db_pool).await
}

/// Get resources with pagination and optional status filter (admin only)
pub async fn get_resources(
    status: Option<ResourceStatusData>,
    limit: i64,
    offset: i64,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Vec<Resource>> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

    info!(status = ?status, "Getting resources with filters");

    let status_filter = status.map(|s| match s {
        ResourceStatusData::PendingApproval => ResourceStatus::PendingApproval,
        ResourceStatusData::Active => ResourceStatus::Active,
        ResourceStatusData::Rejected => ResourceStatus::Rejected,
        ResourceStatusData::Expired => ResourceStatus::Expired,
    });

    Resource::find_with_filters(status_filter, limit, offset, &ctx.deps().db_pool).await
}

/// Count resources with optional status filter (admin only)
pub async fn count_resources(
    status: Option<ResourceStatusData>,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<i64> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

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
    // Admin authorization check
    ctx.next_state().require_admin()?;

    info!("Getting pending resources");

    Resource::find_pending(&ctx.deps().db_pool).await
}

/// Get active resources (admin only)
pub async fn get_active_resources(
    limit: i64,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Vec<Resource>> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

    info!("Getting active resources");

    Resource::find_active(limit, &ctx.deps().db_pool).await
}

/// Get paginated resources with cursor-based pagination (Relay spec)
///
/// Admin only. Returns a ResourceConnection with edges, pageInfo, and totalCount.
pub async fn get_resources_paginated(
    status: Option<ResourceStatusData>,
    args: &ValidatedPaginationArgs,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<ResourceConnection> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

    let pool = &ctx.deps().db_pool;

    // Convert status for model query
    let status_str = status.map(|s| match s {
        ResourceStatusData::PendingApproval => "pending_approval",
        ResourceStatusData::Active => "active",
        ResourceStatusData::Rejected => "rejected",
        ResourceStatusData::Expired => "expired",
    });

    // Fetch resources with cursor pagination
    let (resources, has_more) = Resource::find_paginated(status_str, args, pool).await?;

    // Get total count for the filter
    let status_filter = status.map(|s| match s {
        ResourceStatusData::PendingApproval => ResourceStatus::PendingApproval,
        ResourceStatusData::Active => ResourceStatus::Active,
        ResourceStatusData::Rejected => ResourceStatus::Rejected,
        ResourceStatusData::Expired => ResourceStatus::Expired,
    });
    let total_count = Resource::count_with_filters(status_filter, pool).await? as i32;

    // Build edges with cursors
    let edges: Vec<ResourceEdge> = resources
        .into_iter()
        .map(|resource| {
            let cursor = Cursor::encode_uuid(resource.id.into_uuid());
            ResourceEdge {
                node: ResourceData::from(resource),
                cursor,
            }
        })
        .collect();

    // Build page info
    let page_info = build_page_info(
        has_more,
        args,
        edges.first().map(|e| e.cursor.clone()),
        edges.last().map(|e| e.cursor.clone()),
    );

    Ok(ResourceConnection {
        edges,
        page_info,
        total_count,
    })
}
