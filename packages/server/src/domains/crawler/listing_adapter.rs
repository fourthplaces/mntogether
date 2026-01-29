// Unified listing adapter - converts RawExtraction to database listings
//
// Handles Service, Opportunity, and Business listing types.

use anyhow::{Context, Result};
use chrono::Utc;
use intelligent_crawler::RawExtraction;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tracing::{debug, info, warn};

use crate::common::{DomainId, Id, ListingId};

use super::extraction_schemas::{
    ExtractedBusiness, ExtractedListingEnvelope, ExtractedOpportunity, ExtractedService,
};

/// Unified listing adapter - converts RawExtraction to database listings
pub struct ListingAdapter {
    pool: PgPool,
}

impl ListingAdapter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Process a RawExtraction and route to appropriate type handler
    pub async fn process_extraction(&self, extraction: &RawExtraction) -> Result<ListingId> {
        // Parse the opaque JSON into our type-discriminated enum
        let envelope: ExtractedListingEnvelope = serde_json::from_value(extraction.data.clone())
            .context("Failed to parse extraction data as ExtractedListingEnvelope")?;

        debug!(
            page_id = %extraction.page_id,
            listing_type = envelope.listing_type(),
            title = envelope.core().title,
            "Processing extracted listing"
        );

        // Calculate fingerprint for deduplication
        let core = envelope.core();
        let fingerprint = extraction
            .fingerprint_hint
            .clone()
            .unwrap_or_else(|| self.calculate_fingerprint(&core.organization_name, &core.title));

        // Check if this listing already exists (by content_hash as fingerprint)
        if let Some(existing_id) = self
            .find_by_fingerprint(&fingerprint, envelope.listing_type())
            .await?
        {
            info!(
                listing_id = %existing_id,
                fingerprint = %fingerprint,
                listing_type = envelope.listing_type(),
                "Listing already exists, updating last_seen_at"
            );

            self.mark_as_seen(existing_id).await?;
            return Ok(existing_id);
        }

        // Route to type-specific handler
        match envelope {
            ExtractedListingEnvelope::Service(service) => {
                self.process_service_extraction(extraction, service, &fingerprint)
                    .await
            }
            ExtractedListingEnvelope::Opportunity(opportunity) => {
                self.process_opportunity_extraction(extraction, opportunity, &fingerprint)
                    .await
            }
            ExtractedListingEnvelope::Business(business) => {
                self.process_business_extraction(extraction, business, &fingerprint)
                    .await
            }
        }
    }

    /// Process service listing extraction
    async fn process_service_extraction(
        &self,
        extraction: &RawExtraction,
        service: ExtractedService,
        fingerprint: &str,
    ) -> Result<ListingId> {
        let listing_id = ListingId::new();
        let now = Utc::now();

        // TODO: Get or create domain_id from the page_url domain
        let domain_id: Option<DomainId> = None;

        // TODO: Get or create organization_id from organization_name
        let organization_id: Option<uuid::Uuid> = None;

        // Insert core listing
        sqlx::query(
            r#"
            INSERT INTO listings (
                id, organization_id, organization_name, title, description, description_markdown,
                tldr, urgency, status, content_hash,
                location, submission_type, domain_id, source_url,
                last_seen_at, created_at, updated_at,
                listing_type, category, capacity_status, source_language
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21
            )
            "#,
        )
        .bind(listing_id)
        .bind(organization_id)
        .bind(&service.core.organization_name)
        .bind(&service.core.title)
        .bind(&service.core.description)
        .bind(None::<String>) // description_markdown - could be generated later
        .bind(service.core.tldr.as_ref())
        .bind(service.core.urgency.as_ref())
        .bind("pending_approval") // Status - all crawler-extracted listings start pending
        .bind(Some(fingerprint))
        .bind(service.core.location.as_ref())
        .bind("scraped") // submission_type
        .bind(domain_id)
        .bind(extraction.page_url.as_str()) // source_url
        .bind(now) // last_seen_at
        .bind(now) // created_at
        .bind(now) // updated_at
        .bind("service") // listing_type
        .bind(service.core.category.as_deref().unwrap_or("other")) // category
        .bind("accepting") // capacity_status
        .bind("en") // source_language - TODO: detect from content
        .execute(&self.pool)
        .await
        .context("Failed to insert service listing")?;

        // Insert service-specific fields
        sqlx::query(
            r#"
            INSERT INTO service_listings (
                listing_id,
                requires_identification, requires_appointment, walk_ins_accepted,
                remote_available, in_person_available, home_visits_available,
                wheelchair_accessible, interpretation_available,
                free_service, sliding_scale_fees, accepts_insurance,
                evening_hours, weekend_hours
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#,
        )
        .bind(listing_id)
        .bind(service.requires_identification)
        .bind(service.requires_appointment)
        .bind(service.walk_ins_accepted)
        .bind(service.remote_available)
        .bind(service.in_person_available)
        .bind(service.home_visits_available)
        .bind(service.wheelchair_accessible)
        .bind(service.interpretation_available)
        .bind(service.free_service)
        .bind(service.sliding_scale_fees)
        .bind(service.accepts_insurance)
        .bind(service.evening_hours)
        .bind(service.weekend_hours)
        .execute(&self.pool)
        .await
        .context("Failed to insert service-specific fields")?;

        info!(
            listing_id = %listing_id,
            title = %service.core.title,
            "Created new service listing from extraction"
        );

        Ok(listing_id)
    }

    /// Process opportunity listing extraction
    async fn process_opportunity_extraction(
        &self,
        extraction: &RawExtraction,
        opportunity: ExtractedOpportunity,
        fingerprint: &str,
    ) -> Result<ListingId> {
        let listing_id = ListingId::new();
        let now = Utc::now();

        // TODO: Get or create domain_id from the page_url domain
        let domain_id: Option<DomainId> = None;

        // TODO: Get or create organization_id from organization_name
        let organization_id: Option<uuid::Uuid> = None;

        // Insert core listing
        sqlx::query(
            r#"
            INSERT INTO listings (
                id, organization_id, organization_name, title, description, description_markdown,
                tldr, urgency, status, content_hash,
                location, submission_type, domain_id, source_url,
                last_seen_at, created_at, updated_at,
                listing_type, category, capacity_status, source_language
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21
            )
            "#,
        )
        .bind(listing_id)
        .bind(organization_id)
        .bind(&opportunity.core.organization_name)
        .bind(&opportunity.core.title)
        .bind(&opportunity.core.description)
        .bind(None::<String>) // description_markdown
        .bind(opportunity.core.tldr.as_ref())
        .bind(opportunity.core.urgency.as_ref())
        .bind("pending_approval")
        .bind(Some(fingerprint))
        .bind(opportunity.core.location.as_ref())
        .bind("scraped")
        .bind(domain_id)
        .bind(extraction.page_url.as_str())
        .bind(now)
        .bind(now)
        .bind(now)
        .bind("opportunity")
        .bind(opportunity.core.category.as_deref().unwrap_or("other"))
        .bind("accepting")
        .bind("en")
        .execute(&self.pool)
        .await
        .context("Failed to insert opportunity listing")?;

        // Insert opportunity-specific fields
        sqlx::query(
            r#"
            INSERT INTO opportunity_listings (
                listing_id,
                opportunity_type, time_commitment,
                requires_background_check, minimum_age,
                skills_needed, remote_ok
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(listing_id)
        .bind(opportunity.opportunity_type.as_deref().unwrap_or("other"))
        .bind(opportunity.time_commitment.as_ref())
        .bind(opportunity.requires_background_check)
        .bind(opportunity.minimum_age)
        .bind(opportunity.skills_needed.as_ref())
        .bind(opportunity.remote_ok)
        .execute(&self.pool)
        .await
        .context("Failed to insert opportunity-specific fields")?;

        info!(
            listing_id = %listing_id,
            title = %opportunity.core.title,
            "Created new opportunity listing from extraction"
        );

        Ok(listing_id)
    }

    /// Process business listing extraction
    ///
    /// Note: Business listings create entries in the business_organizations table,
    /// which links to organizations (not listings directly).
    async fn process_business_extraction(
        &self,
        extraction: &RawExtraction,
        business: ExtractedBusiness,
        fingerprint: &str,
    ) -> Result<ListingId> {
        let listing_id = ListingId::new();
        let now = Utc::now();

        // TODO: Get or create domain_id from the page_url domain
        let domain_id: Option<DomainId> = None;

        // TODO: Get or create organization_id from organization_name
        // For businesses, we MUST have an organization_id to link to business_organizations
        let organization_id: Option<uuid::Uuid> = None;

        // Insert core listing
        sqlx::query(
            r#"
            INSERT INTO listings (
                id, organization_id, organization_name, title, description, description_markdown,
                tldr, urgency, status, content_hash,
                location, submission_type, domain_id, source_url,
                last_seen_at, created_at, updated_at,
                listing_type, category, capacity_status, source_language
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21
            )
            "#,
        )
        .bind(listing_id)
        .bind(organization_id)
        .bind(&business.core.organization_name)
        .bind(&business.core.title)
        .bind(&business.core.description)
        .bind(None::<String>)
        .bind(business.core.tldr.as_ref())
        .bind(business.core.urgency.as_ref())
        .bind("pending_approval")
        .bind(Some(fingerprint))
        .bind(business.core.location.as_ref())
        .bind("scraped")
        .bind(domain_id)
        .bind(extraction.page_url.as_str())
        .bind(now)
        .bind(now)
        .bind(now)
        .bind("business")
        .bind(business.core.category.as_deref().unwrap_or("other"))
        .bind("accepting")
        .bind("en")
        .execute(&self.pool)
        .await
        .context("Failed to insert business listing")?;

        // If we have organization_id, insert/update business_organizations
        if let Some(org_id) = organization_id {
            // TODO: Also resolve proceeds_beneficiary_id from proceeds_beneficiary name

            sqlx::query(
                r#"
                INSERT INTO business_organizations (
                    organization_id,
                    proceeds_percentage,
                    proceeds_beneficiary_id,
                    donation_link,
                    gift_card_link,
                    online_store_url,
                    created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (organization_id) DO UPDATE SET
                    proceeds_percentage = COALESCE(EXCLUDED.proceeds_percentage, business_organizations.proceeds_percentage),
                    proceeds_beneficiary_id = COALESCE(EXCLUDED.proceeds_beneficiary_id, business_organizations.proceeds_beneficiary_id),
                    donation_link = COALESCE(EXCLUDED.donation_link, business_organizations.donation_link),
                    gift_card_link = COALESCE(EXCLUDED.gift_card_link, business_organizations.gift_card_link),
                    online_store_url = COALESCE(EXCLUDED.online_store_url, business_organizations.online_store_url)
                "#,
            )
            .bind(org_id)
            .bind(business.proceeds_percentage.map(|p| p as f64))
            .bind(None::<uuid::Uuid>) // proceeds_beneficiary_id - TODO: resolve from name
            .bind(business.donation_link.as_ref())
            .bind(business.gift_card_link.as_ref())
            .bind(business.online_store_url.as_ref())
            .bind(now)
            .execute(&self.pool)
            .await
            .context("Failed to upsert business_organizations")?;

            info!(
                listing_id = %listing_id,
                organization_id = %org_id,
                title = %business.core.title,
                "Created/updated business listing and business_organizations"
            );
        } else {
            warn!(
                listing_id = %listing_id,
                organization_name = %business.core.organization_name,
                "Created business listing without organization_id - business_organizations not updated"
            );
        }

        Ok(listing_id)
    }

    /// Find existing listing by fingerprint (stored in content_hash)
    async fn find_by_fingerprint(
        &self,
        fingerprint: &str,
        listing_type: &str,
    ) -> Result<Option<ListingId>> {
        let row: Option<(uuid::Uuid,)> = sqlx::query_as(
            "SELECT id FROM listings WHERE content_hash = $1 AND listing_type = $2 LIMIT 1",
        )
        .bind(fingerprint)
        .bind(listing_type)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(id,)| Id::from_uuid(id)))
    }

    /// Calculate fingerprint for deduplication
    /// Uses organization name + title as the fingerprint basis
    fn calculate_fingerprint(&self, organization_name: &str, title: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(organization_name.trim().to_lowercase().as_bytes());
        hasher.update(b"|");
        hasher.update(title.trim().to_lowercase().as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Update last_seen_at for existing listing
    pub async fn mark_as_seen(&self, listing_id: ListingId) -> Result<()> {
        sqlx::query("UPDATE listings SET last_seen_at = NOW() WHERE id = $1")
            .bind(listing_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Mark listings as disappeared if not seen recently
    pub async fn mark_disappeared(&self, days_threshold: i64) -> Result<usize> {
        let result = sqlx::query(
            r#"
            UPDATE listings
            SET disappeared_at = NOW()
            WHERE last_seen_at < NOW() - INTERVAL '1 day' * $1
              AND disappeared_at IS NULL
              AND submission_type = 'scraped'
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
        let adapter = ListingAdapter {
            pool: PgPool::connect_lazy("postgres://localhost/test").unwrap(),
        };

        let fp1 = adapter.calculate_fingerprint("Food Bank", "Volunteers Needed");
        let fp2 = adapter.calculate_fingerprint("  Food Bank  ", "  Volunteers Needed  ");
        let fp3 = adapter.calculate_fingerprint("food bank", "volunteers needed");

        // All should produce same fingerprint (case-insensitive, whitespace-trimmed)
        assert_eq!(fp1, fp2);
        assert_eq!(fp2, fp3);
    }

    #[test]
    fn test_different_orgs_different_fingerprints() {
        let adapter = ListingAdapter {
            pool: PgPool::connect_lazy("postgres://localhost/test").unwrap(),
        };

        let fp1 = adapter.calculate_fingerprint("Food Bank A", "Volunteers Needed");
        let fp2 = adapter.calculate_fingerprint("Food Bank B", "Volunteers Needed");

        // Different organizations should have different fingerprints
        assert_ne!(fp1, fp2);
    }
}
