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

/// Result of generating missing embeddings
#[derive(Debug, Clone, juniper::GraphQLObject)]
pub struct GenerateEmbeddingsResult {
    pub processed: i32,
    pub failed: i32,
    pub remaining: i32,
}

/// Generate missing embeddings for resources (admin only)
/// Processes up to `batch_size` resources at a time
pub async fn generate_missing_embeddings(
    ctx: &GraphQLContext,
    batch_size: Option<i32>,
) -> FieldResult<GenerateEmbeddingsResult> {
    info!("generate_missing_embeddings mutation called");

    // Check admin auth
    let _user = ctx.auth_user.as_ref().ok_or_else(|| {
        juniper::FieldError::new("Authentication required", juniper::Value::null())
    })?;

    let limit = batch_size.unwrap_or(50) as i64;

    // Find resources without embeddings
    let resources = Resource::find_without_embeddings(limit, &ctx.db_pool).await?;

    let mut processed = 0;
    let mut failed = 0;

    for resource in &resources {
        // Generate embedding content from title + content + location
        let content_for_embedding = format!(
            "{}\n\n{}\n\nLocation: {}\nOrganization: {}",
            resource.title,
            resource.content,
            resource.location.as_deref().unwrap_or("Not specified"),
            resource.organization_name.as_deref().unwrap_or("Unknown")
        );

        match ctx.openai_client.create_embedding(&content_for_embedding).await {
            Ok(response) => {
                if let Some(data) = response.data.first() {
                    if let Err(e) = Resource::update_embedding(resource.id, &data.embedding, &ctx.db_pool).await {
                        tracing::error!(resource_id = %resource.id.into_uuid(), error = %e, "Failed to save embedding");
                        failed += 1;
                    } else {
                        processed += 1;
                    }
                } else {
                    tracing::error!(resource_id = %resource.id.into_uuid(), "No embedding data returned");
                    failed += 1;
                }
            }
            Err(e) => {
                tracing::error!(resource_id = %resource.id.into_uuid(), error = %e, "Failed to generate embedding");
                failed += 1;
            }
        }
    }

    // Count remaining
    let remaining = Resource::count_without_embeddings(&ctx.db_pool).await? as i32;

    info!(
        processed = processed,
        failed = failed,
        remaining = remaining,
        "Finished generating embeddings"
    );

    Ok(GenerateEmbeddingsResult {
        processed,
        failed,
        remaining,
    })
}
