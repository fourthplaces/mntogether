use crate::common::{DomainId, ListingId};
use crate::domains::listings::models::{ListingStatus, Listing};
use crate::domains::organization::utils::{generate_need_content_hash as generate_listing_content_hash, generate_tldr};
use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;

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

/// Synchronize extracted listings with database (with transaction)
///
/// Algorithm:
/// 1. Calculate content hash for each extracted listing
/// 2. Find existing listings from same domain
/// 3. Compare hashes:
///    - Same hash = unchanged (update last_seen_at)
///    - Different hash = changed (create new pending_approval)
///    - Not found = new (create pending_approval)
/// 4. Mark listings not in extracted set as disappeared
///
/// All operations are wrapped in a transaction to ensure atomicity.
/// If any operation fails, all changes are rolled back.
pub async fn sync_listings(
    pool: &PgPool,
    domain_id: DomainId,
    extracted_listings: Vec<ExtractedListingInput>,
) -> Result<SyncResult> {
    // TODO: Re-implement with transaction-scoped Listing model methods
    // The Listing model currently doesn't have transaction-aware methods (_tx suffix)
    // This is a temporary stub to allow compilation

    tracing::warn!(
        domain_id = %domain_id,
        listing_count = extracted_listings.len(),
        "Syncing logic temporarily disabled - transaction-scoped methods not yet implemented"
    );

    // For now, just create new listings without deduplication
    let mut new_listings = Vec::new();

    for listing in extracted_listings {
        let content_hash = generate_listing_content_hash(
            &listing.title,
            &listing.description,
            &listing.organization_name
        );

        let tldr = listing.tldr.or_else(|| Some(generate_tldr(&listing.description, 100)));

        match Listing::create(
            listing.organization_name,
            listing.title,
            listing.description,
            tldr,
            "opportunity".to_string(),
            "general".to_string(),
            Some("accepting".to_string()),
            listing.urgency,
            None, // location
            ListingStatus::PendingApproval.to_string(),
            Some(content_hash),
            "en".to_string(), // source_language
            Some("scraped".to_string()),
            None, // submitted_by_admin_id
            Some(domain_id),
            listing.source_url,
            None, // organization_id
            pool,
        ).await {
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

/// Create a new pending listing (transaction-aware)
/// TODO: Re-implement when transaction-scoped Listing methods are available
#[allow(dead_code)]
async fn create_pending_listing_tx(
    _tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    _domain_id: DomainId,
    _listing: &ExtractedListingInput,
    _content_hash: &str,
) -> Result<ListingId> {
    anyhow::bail!("create_pending_listing_tx not yet implemented with new Listing model")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash_generation() {
        let listing = ExtractedListingInput {
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
            generate_listing_content_hash(&listing.title, &listing.description, &listing.organization_name);

        // Same content should produce same hash
        let hash2 =
            generate_listing_content_hash(&listing.title, &listing.description, &listing.organization_name);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 is 64 hex chars
    }
}
