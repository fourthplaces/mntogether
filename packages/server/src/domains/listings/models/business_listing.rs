use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{ListingId, OrganizationId};

/// Business-specific listing properties
///
/// Supports three models:
/// 1. Direct donation businesses (donation_link)
/// 2. Gift card sales (gift_card_link)
/// 3. Cause-driven commerce (proceeds_percentage + proceeds_beneficiary_id)
///
/// Example: Bailey Aro - sells merchandise where 15% goes to immigrant legal aid
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BusinessListing {
    pub listing_id: ListingId,

    // Basic business info
    pub business_type: Option<String>,
    pub support_needed: Option<Vec<String>>,
    pub current_situation: Option<String>,

    // Direct donations
    pub accepts_donations: bool,
    pub donation_link: Option<String>,

    // Gift cards
    pub gift_cards_available: bool,
    pub gift_card_link: Option<String>,

    // Cause-driven commerce (NEW)
    pub proceeds_percentage: Option<f64>,
    pub proceeds_beneficiary_id: Option<OrganizationId>,
    pub proceeds_description: Option<String>,
    pub impact_statement: Option<String>,

    // Commerce options
    pub remote_ok: bool,
    pub delivery_available: bool,
    pub online_ordering_link: Option<String>,
}

impl BusinessListing {
    /// Find business listing by listing ID
    pub async fn find_by_listing_id(
        listing_id: ListingId,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        let business = sqlx::query_as::<_, BusinessListing>(
            r#"
            SELECT * FROM business_listings
            WHERE listing_id = $1
            "#,
        )
        .bind(listing_id)
        .fetch_optional(pool)
        .await?;

        Ok(business)
    }

    /// Create a new business listing
    pub async fn create(
        listing_id: ListingId,
        business_type: Option<String>,
        pool: &PgPool,
    ) -> Result<Self> {
        let business = sqlx::query_as::<_, BusinessListing>(
            r#"
            INSERT INTO business_listings (listing_id, business_type)
            VALUES ($1, $2)
            RETURNING *
            "#,
        )
        .bind(listing_id)
        .bind(business_type)
        .fetch_one(pool)
        .await?;

        Ok(business)
    }

    /// Update proceeds allocation
    pub async fn update_proceeds(
        &mut self,
        proceeds_percentage: Option<f64>,
        beneficiary_id: Option<OrganizationId>,
        description: Option<String>,
        impact_statement: Option<String>,
        pool: &PgPool,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE business_listings
            SET proceeds_percentage = $2,
                proceeds_beneficiary_id = $3,
                proceeds_description = $4,
                impact_statement = $5
            WHERE listing_id = $1
            "#,
        )
        .bind(self.listing_id)
        .bind(proceeds_percentage)
        .bind(beneficiary_id)
        .bind(&description)
        .bind(&impact_statement)
        .execute(pool)
        .await?;

        // Update self
        self.proceeds_percentage = proceeds_percentage;
        self.proceeds_beneficiary_id = beneficiary_id;
        self.proceeds_description = description;
        self.impact_statement = impact_statement;

        Ok(())
    }

    /// Check if this is a cause-driven business
    pub fn is_cause_driven(&self) -> bool {
        self.proceeds_percentage.is_some() && self.proceeds_percentage.unwrap() > 0.0
    }

    /// Get all cause-driven businesses
    pub async fn find_cause_driven(pool: &PgPool) -> Result<Vec<Self>> {
        let businesses = sqlx::query_as::<_, BusinessListing>(
            r#"
            SELECT * FROM business_listings
            WHERE proceeds_percentage IS NOT NULL AND proceeds_percentage > 0
            ORDER BY proceeds_percentage DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(businesses)
    }
}
