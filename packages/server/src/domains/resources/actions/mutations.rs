//! Resource mutation actions
//!
//! All resource write operations go through these actions via `engine.activate().process()`.
//! Actions are self-contained: they handle ID parsing and return final models.
//! They emit events for observability and potential future cascades.

use anyhow::{Context, Result};
use seesaw_core::EffectContext;
use tracing::info;

use crate::common::{AppState, ResourceId};
use crate::domains::resources::data::EditResourceInput;
use crate::domains::resources::events::ResourceEvent;
use crate::domains::resources::models::{ChangeReason, Resource, ResourceStatus, ResourceVersion};
use crate::kernel::ServerDeps;

/// Approve a resource (make it active)
/// Returns the updated Resource directly.
pub async fn approve_resource(
    resource_id: String,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Resource> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

    let id = ResourceId::parse(&resource_id).context("Invalid resource ID")?;

    info!(resource_id = %id, "Approving resource");

    Resource::update_status(id, ResourceStatus::Active, &ctx.deps().db_pool).await?;

    // Emit event for observability
    ctx.emit(ResourceEvent::ResourceApproved { resource_id: id });

    Resource::find_by_id(id, &ctx.deps().db_pool).await
}

/// Reject a resource
/// Returns the updated Resource directly.
pub async fn reject_resource(
    resource_id: String,
    reason: String,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Resource> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

    let id = ResourceId::parse(&resource_id).context("Invalid resource ID")?;

    info!(resource_id = %id, reason = %reason, "Rejecting resource");

    Resource::update_status(id, ResourceStatus::Rejected, &ctx.deps().db_pool).await?;

    // Emit event for observability
    ctx.emit(ResourceEvent::ResourceRejected {
        resource_id: id,
        reason,
    });

    Resource::find_by_id(id, &ctx.deps().db_pool).await
}

/// Edit a resource
/// Returns the updated Resource directly.
pub async fn edit_resource(
    resource_id: String,
    input: EditResourceInput,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Resource> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

    let id = ResourceId::parse(&resource_id).context("Invalid resource ID")?;

    info!(resource_id = %id, "Editing resource");

    // Get current state for version history
    let current = Resource::find_by_id(id, &ctx.deps().db_pool).await?;

    // Update the resource
    Resource::update_content(
        id,
        input.title.clone(),
        input.content.clone(),
        input.location.clone(),
        &ctx.deps().db_pool,
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
        &ctx.deps().db_pool,
    )
    .await?;

    // Emit event for observability
    ctx.emit(ResourceEvent::ResourceEdited { resource_id: id });

    Resource::find_by_id(id, &ctx.deps().db_pool).await
}

/// Edit and approve a resource in one operation
/// Returns the updated Resource directly.
pub async fn edit_and_approve_resource(
    resource_id: String,
    input: EditResourceInput,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<Resource> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

    let id = ResourceId::parse(&resource_id).context("Invalid resource ID")?;

    info!(resource_id = %id, "Editing and approving resource");

    // Get current state for version history
    let current = Resource::find_by_id(id, &ctx.deps().db_pool).await?;

    // Update content
    Resource::update_content(
        id,
        input.title.clone(),
        input.content.clone(),
        input.location.clone(),
        &ctx.deps().db_pool,
    )
    .await?;

    // Update status to active
    Resource::update_status(id, ResourceStatus::Active, &ctx.deps().db_pool).await?;

    // Create version record
    ResourceVersion::create(
        id,
        input.title.unwrap_or(current.title),
        input.content.unwrap_or(current.content),
        input.location.or(current.location),
        ChangeReason::ManualEdit,
        None,
        &ctx.deps().db_pool,
    )
    .await?;

    // Emit events for both edit and approve
    ctx.emit(ResourceEvent::ResourceEdited { resource_id: id });
    ctx.emit(ResourceEvent::ResourceApproved { resource_id: id });

    Resource::find_by_id(id, &ctx.deps().db_pool).await
}

/// Delete a resource
///
/// Emits ResourceDeleted event for observability.
/// Note: Related data (tags, sources) are cascade deleted via FK constraints.
pub async fn delete_resource(
    resource_id: String,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<bool> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

    let id = ResourceId::parse(&resource_id).context("Invalid resource ID")?;

    info!(resource_id = %id, "Deleting resource");

    Resource::delete(id, &ctx.deps().db_pool).await?;

    // Emit event for observability
    ctx.emit(ResourceEvent::ResourceDeleted { resource_id: id });

    Ok(true)
}

/// Result of generating missing embeddings
#[derive(Debug, Clone)]
pub struct GenerateEmbeddingsResult {
    pub processed: i32,
    pub failed: i32,
    pub remaining: i32,
}

/// Generate missing embeddings for resources
pub async fn generate_missing_embeddings(
    batch_size: i64,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<GenerateEmbeddingsResult> {
    // Admin authorization check
    ctx.next_state().require_admin()?;

    info!(batch_size = batch_size, "Generating missing embeddings");

    let resources = Resource::find_without_embeddings(batch_size, &ctx.deps().db_pool).await?;

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

        match ctx
            .deps()
            .embedding_service
            .generate(&content_for_embedding)
            .await
        {
            Ok(embedding) => {
                if let Err(e) =
                    Resource::update_embedding(resource.id, &embedding, &ctx.deps().db_pool).await
                {
                    tracing::error!(resource_id = %resource.id.into_uuid(), error = %e, "Failed to save embedding");
                    failed += 1;
                } else {
                    processed += 1;
                }
            }
            Err(e) => {
                tracing::error!(resource_id = %resource.id.into_uuid(), error = %e, "Failed to generate embedding");
                failed += 1;
            }
        }
    }

    let remaining = Resource::count_without_embeddings(&ctx.deps().db_pool).await? as i32;

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
