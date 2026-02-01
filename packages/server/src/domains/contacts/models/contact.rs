use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{ContactId, PostId, OrganizationId, ProviderId, ResourceId};

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

/// Contactable type enum for type-safe querying
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContactableType {
    Organization,
    Listing,
    Provider,
    Resource,
}

impl std::fmt::Display for ContactableType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContactableType::Organization => write!(f, "organization"),
            ContactableType::Listing => write!(f, "listing"),
            ContactableType::Provider => write!(f, "provider"),
            ContactableType::Resource => write!(f, "resource"),
        }
    }
}

impl std::str::FromStr for ContactableType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "organization" => Ok(ContactableType::Organization),
            "listing" => Ok(ContactableType::Listing),
            "provider" => Ok(ContactableType::Provider),
            "resource" => Ok(ContactableType::Resource),
            _ => Err(anyhow::anyhow!("Invalid contactable type: {}", s)),
        }
    }
}

/// Polymorphic contact - stores contact info for any entity type
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Contact {
    pub id: ContactId,
    pub contactable_type: String,  // 'organization', 'listing', 'provider'
    pub contactable_id: Uuid,
    pub contact_type: String,      // 'phone', 'email', 'website', etc.
    pub contact_value: String,
    pub contact_label: Option<String>,  // 'Office', 'Mobile', 'LinkedIn'
    pub is_public: bool,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
}

/// Input for creating a new contact
#[derive(Debug, Clone)]
pub struct CreateContact {
    pub contactable_type: String,
    pub contactable_id: Uuid,
    pub contact_type: String,
    pub contact_value: String,
    pub contact_label: Option<String>,
    pub is_public: bool,
    pub display_order: i32,
}

