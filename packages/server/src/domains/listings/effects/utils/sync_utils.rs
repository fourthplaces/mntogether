use crate::common::{ListingId, WebsiteId};
use crate::domains::listings::models::{Listing, ListingContact, ListingStatus, ListingWebsiteSync};
use crate::domains::organization::utils::generate_tldr;
use anyhow::Result;
use sqlx::PgPool;

/// Valid urgency values per database constraint
const VALID_URGENCY_VALUES: &[&str] = &["low", "medium", "high", "urgent"];

/// Normalize urgency value to a valid database value
/// Returns None if the input is invalid or None
fn normalize_urgency(urgency: Option<String>) -> Option<String> {
    urgency.and_then(|u| {
        let normalized = u.to_lowercase();
        if VALID_URGENCY_VALUES.contains(&normalized.as_str()) {
            Some(normalized)
        } else {
            tracing::warn!(
                urgency = %u,
                "Invalid urgency value from AI, ignoring"
            );
            None
        }
    })
}

/// Generate a content hash for deduplication
/// Hash is based on normalized title + description
fn generate_content_hash(title: &str, description: &str) -> String {
    let normalized_content = format!(
        "{}|{}",
        title.trim().to_lowercase(),
        description.trim().to_lowercase()
    );
    format!("{:x}", md5::compute(normalized_content.as_bytes()))
}

/// Sync result showing what changed
#[derive(Debug)]
pub struct SyncResult {
    pub new_listings: Vec<ListingId>,
    pub unchanged_listings: Vec<ListingId>,
    pub changed_listings: Vec<ListingId>,
    pub disappeared_listings: Vec<ListingId>,
}

/// Extracted listing input (from AI)
#[derive(Debug, Clone)]
pub struct ExtractedListingInput {
    pub organization_name: String,
    pub title: String,
    pub description: String,
    pub description_markdown: Option<String>,
    pub tldr: Option<String>,
    pub contact: Option<serde_json::Value>,
    pub urgency: Option<String>,
    pub confidence: Option<String>,
    pub source_url: Option<String>, // Page URL where listing was found
}

