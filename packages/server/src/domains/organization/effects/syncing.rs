// Domain functions for need synchronization
//
// These functions contain business logic for syncing extracted needs
// with the database, separated from the thin Effect orchestrator.

use anyhow::{Context, Result};
use sqlx::PgPool;

use super::{sync_needs, ExtractedNeedInput};
use crate::common::SourceId;
use crate::domains::organization::events::ExtractedNeed;
use crate::domains::organization::models::source::OrganizationSource;

/// Result of syncing needs with the database
pub struct NeedSyncResult {
    pub new_count: usize,
    pub changed_count: usize,
    pub disappeared_count: usize,
}

/// Sync extracted needs with the database for a given source
///
/// This function:
/// 1. Fetches the source to get organization_name
/// 2. Converts extracted needs to sync input format
/// 3. Performs sync operation with database
/// 4. Returns summary of changes
pub async fn sync_extracted_needs(
    source_id: SourceId,
    needs: Vec<ExtractedNeed>,
    pool: &PgPool,
) -> Result<NeedSyncResult> {
    // Get source to fetch organization_name
    let source = OrganizationSource::find_by_id(source_id, pool)
        .await
        .context("Failed to find source")?;

    // Convert event needs to sync input
    let sync_input: Vec<ExtractedNeedInput> = needs
        .into_iter()
        .map(|need| ExtractedNeedInput {
            organization_name: source.organization_name.clone(),
            title: need.title,
            description: need.description,
            description_markdown: None,
            tldr: Some(need.tldr),
            contact: need.contact.and_then(|c| {
                serde_json::json!({
                    "email": c.email,
                    "phone": c.phone,
                    "website": c.website
                })
                .as_object()
                .map(|obj| serde_json::Value::Object(obj.clone()))
            }),
            urgency: need.urgency,
            confidence: need.confidence,
        })
        .collect();

    // Sync with database
    let sync_result = sync_needs(pool, source_id, sync_input)
        .await
        .context("Sync failed")?;

    Ok(NeedSyncResult {
        new_count: sync_result.new_needs.len(),
        changed_count: sync_result.changed_needs.len(),
        disappeared_count: sync_result.disappeared_needs.len(),
    })
}
