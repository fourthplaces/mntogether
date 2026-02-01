//! Resource synchronization with AI semantic deduplication
//!
//! This module handles syncing extracted resources to the database:
//! 1. For each extracted resource, check for semantic duplicates
//! 2. Based on AI decision: create new, update existing, or skip
//! 3. Track source URLs and generate embeddings
//! 4. Create version records for audit trail

use anyhow::Result;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::common::{ResourceId, WebsiteId};
use crate::domains::contacts::Contact;
use crate::domains::resources::models::{
    ChangeReason, DedupDecision, Resource, ResourceSource, ResourceStatus, ResourceVersion,
};
use crate::kernel::{BaseAI, BaseEmbeddingService};

use super::deduplication::{deduplicate_resource, DedupAction, DedupInput};

/// Extracted resource input (from AI extraction)
#[derive(Debug, Clone)]
pub struct ExtractedResourceInput {
    pub title: String,
    pub content: String,
    pub location: Option<String>,
    pub organization_name: Option<String>,
    pub source_url: String,
    /// Contact information extracted by AI
    pub contacts: Vec<ExtractedContact>,
    /// Tag values to apply (kind, value pairs)
    pub tags: Vec<(String, String)>,
}

/// Contact information extracted from content
#[derive(Debug, Clone)]
pub struct ExtractedContact {
    pub contact_type: String, // phone, email, website, address
    pub value: String,
    pub label: Option<String>,
}

/// Sync result showing what changed
#[derive(Debug)]
pub struct SyncResult {
    pub new_resources: Vec<ResourceId>,
    pub updated_resources: Vec<ResourceId>,
    pub skipped_count: usize,
    pub errors: Vec<String>,
}

/// Synchronize extracted resources with database using AI semantic deduplication
///
/// Algorithm:
/// 1. For each extracted resource:
///    - Generate embedding
///    - Find similar existing resources (vector pre-filter)
///    - If similar found, ask AI to decide: NEW, UPDATE, or SKIP
///    - Execute decision
/// 2. Track source URLs
/// 3. Create version records
pub async fn sync_resources(
    pool: &PgPool,
    website_id: WebsiteId,
    extracted_resources: Vec<ExtractedResourceInput>,
    embedding_service: &dyn BaseEmbeddingService,
    ai: &dyn BaseAI,
) -> Result<SyncResult> {
    info!(
        website_id = %website_id,
        count = extracted_resources.len(),
        "Syncing resources with AI deduplication"
    );

    let mut new_resources = Vec::new();
    let mut updated_resources = Vec::new();
    let mut skipped_count = 0;
    let mut errors = Vec::new();

    for input in extracted_resources {
        match sync_single_resource(
            &input,
            website_id,
            embedding_service,
            ai,
            pool,
        )
        .await
        {
            Ok(SyncAction::Created(id)) => {
                info!(resource_id = %id, title = %input.title, "Created new resource");
                new_resources.push(id);
            }
            Ok(SyncAction::Updated(id)) => {
                info!(resource_id = %id, title = %input.title, "Updated existing resource");
                updated_resources.push(id);
            }
            Ok(SyncAction::Skipped) => {
                info!(title = %input.title, "Skipped duplicate resource");
                skipped_count += 1;
            }
            Err(e) => {
                warn!(
                    title = %input.title,
                    error = %e,
                    "Failed to sync resource"
                );
                errors.push(format!("Failed to sync '{}': {}", input.title, e));
            }
        }
    }

    info!(
        new = new_resources.len(),
        updated = updated_resources.len(),
        skipped = skipped_count,
        errors = errors.len(),
        "Resource sync complete"
    );

    Ok(SyncResult {
        new_resources,
        updated_resources,
        skipped_count,
        errors,
    })
}

enum SyncAction {
    Created(ResourceId),
    Updated(ResourceId),
    Skipped,
}