/// Synchronize extracted listings with database using content-hash deduplication
///
/// Algorithm:
/// 1. Calculate content hash for each extracted listing (title + description)
/// 2. For each extracted listing:
///    - Check if a sync record with this hash exists for this website
///    - If exists: update last_seen_at (listing already exists, no duplicate)
///    - If not exists: create new listing and sync record
/// 3. Mark listings not in extracted set as disappeared
pub async fn sync_listings(
    pool: &PgPool,
    website_id: WebsiteId,
    extracted_listings: Vec<ExtractedListingInput>,
) -> Result<SyncResult> {
    tracing::info!(
        website_id = %website_id,
        listing_count = extracted_listings.len(),
        "Syncing listings with deduplication"
    );

    let mut new_listings = Vec::new();
    let mut unchanged_listings = Vec::new();
    let mut seen_hashes = Vec::new();

    for listing in extracted_listings {
        // Generate content hash for deduplication
        let content_hash = generate_content_hash(&listing.title, &listing.description);
        seen_hashes.push(content_hash.clone());

        // Check if we've already seen this listing on this website
        let existing_sync =
            ListingWebsiteSync::find_by_content_hash(website_id, &content_hash, pool).await?;

        if let Some(sync_record) = existing_sync {
            // Listing already exists - just update last_seen_at
            tracing::debug!(
                content_hash = %content_hash,
                listing_id = %sync_record.listing_id,
                "Listing already exists, updating last_seen_at"
            );

            // Upsert updates last_seen_at and clears disappeared_at
            ListingWebsiteSync::upsert(
                sync_record.get_listing_id(),
                website_id,
                content_hash,
                listing.source_url.clone().unwrap_or_default(),
                pool,
            )
            .await?;

            unchanged_listings.push(sync_record.get_listing_id());
        } else {
            // New listing - create it
            let tldr = listing
                .tldr
                .or_else(|| Some(generate_tldr(&listing.description, 100)));

            let urgency = normalize_urgency(listing.urgency);

            match Listing::create(
                listing.organization_name,
                listing.title,
                listing.description,
                tldr,
                "opportunity".to_string(),
                "general".to_string(),
                Some("accepting".to_string()),
                urgency,
                None, // location
                ListingStatus::PendingApproval.to_string(),
                "en".to_string(),
                Some("scraped".to_string()),
                None, // submitted_by_admin_id
                Some(website_id),
                listing.source_url.clone(),
                None, // organization_id
                pool,
            )
            .await
            {
                Ok(created) => {
                    tracing::info!(
                        listing_id = %created.id,
                        content_hash = %content_hash,
                        "Created new listing"
                    );

                    // Save contact info if present
                    if let Some(ref contact_info) = listing.contact {
                        if let Err(e) =
                            ListingContact::create_from_json(created.id, contact_info, pool).await
                        {
                            tracing::warn!(
                                listing_id = %created.id,
                                error = %e,
                                "Failed to save contact info"
                            );
                        }
                    }

                    // Create sync record to track this listing
                    ListingWebsiteSync::upsert(
                        created.id,
                        website_id,
                        content_hash,
                        listing.source_url.unwrap_or_default(),
                        pool,
                    )
                    .await?;

                    new_listings.push(created.id);
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to create listing during sync");
                }
            }
        }
    }

    // Mark listings that weren't seen as disappeared
    let disappeared_uuids =
        ListingWebsiteSync::mark_disappeared_except(website_id, seen_hashes, pool).await?;

    let disappeared_listings: Vec<ListingId> = disappeared_uuids
        .into_iter()
        .map(ListingId::from_uuid)
        .collect();

    if !disappeared_listings.is_empty() {
        tracing::info!(
            count = disappeared_listings.len(),
            "Marked listings as disappeared"
        );
    }

    Ok(SyncResult {
        new_listings,
        unchanged_listings,
        changed_listings: Vec::new(), // TODO: Detect content changes (same org/title, different description)
        disappeared_listings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_urgency_valid_values() {
        assert_eq!(
            normalize_urgency(Some("low".to_string())),
            Some("low".to_string())
        );
        assert_eq!(
            normalize_urgency(Some("medium".to_string())),
            Some("medium".to_string())
        );
        assert_eq!(
            normalize_urgency(Some("high".to_string())),
            Some("high".to_string())
        );
        assert_eq!(
            normalize_urgency(Some("urgent".to_string())),
            Some("urgent".to_string())
        );
    }

    #[test]
    fn test_normalize_urgency_case_insensitive() {
        assert_eq!(
            normalize_urgency(Some("LOW".to_string())),
            Some("low".to_string())
        );
        assert_eq!(
            normalize_urgency(Some("High".to_string())),
            Some("high".to_string())
        );
        assert_eq!(
            normalize_urgency(Some("URGENT".to_string())),
            Some("urgent".to_string())
        );
    }

    #[test]
    fn test_normalize_urgency_invalid_values() {
        assert_eq!(normalize_urgency(Some("critical".to_string())), None);
        assert_eq!(normalize_urgency(Some("asap".to_string())), None);
        assert_eq!(normalize_urgency(Some("normal".to_string())), None);
        assert_eq!(normalize_urgency(None), None);
    }

    #[test]
    fn test_generate_content_hash_consistency() {
        let hash1 = generate_content_hash("Test Title", "Test Description");
        let hash2 = generate_content_hash("Test Title", "Test Description");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_generate_content_hash_case_insensitive() {
        let hash1 = generate_content_hash("Test Title", "Test Description");
        let hash2 = generate_content_hash("TEST TITLE", "TEST DESCRIPTION");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_generate_content_hash_whitespace_normalization() {
        let hash1 = generate_content_hash("Test Title", "Test Description");
        let hash2 = generate_content_hash("  Test Title  ", "  Test Description  ");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_generate_content_hash_different_content() {
        let hash1 = generate_content_hash("Title A", "Description A");
        let hash2 = generate_content_hash("Title B", "Description B");
        assert_ne!(hash1, hash2);
    }
}
