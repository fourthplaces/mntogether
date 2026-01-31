use crate::common::{ListingId, WebsiteId};
use crate::domains::listings::models::{Listing, ListingStatus};
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

/// Synchronize extracted listings with database
///
/// Current implementation: Creates all extracted listings as new pending_approval listings.
///
/// TODO: Implement full sync logic with deduplication:
/// 1. Calculate content hash for each extracted listing
/// 2. Find existing listings from same website
/// 3. Compare hashes:
///    - Same hash = unchanged (update last_seen_at)
///    - Different hash = changed (create new pending_approval)
///    - Not found = new (create pending_approval)
/// 4. Mark listings not in extracted set as disappeared
///
/// This requires implementing transaction-scoped methods in the Listing model
/// to enable proper atomic updates within a database transaction.
pub async fn sync_listings(
    pool: &PgPool,
    website_id: WebsiteId,
    extracted_listings: Vec<ExtractedListingInput>,
) -> Result<SyncResult> {
    tracing::info!(
        website_id = %website_id,
        listing_count = extracted_listings.len(),
        "Creating listings from extracted data (deduplication not yet implemented)"
    );

    // For now, just create new listings without deduplication
    let mut new_listings = Vec::new();

    for listing in extracted_listings {
        let tldr = listing
            .tldr
            .or_else(|| Some(generate_tldr(&listing.description, 100)));

        // Validate urgency value against database constraint
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
            "en".to_string(), // source_language
            Some("scraped".to_string()),
            None, // submitted_by_admin_id
            Some(website_id),
            listing.source_url,
            None, // organization_id
            pool,
        )
        .await
        {
            Ok(created) => new_listings.push(created.id),
            Err(e) => tracing::error!(error = %e, "Failed to create listing during sync"),
        }
    }

    Ok(SyncResult {
        new_listings,
        unchanged_listings: Vec::new(),
        changed_listings: Vec::new(),
        disappeared_listings: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_urgency_valid_values() {
        assert_eq!(normalize_urgency(Some("low".to_string())), Some("low".to_string()));
        assert_eq!(normalize_urgency(Some("medium".to_string())), Some("medium".to_string()));
        assert_eq!(normalize_urgency(Some("high".to_string())), Some("high".to_string()));
        assert_eq!(normalize_urgency(Some("urgent".to_string())), Some("urgent".to_string()));
    }

    #[test]
    fn test_normalize_urgency_case_insensitive() {
        assert_eq!(normalize_urgency(Some("LOW".to_string())), Some("low".to_string()));
        assert_eq!(normalize_urgency(Some("High".to_string())), Some("high".to_string()));
        assert_eq!(normalize_urgency(Some("URGENT".to_string())), Some("urgent".to_string()));
    }

    #[test]
    fn test_normalize_urgency_invalid_values() {
        assert_eq!(normalize_urgency(Some("critical".to_string())), None);
        assert_eq!(normalize_urgency(Some("asap".to_string())), None);
        assert_eq!(normalize_urgency(Some("normal".to_string())), None);
        assert_eq!(normalize_urgency(None), None);
    }
}
