//! GraphQL query resolvers for resources

use juniper::FieldResult;
use tracing::info;

use crate::common::ResourceId;
use crate::domains::resources::data::{ResourceConnection, ResourceData, ResourceStatusData};
use crate::domains::resources::models::Resource;
use crate::server::graphql::context::GraphQLContext;

/// Get a single resource by ID
pub async fn get_resource(
    ctx: &GraphQLContext,
    id: String,
) -> FieldResult<Option<ResourceData>> {
    info!("get_resource query called: {}", id);

    let resource_id = ResourceId::parse(&id)?;
    let resource = Resource::find_by_id_optional(resource_id, &ctx.db_pool).await?;

    Ok(resource.map(ResourceData::from))
}

/// Get resources with pagination and optional status filter
pub async fn get_resources(
    ctx: &GraphQLContext,
    status: Option<ResourceStatusData>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> FieldResult<ResourceConnection> {
    info!(
        "get_resources query called: status={:?}",
        status
    );

    let limit = limit.unwrap_or(20).min(100) as i64;
    let offset = offset.unwrap_or(0) as i64;

    let status_str = status.map(|s| match s {
        ResourceStatusData::PendingApproval => "pending_approval",
        ResourceStatusData::Active => "active",
        ResourceStatusData::Rejected => "rejected",
        ResourceStatusData::Expired => "expired",
    });

    let resources = if let Some(status) = status_str {
        Resource::find_by_status(status, limit, offset, &ctx.db_pool).await?
    } else {
        // Default to pending + active if no status specified
        Resource::find_by_status("pending_approval", limit, offset, &ctx.db_pool).await?
    };

    let total_count = if let Some(status) = status_str {
        Resource::count_by_status(status, &ctx.db_pool).await? as i32
    } else {
        Resource::count_by_status("pending_approval", &ctx.db_pool).await? as i32
    };

    let has_next_page = (offset + limit) < total_count as i64;

    Ok(ResourceConnection {
        nodes: resources.into_iter().map(ResourceData::from).collect(),
        total_count,
        has_next_page,
    })
}

/// Get pending resources (for admin approval queue)
pub async fn get_pending_resources(ctx: &GraphQLContext) -> FieldResult<Vec<ResourceData>> {
    info!("get_pending_resources query called");

    let resources = Resource::find_by_status("pending_approval", 100, 0, &ctx.db_pool).await?;

    Ok(resources.into_iter().map(ResourceData::from).collect())
}

/// Get active resources
pub async fn get_active_resources(
    ctx: &GraphQLContext,
    limit: Option<i32>,
) -> FieldResult<Vec<ResourceData>> {
    info!("get_active_resources query called");

    let limit = limit.unwrap_or(50).min(100) as i64;
    let resources = Resource::find_by_status("active", limit, 0, &ctx.db_pool).await?;

    Ok(resources.into_iter().map(ResourceData::from).collect())
}
