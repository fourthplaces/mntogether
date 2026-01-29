use crate::common::{NeedId, SourceId};
use crate::domains::organization::models::{NeedStatus, OrganizationNeed};
use crate::domains::organization::utils::{generate_need_content_hash, generate_tldr};
use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;

/// Sync result showing what changed
#[derive(Debug)]
pub struct SyncResult {
    pub new_needs: Vec<NeedId>,
    pub unchanged_needs: Vec<NeedId>,
    pub changed_needs: Vec<NeedId>,
    pub disappeared_needs: Vec<NeedId>,
}

/// Extracted need input (from AI)
#[derive(Debug, Clone)]
pub struct ExtractedNeedInput {
    pub organization_name: String,
    pub title: String,
    pub description: String,
    pub description_markdown: Option<String>,
    pub tldr: Option<String>,
    pub contact: Option<serde_json::Value>,
    pub urgency: Option<String>,
    pub confidence: Option<String>,
    pub source_url: Option<String>, // Page URL where need was found
}

/// Synchronize extracted needs with database
///
/// Algorithm:
/// 1. Calculate content hash for each extracted need
/// 2. Find existing needs from same source
/// 3. Compare hashes:
///    - Same hash = unchanged (update last_seen_at)
///    - Different hash = changed (create new pending_approval)
///    - Not found = new (create pending_approval)
/// 4. Mark needs not in extracted set as disappeared
pub async fn sync_needs(
    pool: &PgPool,
    source_id: SourceId,
    extracted_needs: Vec<ExtractedNeedInput>,
) -> Result<SyncResult> {
    // Calculate content hashes for extracted needs
    let extracted_with_hashes: Vec<_> = extracted_needs
        .into_iter()
        .map(|need| {
            let content_hash =
                generate_need_content_hash(&need.title, &need.description, &need.organization_name);
            (need, content_hash)
        })
        .collect();

    // Fetch existing active needs from this source
    let existing_needs = OrganizationNeed::find_active_by_source(source_id, pool).await?;

    let mut new_needs = Vec::new();
    let mut unchanged_needs = Vec::new();
    let mut changed_needs = Vec::new();

    // Process each extracted need
    for (need, content_hash) in &extracted_with_hashes {
        // Check if this content hash exists in database
        if let Some(existing) = existing_needs
            .iter()
            .find(|n| n.content_hash.as_ref() == Some(content_hash))
        {
            // Unchanged - just update last_seen_at
            OrganizationNeed::touch_last_seen(existing.id, pool).await?;
            unchanged_needs.push(existing.id);
        } else {
            // Check if there's an existing need with same title (might be changed)
            let maybe_changed =
                OrganizationNeed::find_by_source_and_title(source_id, &need.title, pool).await?;

            if maybe_changed.is_some() {
                // Content changed - create new pending_approval need
                let new_id = create_pending_need(pool, source_id, need, content_hash).await?;
                changed_needs.push(new_id);
            } else {
                // New need - create pending_approval
                let new_id = create_pending_need(pool, source_id, need, content_hash).await?;
                new_needs.push(new_id);
            }
        }
    }

    // Find needs that disappeared (weren't in extracted set)
    let extracted_hashes: Vec<String> = extracted_with_hashes
        .iter()
        .map(|(_, hash)| hash.clone())
        .collect();

    let disappeared_needs =
        OrganizationNeed::mark_disappeared_except(source_id, &extracted_hashes, pool).await?;

    Ok(SyncResult {
        new_needs,
        unchanged_needs,
        changed_needs,
        disappeared_needs,
    })
}

/// Create a new pending need
async fn create_pending_need(
    pool: &PgPool,
    source_id: SourceId,
    need: &ExtractedNeedInput,
    content_hash: &str,
) -> Result<NeedId> {
    let created = OrganizationNeed::create(
        need.organization_name.clone(),
        need.title.clone(),
        need.description.clone(),
        need.tldr.clone().unwrap_or_else(|| {
            // Generate TLDR if not provided
            generate_tldr(&need.description, 100)
        }),
        need.contact.clone(),
        need.urgency.clone(),
        None, // location
        NeedStatus::PendingApproval.to_string(),
        content_hash.to_string(),
        Some("scraped".to_string()),
        None, // submitted_by_volunteer_id
        None, // submitted_from_ip
        Some(source_id),
        need.source_url.clone(),
        pool,
    )
    .await?;

    Ok(created.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash_generation() {
        let need = ExtractedNeedInput {
            organization_name: "Test Org".to_string(),
            title: "Help Needed".to_string(),
            description: "We need volunteers".to_string(),
            description_markdown: None,
            tldr: None,
            contact: None,
            urgency: None,
            confidence: None,
            source_url: None,
        };

        let hash1 =
            generate_need_content_hash(&need.title, &need.description, &need.organization_name);

        // Same content should produce same hash
        let hash2 =
            generate_need_content_hash(&need.title, &need.description, &need.organization_name);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 is 64 hex chars
    }
}
