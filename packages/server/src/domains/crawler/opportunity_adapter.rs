use anyhow::{Context, Result};
use chrono::Utc;
use intelligent_crawler::RawExtraction;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tracing::{debug, info};

use crate::common::Id;

use crate::common::{DomainId, ListingId};
use crate::domains::listings::models::listing::Listing;

/// Extracted opportunity data structure (what the AI extracts)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedOpportunity {
    pub organization_name: String,
    pub title: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tldr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_info: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urgency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

/// Opportunity adapter - converts RawExtraction to Listing
pub struct OpportunityAdapter {
    pool: PgPool,
}

impl OpportunityAdapter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Convert RawExtraction to Listing and save to database
    pub async fn process_extraction(&self, extraction: &RawExtraction) -> Result<ListingId> {
        // Parse the opaque JSON into our expected structure
        let opportunity: ExtractedOpportunity = serde_json::from_value(extraction.data.clone())
            .context("Failed to parse extraction data as ExtractedOpportunity")?;

        debug!(
            page_id = %extraction.page_id,
            title = %opportunity.title,
            "Processing extracted opportunity"
        );

        // Calculate fingerprint for deduplication
        let fingerprint = extraction
            .fingerprint_hint
            .clone()
            .unwrap_or_else(|| self.calculate_fingerprint(&opportunity));

        // Check if this opportunity already exists (by content_hash as fingerprint)
        if let Some(existing_id) = self.find_by_fingerprint(&fingerprint).await? {
            info!(
                listing_id = %existing_id,
                fingerprint = %fingerprint,
                "Opportunity already exists, skipping"
            );
            return Ok(existing_id);
        }

        // Create new listing
        let listing_id = ListingId::new();
        let now = Utc::now();

        // TODO: Get or create domain_id from the page_url domain
        let domain_id: Option<DomainId> = None;

        sqlx::query(
            r#"
            INSERT INTO listings (
                id, organization_name, title, description, description_markdown,
                tldr, urgency, status, content_hash,
                location, submission_type, domain_id, source_url,
                last_seen_at, created_at, updated_at,
                listing_type, category, capacity_status, source_language
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17, $18, $19, $20
            )
            "#,
        )
        .bind(listing_id)
        .bind(&opportunity.organization_name)
        .bind(&opportunity.title)
        .bind(&opportunity.description)
        .bind(None::<String>) // description_markdown - could be generated later
        .bind(opportunity.tldr.as_ref())
        .bind(opportunity.urgency.as_ref())
        .bind("pending_approval") // Status - all crawler-extracted listings start pending
        .bind(Some(&fingerprint))
        .bind(opportunity.location.as_ref())
        .bind("scraped") // submission_type
        .bind(domain_id)
        .bind(extraction.page_url.as_str()) // source_url
        .bind(now) // last_seen_at
        .bind(now) // created_at
        .bind(now) // updated_at
        .bind("opportunity") // listing_type
        .bind(opportunity.category.as_deref().unwrap_or("other")) // category
        .bind("accepting") // capacity_status
        .bind("en") // source_language - TODO: detect from content
        .execute(&self.pool)
        .await
        .context("Failed to insert listing")?;

        info!(
            listing_id = %listing_id,
            title = %opportunity.title,
            "Created new opportunity from extraction"
        );

        Ok(listing_id)
    }

    /// Find existing opportunity by fingerprint (stored in content_hash)
    async fn find_by_fingerprint(&self, fingerprint: &str) -> Result<Option<ListingId>> {
        let row: Option<(uuid::Uuid,)> = sqlx::query_as(
            "SELECT id FROM listings WHERE content_hash = $1 AND listing_type = 'opportunity' LIMIT 1"
        )
        .bind(fingerprint)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(id,)| Id::from_uuid(id)))
    }

    /// Calculate fingerprint for deduplication
    /// Uses organization name + title as the fingerprint basis
    fn calculate_fingerprint(&self, opportunity: &ExtractedOpportunity) -> String {
        let mut hasher = Sha256::new();
        hasher.update(opportunity.organization_name.trim().to_lowercase().as_bytes());
        hasher.update(b"|");
        hasher.update(opportunity.title.trim().to_lowercase().as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Update last_seen_at for existing opportunity
    pub async fn mark_as_seen(&self, listing_id: ListingId) -> Result<()> {
        sqlx::query("UPDATE listings SET last_seen_at = NOW() WHERE id = $1")
            .bind(listing_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Mark opportunities as disappeared if not seen recently
    pub async fn mark_disappeared(&self, days_threshold: i64) -> Result<usize> {
        let result = sqlx::query(
            r#"
            UPDATE listings
            SET disappeared_at = NOW()
            WHERE last_seen_at < NOW() - INTERVAL '1 day' * $1
              AND disappeared_at IS NULL
              AND submission_type = 'scraped'
              AND listing_type = 'opportunity'
            "#,
        )
        .bind(days_threshold)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_calculation() {
        let adapter = OpportunityAdapter {
            pool: PgPool::connect_lazy("postgres://localhost/test").unwrap(),
        };

        let opp1 = ExtractedOpportunity {
            organization_name: "Food Bank".to_string(),
            title: "Volunteers Needed".to_string(),
            description: "Help us sort food".to_string(),
            tldr: None,
            contact_info: None,
            urgency: None,
            location: None,
            category: None,
        };

        let opp2 = ExtractedOpportunity {
            organization_name: "  Food Bank  ".to_string(), // Extra whitespace
            title: "  Volunteers Needed  ".to_string(),
            description: "Different description".to_string(),
            tldr: None,
            contact_info: None,
            urgency: None,
            location: None,
            category: None,
        };

        // Same fingerprint despite whitespace and different description
        assert_eq!(
            adapter.calculate_fingerprint(&opp1),
            adapter.calculate_fingerprint(&opp2)
        );
    }
}
