use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::PostId;

/// Contact type enum for type-safe querying
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContactType {
    Phone,
    Email,
    Website,
    Address,
    BookingUrl,
    Social,
}

impl std::fmt::Display for ContactType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContactType::Phone => write!(f, "phone"),
            ContactType::Email => write!(f, "email"),
            ContactType::Website => write!(f, "website"),
            ContactType::Address => write!(f, "address"),
            ContactType::BookingUrl => write!(f, "booking_url"),
            ContactType::Social => write!(f, "social"),
        }
    }
}

impl std::str::FromStr for ContactType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "phone" => Ok(ContactType::Phone),
            "email" => Ok(ContactType::Email),
            "website" => Ok(ContactType::Website),
            "address" => Ok(ContactType::Address),
            "booking_url" => Ok(ContactType::BookingUrl),
            "social" => Ok(ContactType::Social),
            _ => Err(anyhow::anyhow!("Invalid contact type: {}", s)),
        }
    }
}

/// Polymorphic contact information — any entity can have contacts.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Contact {
    pub id: Uuid,
    pub contactable_type: String,
    pub contactable_id: Uuid,
    pub contact_type: String,
    pub contact_value: String,
    pub contact_label: Option<String>,
    pub is_public: Option<bool>,
    pub display_order: Option<i32>,
}

impl Contact {
    // =========================================================================
    // Generic polymorphic methods
    // =========================================================================

    /// Find all contacts for an entity
    pub async fn find_by_entity(
        contactable_type: &str,
        contactable_id: Uuid,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM contacts
             WHERE contactable_type = $1 AND contactable_id = $2
             ORDER BY display_order, contact_type",
        )
        .bind(contactable_type)
        .bind(contactable_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Create a new contact
    pub async fn create(
        contactable_type: &str,
        contactable_id: Uuid,
        contact_type: ContactType,
        contact_value: String,
        contact_label: Option<String>,
        display_order: Option<i32>,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO contacts (contactable_type, contactable_id, contact_type, contact_value, contact_label, display_order)
             VALUES ($1, $2, $3, $4, $5, COALESCE($6, 0))
             RETURNING *",
        )
        .bind(contactable_type)
        .bind(contactable_id)
        .bind(contact_type.to_string())
        .bind(contact_value)
        .bind(contact_label)
        .bind(display_order)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Create multiple contacts from JSON contact info.
    /// Handles the common {"phone": "...", "email": "...", "website": "..."} format.
    pub async fn create_from_json(
        contactable_type: &str,
        contactable_id: Uuid,
        contact_info: &serde_json::Value,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let mut contacts = Vec::new();
        let mut order = 0;

        if let Some(obj) = contact_info.as_object() {
            if let Some(phone) = obj.get("phone").and_then(|v| v.as_str()) {
                if !phone.is_empty() {
                    contacts.push(
                        Self::create(contactable_type, contactable_id, ContactType::Phone, phone.to_string(), None, Some(order), pool).await?,
                    );
                    order += 1;
                }
            }

            if let Some(email) = obj.get("email").and_then(|v| v.as_str()) {
                if !email.is_empty() {
                    contacts.push(
                        Self::create(contactable_type, contactable_id, ContactType::Email, email.to_string(), None, Some(order), pool).await?,
                    );
                    order += 1;
                }
            }

            if let Some(website) = obj.get("website").and_then(|v| v.as_str()) {
                if !website.is_empty() {
                    contacts.push(
                        Self::create(contactable_type, contactable_id, ContactType::Website, website.to_string(), None, Some(order), pool).await?,
                    );
                    order += 1;
                }
            }

            if let Some(address) = obj.get("address").and_then(|v| v.as_str()) {
                if !address.is_empty() {
                    contacts.push(
                        Self::create(contactable_type, contactable_id, ContactType::Address, address.to_string(), None, Some(order), pool).await?,
                    );
                }
            }
        }

        Ok(contacts)
    }

    /// Delete all contacts for an entity
    pub async fn delete_all_for_entity(
        contactable_type: &str,
        contactable_id: Uuid,
        pool: &PgPool,
    ) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM contacts WHERE contactable_type = $1 AND contactable_id = $2",
        )
        .bind(contactable_type)
        .bind(contactable_id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    // =========================================================================
    // Convenience methods for posts (thin wrappers)
    // =========================================================================

    /// Find all contacts for a post
    pub async fn find_by_post(post_id: PostId, pool: &PgPool) -> Result<Vec<Self>> {
        Self::find_by_entity("post", post_id.into_uuid(), pool).await
    }

    /// Create contacts from JSON for a post
    pub async fn create_from_json_for_post(
        post_id: PostId,
        contact_info: &serde_json::Value,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        Self::create_from_json("post", post_id.into_uuid(), contact_info, pool).await
    }

    /// Delete all contacts for a post
    pub async fn delete_all_for_post(post_id: PostId, pool: &PgPool) -> Result<u64> {
        Self::delete_all_for_entity("post", post_id.into_uuid(), pool).await
    }
}