impl Contact {
    /// Find contact by ID
    pub async fn find_by_id(id: ContactId, pool: &PgPool) -> Result<Self> {
        let contact = sqlx::query_as::<_, Self>("SELECT * FROM contacts WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await?;
        Ok(contact)
    }

    /// Find all contacts for a provider
    pub async fn find_for_provider(provider_id: ProviderId, pool: &PgPool) -> Result<Vec<Self>> {
        let contacts = sqlx::query_as::<_, Self>(
            r#"
            SELECT *
            FROM contacts
            WHERE contactable_type = 'provider' AND contactable_id = $1
            ORDER BY display_order ASC, created_at ASC
            "#,
        )
        .bind(provider_id.as_uuid())
        .fetch_all(pool)
        .await?;
        Ok(contacts)
    }

    /// Find all contacts for a listing
    pub async fn find_for_post(post_id: PostId, pool: &PgPool) -> Result<Vec<Self>> {
        let contacts = sqlx::query_as::<_, Self>(
            r#"
            SELECT *
            FROM contacts
            WHERE contactable_type = 'listing' AND contactable_id = $1
            ORDER BY display_order ASC, created_at ASC
            "#,
        )
        .bind(post_id.as_uuid())
        .fetch_all(pool)
        .await?;
        Ok(contacts)
    }

    /// Find all contacts for a resource
    pub async fn find_for_resource(resource_id: ResourceId, pool: &PgPool) -> Result<Vec<Self>> {
        let contacts = sqlx::query_as::<_, Self>(
            r#"
            SELECT *
            FROM contacts
            WHERE contactable_type = 'resource' AND contactable_id = $1
            ORDER BY display_order ASC, created_at ASC
            "#,
        )
        .bind(resource_id.as_uuid())
        .fetch_all(pool)
        .await?;
        Ok(contacts)
    }

    /// Find all contacts for an organization
    pub async fn find_for_organization(
        organization_id: OrganizationId,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let contacts = sqlx::query_as::<_, Self>(
            r#"
            SELECT *
            FROM contacts
            WHERE contactable_type = 'organization' AND contactable_id = $1
            ORDER BY display_order ASC, created_at ASC
            "#,
        )
        .bind(organization_id.as_uuid())
        .fetch_all(pool)
        .await?;
        Ok(contacts)
    }

    /// Find public contacts for a provider
    pub async fn find_public_for_provider(
        provider_id: ProviderId,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        let contacts = sqlx::query_as::<_, Self>(
            r#"
            SELECT *
            FROM contacts
            WHERE contactable_type = 'provider' AND contactable_id = $1 AND is_public = true
            ORDER BY display_order ASC, created_at ASC
            "#,
        )
        .bind(provider_id.as_uuid())
        .fetch_all(pool)
        .await?;
        Ok(contacts)
    }

    /// Create a new contact for a provider
    pub async fn create_for_provider(
        provider_id: ProviderId,
        contact_type: &str,
        contact_value: &str,
        contact_label: Option<String>,
        is_public: bool,
        display_order: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        Self::create(
            CreateContact {
                contactable_type: "provider".to_string(),
                contactable_id: *provider_id.as_uuid(),
                contact_type: contact_type.to_string(),
                contact_value: contact_value.to_string(),
                contact_label,
                is_public,
                display_order,
            },
            pool,
        )
        .await
    }

    /// Create a new contact for a listing
    pub async fn create_for_post(
        post_id: PostId,
        contact_type: &str,
        contact_value: &str,
        contact_label: Option<String>,
        is_public: bool,
        display_order: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        Self::create(
            CreateContact {
                contactable_type: "listing".to_string(),
                contactable_id: *post_id.as_uuid(),
                contact_type: contact_type.to_string(),
                contact_value: contact_value.to_string(),
                contact_label,
                is_public,
                display_order,
            },
            pool,
        )
        .await
    }

    /// Create a new contact for a resource
    pub async fn create_for_resource(
        resource_id: ResourceId,
        contact_type: &str,
        contact_value: &str,
        contact_label: Option<String>,
        is_public: bool,
        display_order: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        Self::create(
            CreateContact {
                contactable_type: "resource".to_string(),
                contactable_id: *resource_id.as_uuid(),
                contact_type: contact_type.to_string(),
                contact_value: contact_value.to_string(),
                contact_label,
                is_public,
                display_order,
            },
            pool,
        )
        .await
    }

    /// Create a new contact for an organization
    pub async fn create_for_organization(
        organization_id: OrganizationId,
        contact_type: &str,
        contact_value: &str,
        contact_label: Option<String>,
        is_public: bool,
        display_order: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        Self::create(
            CreateContact {
                contactable_type: "organization".to_string(),
                contactable_id: *organization_id.as_uuid(),
                contact_type: contact_type.to_string(),
                contact_value: contact_value.to_string(),
                contact_label,
                is_public,
                display_order,
            },
            pool,
        )
        .await
    }

    /// Generic create method
    pub async fn create(input: CreateContact, pool: &PgPool) -> Result<Self> {
        let contact = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO contacts (
                contactable_type, contactable_id, contact_type, contact_value,
                contact_label, is_public, display_order
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (contactable_type, contactable_id, contact_type, contact_value)
            DO UPDATE SET
                contact_label = COALESCE(EXCLUDED.contact_label, contacts.contact_label),
                is_public = EXCLUDED.is_public,
                display_order = EXCLUDED.display_order
            RETURNING *
            "#,
        )
        .bind(&input.contactable_type)
        .bind(&input.contactable_id)
        .bind(&input.contact_type)
        .bind(&input.contact_value)
        .bind(&input.contact_label)
        .bind(input.is_public)
        .bind(input.display_order)
        .fetch_one(pool)
        .await?;
        Ok(contact)
    }

    /// Update a contact
    pub async fn update(
        id: ContactId,
        contact_value: &str,
        contact_label: Option<String>,
        is_public: bool,
        display_order: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        let contact = sqlx::query_as::<_, Self>(
            r#"
            UPDATE contacts
            SET contact_value = $2, contact_label = $3, is_public = $4, display_order = $5
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(contact_value)
        .bind(contact_label)
        .bind(is_public)
        .bind(display_order)
        .fetch_one(pool)
        .await?;
        Ok(contact)
    }

    /// Delete a contact
    pub async fn delete(id: ContactId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM contacts WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Delete all contacts for a provider
    pub async fn delete_all_for_provider(provider_id: ProviderId, pool: &PgPool) -> Result<()> {
        sqlx::query(
            "DELETE FROM contacts WHERE contactable_type = 'provider' AND contactable_id = $1",
        )
        .bind(provider_id.as_uuid())
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete all contacts for a listing
    pub async fn delete_all_for_post(post_id: PostId, pool: &PgPool) -> Result<()> {
        sqlx::query(
            "DELETE FROM contacts WHERE contactable_type = 'listing' AND contactable_id = $1",
        )
        .bind(post_id.as_uuid())
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete all contacts for a resource
    pub async fn delete_all_for_resource(resource_id: ResourceId, pool: &PgPool) -> Result<()> {
        sqlx::query(
            "DELETE FROM contacts WHERE contactable_type = 'resource' AND contactable_id = $1",
        )
        .bind(resource_id.as_uuid())
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete all contacts for an organization
    pub async fn delete_all_for_organization(
        organization_id: OrganizationId,
        pool: &PgPool,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM contacts WHERE contactable_type = 'organization' AND contactable_id = $1",
        )
        .bind(organization_id.as_uuid())
        .execute(pool)
        .await?;
        Ok(())
    }
}
