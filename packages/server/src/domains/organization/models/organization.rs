use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use uuid::Uuid;

/// Organization - anchor entity for grouping needs and sources
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub contact_info: Option<JsonValue>, // { email, phone, website }
    pub location: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub status: String, // 'pending' | 'active' | 'inactive'
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Status enum for type-safe edges
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationStatus {
    Pending,
    Active,
    Inactive,
}

impl OrganizationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Inactive => "inactive",
        }
    }
}

// =============================================================================
// SQL Queries - ALL queries must be in models/
// =============================================================================

impl Organization {
    /// Find organization by ID
    pub async fn find_by_id(id: Uuid, pool: &PgPool) -> Result<Self> {
        let org = sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await?;
        Ok(org)
    }

    /// Find organization by name (exact match)
    pub async fn find_by_name(name: &str, pool: &PgPool) -> Result<Option<Self>> {
        let org = sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE name = $1")
            .bind(name)
            .fetch_optional(pool)
            .await?;
        Ok(org)
    }

    /// Find all active organizations
    pub async fn find_active(pool: &PgPool) -> Result<Vec<Self>> {
        let orgs = sqlx::query_as::<_, Organization>(
            "SELECT * FROM organizations WHERE status = 'active' ORDER BY name",
        )
        .fetch_all(pool)
        .await?;
        Ok(orgs)
    }

    /// Search organizations by name (fuzzy)
    pub async fn search_by_name(query: &str, pool: &PgPool) -> Result<Vec<Self>> {
        let orgs = sqlx::query_as::<_, Organization>(
            r#"
            SELECT * FROM organizations
            WHERE status = 'active'
              AND name ILIKE $1
            ORDER BY name
            LIMIT 20
            "#,
        )
        .bind(format!("%{}%", query))
        .fetch_all(pool)
        .await?;
        Ok(orgs)
    }

    /// Insert new organization
    pub async fn insert(&self, pool: &PgPool) -> Result<Self> {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            INSERT INTO organizations (
                id, name, description, contact_info, location, city, state, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(self.id)
        .bind(&self.name)
        .bind(&self.description)
        .bind(&self.contact_info)
        .bind(&self.location)
        .bind(&self.city)
        .bind(&self.state)
        .bind(&self.status)
        .bind(self.created_at)
        .bind(self.updated_at)
        .fetch_one(pool)
        .await?;
        Ok(org)
    }

    /// Update organization
    pub async fn update(&self, pool: &PgPool) -> Result<Self> {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            UPDATE organizations
            SET name = $2,
                description = $3,
                contact_info = $4,
                location = $5,
                city = $6,
                state = $7,
                status = $8,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(self.id)
        .bind(&self.name)
        .bind(&self.description)
        .bind(&self.contact_info)
        .bind(&self.location)
        .bind(&self.city)
        .bind(&self.state)
        .bind(&self.status)
        .fetch_one(pool)
        .await?;
        Ok(org)
    }

    /// Set organization status
    pub async fn set_status(id: Uuid, status: OrganizationStatus, pool: &PgPool) -> Result<Self> {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            UPDATE organizations
            SET status = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status.as_str())
        .fetch_one(pool)
        .await?;
        Ok(org)
    }
}
