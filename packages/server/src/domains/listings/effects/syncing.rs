// Domain functions for listing synchronization
//
// These functions contain business logic for syncing extracted listings
// with the database, separated from the thin Effect orchestrator.

use anyhow::{Context, Result};
use sqlx::PgPool;

use super::utils::sync_utils::{sync_listings, ExtractedListingInput};
use crate::common::{DomainId, SourceId};
use crate::domains::listings::events::ExtractedListing;
use crate::domains::organization::models::source::OrganizationSource;

/// Result of syncing listings with the database
pub struct ListingSyncResult {
    pub new_count: usize,
    pub changed_count: usize,
    pub disappeared_count: usize,
}

/// Sync extracted listings with the database for a given source
///
/// This function:
/// 1. Fetches the source to get organization_name
/// 2. Converts extracted listings to sync input format
/// 3. Performs sync operation with database
/// 4. Returns summary of changes
pub async fn sync_extracted_listings(
    source_id: SourceId,
    listings: Vec<ExtractedListing>,
    pool: &PgPool,
) -> Result<ListingSyncResult> {
    // Get source to fetch organization_name
    let source = OrganizationSource::find_by_id(source_id, pool)
        .await
        .context("Failed to find source")?;

    // Convert event listings to sync input
    let sync_input: Vec<ExtractedListingInput> = listings
        .into_iter()
        .map(|listing| ExtractedListingInput {
            organization_name: source.organization_name.clone(),
            title: listing.title,
            description: listing.description,
            description_markdown: None,
            tldr: Some(listing.tldr),
            contact: listing.contact.and_then(|c| {
                serde_json::json!({
                    "email": c.email,
                    "phone": c.phone,
                    "website": c.website
                })
                .as_object()
                .map(|obj| serde_json::Value::Object(obj.clone()))
            }),
            urgency: listing.urgency,
            confidence: listing.confidence,
            source_url: Some(source.source_url.clone()), // Use main source URL for now
        })
        .collect();

    // Sync with database (convert SourceId to DomainId)
    let domain_id = DomainId::from_uuid(source_id.into_uuid());
    let sync_result = sync_listings(pool, domain_id, sync_input)
        .await
        .context("Sync failed")?;

    Ok(ListingSyncResult {
        new_count: sync_result.new_listings.len(),
        changed_count: sync_result.changed_listings.len(),
        disappeared_count: sync_result.disappeared_listings.len(),
    })
}