/// Sync a single resource
async fn sync_single_resource(
    input: &ExtractedResourceInput,
    website_id: WebsiteId,
    embedding_service: &dyn BaseEmbeddingService,
    ai: &dyn BaseAI,
    pool: &PgPool,
) -> Result<SyncAction> {
    // Check for duplicates using AI
    let dedup_input = DedupInput {
        title: input.title.clone(),
        content: input.content.clone(),
        location: input.location.clone(),
        organization_name: input.organization_name.clone(),
    };

    let dedup_action = deduplicate_resource(
        &dedup_input,
        website_id,
        embedding_service,
        ai,
        pool,
    )
    .await?;

    match dedup_action {
        DedupAction::New { reasoning } => {
            // Create new resource
            let resource = Resource::create(
                website_id,
                input.title.clone(),
                input.content.clone(),
                input.location.clone(),
                input.organization_name.clone(),
                ResourceStatus::PendingApproval,
                pool,
            )
            .await?;

            // Generate and save embedding
            let content_for_embedding = format!(
                "{}\n\n{}\n\nLocation: {}",
                input.title,
                input.content,
                input.location.as_deref().unwrap_or("")
            );
            let embedding = embedding_service.generate(&content_for_embedding).await?;
            Resource::update_embedding(resource.id, &embedding, pool).await?;

            // Add source URL
            ResourceSource::add(resource.id, input.source_url.clone(), None, pool).await?;

            // Add contacts
            for (i, contact) in input.contacts.iter().enumerate() {
                Contact::create_for_resource(
                    resource.id,
                    &contact.contact_type,
                    &contact.value,
                    contact.label.clone(),
                    true, // is_public
                    i as i32,
                    pool,
                )
                .await?;
            }

            // Add tags
            for (kind, value) in &input.tags {
                use crate::domains::resources::models::ResourceTag;
                if let Err(e) = ResourceTag::add_tag_by_value(resource.id, kind, value, None, pool).await {
                    warn!(
                        resource_id = %resource.id,
                        kind = %kind,
                        value = %value,
                        error = %e,
                        "Failed to add tag to resource"
                    );
                }
            }

            // Create initial version record
            ResourceVersion::create(
                resource.id,
                input.title.clone(),
                input.content.clone(),
                input.location.clone(),
                ChangeReason::Created,
                None,
                pool,
            )
            .await?;

            info!(
                resource_id = %resource.id,
                reasoning = %reasoning,
                "Created new resource"
            );

            Ok(SyncAction::Created(resource.id))
        }

        DedupAction::Update {
            existing_id,
            similarity_score,
            reasoning,
        } => {
            let resource_id = ResourceId::from_uuid(existing_id);

            // Update the existing resource
            Resource::update_content(
                resource_id,
                Some(input.title.clone()),
                Some(input.content.clone()),
                input.location.clone(),
                pool,
            )
            .await?;

            // Regenerate embedding with new content
            let content_for_embedding = format!(
                "{}\n\n{}\n\nLocation: {}",
                input.title,
                input.content,
                input.location.as_deref().unwrap_or("")
            );
            let embedding = embedding_service.generate(&content_for_embedding).await?;
            Resource::update_embedding(resource_id, &embedding, pool).await?;

            // Add this source URL (might be new source for same resource)
            ResourceSource::add(resource_id, input.source_url.clone(), None, pool).await?;

            // Create version record for the update
            ResourceVersion::create(
                resource_id,
                input.title.clone(),
                input.content.clone(),
                input.location.clone(),
                ChangeReason::AiUpdate,
                Some(DedupDecision {
                    matched_resource_id: Some(existing_id),
                    similarity_score: Some(similarity_score),
                    ai_reasoning: Some(reasoning.clone()),
                }),
                pool,
            )
            .await?;

            info!(
                resource_id = %resource_id,
                similarity = %similarity_score,
                reasoning = %reasoning,
                "Updated existing resource"
            );

            Ok(SyncAction::Updated(resource_id))
        }

        DedupAction::Skip {
            existing_id,
            similarity_score,
            reasoning,
        } => {
            // Just add this URL as an additional source (the content was found elsewhere)
            let resource_id = ResourceId::from_uuid(existing_id);
            ResourceSource::add(resource_id, input.source_url.clone(), None, pool).await?;

            info!(
                resource_id = %resource_id,
                similarity = %similarity_score,
                reasoning = %reasoning,
                "Skipped duplicate, added source URL"
            );

            Ok(SyncAction::Skipped)
        }
    }
}
