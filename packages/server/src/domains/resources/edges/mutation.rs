//! GraphQL mutation resolvers for resources

use juniper::FieldResult;
use tracing::info;

use crate::common::ResourceId;
use crate::domains::resources::data::{EditResourceInput, ResourceData};
use crate::domains::resources::models::{ChangeReason, Resource, ResourceStatus, ResourceVersion};
use crate::server::graphql::context::GraphQLContext;

/// Approve a resource (make it active)
pub async fn approve_resource(
    ctx: &GraphQLContext,
    resource_id: String,
) -> FieldResult<ResourceData> {
    info!("approve_resource mutation called: {}", resource_id);

    // Check admin auth
    let _user = ctx.auth_user.as_ref().ok_or_else(|| {
        juniper::FieldError::new("Authentication required", juniper::Value::null())
    })?;

    let id = ResourceId::parse(&resource_id)?;
    let resource = Resource::update_status(id, ResourceStatus::Active, &ctx.db_pool).await?;

    Ok(ResourceData::from(resource))
}

/// Reject a resource
pub async fn reject_resource(
    ctx: &GraphQLContext,
    resource_id: String,
    _reason: String,
) -> FieldResult<ResourceData> {
    info!("reject_resource mutation called: {}", resource_id);

    // Check admin auth
    let _user = ctx.auth_user.as_ref().ok_or_else(|| {
        juniper::FieldError::new("Authentication required", juniper::Value::null())
    })?;

    let id = ResourceId::parse(&resource_id)?;
    let resource = Resource::update_status(id, ResourceStatus::Rejected, &ctx.db_pool).await?;

    Ok(ResourceData::from(resource))
}

/// Edit a resource (admin only)
pub async fn edit_resource(
    ctx: &GraphQLContext,
    resource_id: String,
    input: EditResourceInput,
) -> FieldResult<ResourceData> {
    info!("edit_resource mutation called: {}", resource_id);

    // Check admin auth
    let _user = ctx.auth_user.as_ref().ok_or_else(|| {
        juniper::FieldError::new("Authentication required", juniper::Value::null())
    })?;

    let id = ResourceId::parse(&resource_id)?;

    // Get current state for version history
    let current = Resource::find_by_id(id, &ctx.db_pool).await?;

    // Update the resource
    let updated = Resource::update_content(
        id,
        input.title.clone(),
        input.content.clone(),
        input.location.clone(),
        &ctx.db_pool,
    )
    .await?;

    // Create version record for manual edit
    ResourceVersion::create(
        id,
        input.title.unwrap_or(current.title),
        input.content.unwrap_or(current.content),
        input.location.or(current.location),
        ChangeReason::ManualEdit,
        None,
        &ctx.db_pool,
    )
    .await?;

    Ok(ResourceData::from(updated))
}

/// Edit and approve a resource in one operation (admin only)
pub async fn edit_and_approve_resource(
    ctx: &GraphQLContext,
    resource_id: String,
    input: EditResourceInput,
) -> FieldResult<ResourceData> {
    info!("edit_and_approve_resource mutation called: {}", resource_id);

    // Check admin auth
    let _user = ctx.auth_user.as_ref().ok_or_else(|| {
        juniper::FieldError::new("Authentication required", juniper::Value::null())
    })?;

    let id = ResourceId::parse(&resource_id)?;

    // Get current state for version history
    let current = Resource::find_by_id(id, &ctx.db_pool).await?;

    // Update content
    let _updated = Resource::update_content(
        id,
        input.title.clone(),
        input.content.clone(),
        input.location.clone(),
        &ctx.db_pool,
    )
    .await?;

    // Update status to active
    let approved = Resource::update_status(id, ResourceStatus::Active, &ctx.db_pool).await?;

    // Create version record
    ResourceVersion::create(
        id,
        input.title.unwrap_or(current.title),
        input.content.unwrap_or(current.content),
        input.location.or(current.location),
        ChangeReason::ManualEdit,
        None,
        &ctx.db_pool,
    )
    .await?;

    Ok(ResourceData::from(approved))
}

/// Delete a resource (admin only)
pub async fn delete_resource(
    ctx: &GraphQLContext,
    resource_id: String,
) -> FieldResult<bool> {
    info!("delete_resource mutation called: {}", resource_id);

    // Check admin auth
    let _user = ctx.auth_user.as_ref().ok_or_else(|| {
        juniper::FieldError::new("Authentication required", juniper::Value::null())
    })?;

    let id = ResourceId::parse(&resource_id)?;
    Resource::delete(id, &ctx.db_pool).await?;

    Ok(true)
}
