use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::PostId;

/// Contact type enum matching database constraint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContactType {
    Phone,
    Email,
    Website,
    Address,
}

impl std::fmt::Display for ContactType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContactType::Phone => write!(f, "phone"),
            ContactType::Email => write!(f, "email"),
            ContactType::Website => write!(f, "website"),
            ContactType::Address => write!(f, "address"),
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
            _ => Err(anyhow::anyhow!("Invalid contact type: {}", s)),
        }
    }
}

/// PostContact - contact information for a listing
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PostContact {
    pub id: Uuid,
    pub post_id: Uuid,
    pub contact_type: String,
    pub contact_value: String,
    pub contact_label: Option<String>,
    pub display_order: i32,
}

impl PostContact {
    /// Get post_id as typed ID
    pub fn get_post_id(&self) -> PostId {
        PostId::from_uuid(self.post_id)
    }

    /// Find all contacts for a listing
    pub async fn find_by_post(post_id: PostId, pool: &PgPool) -> Result<Vec<Self>> {
        let post_uuid = post_id.into_uuid();

        sqlx::query_as::<_, Self>(
            "SELECT * FROM post_contacts
             WHERE post_id = $1
             ORDER BY display_order, contact_type",
        )
        .bind(post_uuid)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Create a new contact
    pub async fn create(
        post_id: PostId,
        contact_type: ContactType,
        contact_value: String,
        contact_label: Option<String>,
        display_order: Option<i32>,
        pool: &PgPool,
    ) -> Result<Self> {
        let post_uuid = post_id.into_uuid();

        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO post_contacts (post_id, contact_type, contact_value, contact_label, display_order)
            VALUES ($1, $2, $3, $4, COALESCE($5, 0))
            RETURNING *
            "#,
        )
        .bind(post_uuid)
        .bind(contact_type.to_string())
        .bind(contact_value)
        .bind(contact_label)
        .bind(display_order)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Create multiple contacts from JSON contact info
    /// Handles the common {"phone": "...", "email": "...", "website": "..."} format
    pub async fn create_from_json(
        post_id: PostId,
        contact_info: &serde_json::Value,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let mut contacts = Vec::new();
        let mut order = 0;

        if let Some(obj) = contact_info.as_object() {
            // Phone
            if let Some(phone) = obj.get("phone").and_then(|v| v.as_str()) {
                if !phone.is_empty() {
                    contacts.push(
                        Self::create(
                            post_id,
                            ContactType::Phone,
                            phone.to_string(),
                            None,
                            Some(order),
                            pool,
                        )
                        .await?,
                    );
                    order += 1;
                }
            }

            // Email
            if let Some(email) = obj.get("email").and_then(|v| v.as_str()) {
                if !email.is_empty() {
                    contacts.push(
                        Self::create(
                            post_id,
                            ContactType::Email,
                            email.to_string(),
                            None,
                            Some(order),
                            pool,
                        )
                        .await?,
                    );
                    order += 1;
                }
            }

            // Website
            if let Some(website) = obj.get("website").and_then(|v| v.as_str()) {
                if !website.is_empty() {
                    contacts.push(
                        Self::create(
                            post_id,
                            ContactType::Website,
                            website.to_string(),
                            None,
                            Some(order),
                            pool,
                        )
                        .await?,
                    );
                    order += 1;
                }
            }

            // Address
            if let Some(address) = obj.get("address").and_then(|v| v.as_str()) {
                if !address.is_empty() {
                    contacts.push(
                        Self::create(
                            post_id,
                            ContactType::Address,
                            address.to_string(),
                            None,
                            Some(order),
                            pool,
                        )
                        .await?,
                    );
                }
            }
        }

        Ok(contacts)
    }

    /// Delete all contacts for a listing
    pub async fn delete_all_for_post(post_id: PostId, pool: &PgPool) -> Result<u64> {
        let post_uuid = post_id.into_uuid();

        let result = sqlx::query("DELETE FROM post_contacts WHERE post_id = $1")
            .bind(post_uuid)
            .execute(pool)
            .await?;

        Ok(result.rows_affected())
    }
}
