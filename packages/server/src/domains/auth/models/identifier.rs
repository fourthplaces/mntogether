use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

/// Identifier - maps hashed phone numbers to members
///
/// Phone numbers are hashed for privacy - we never store raw phone numbers
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Identifier {
    pub id: Uuid,
    pub member_id: Uuid,
    pub phone_hash: String,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// SQL Queries - ALL queries must be in models/
// =============================================================================

impl Identifier {
    /// Find identifier by phone hash
    pub async fn find_by_phone_hash(phone_hash: &str, pool: &PgPool) -> Result<Option<Self>> {
        let identifier =
            sqlx::query_as::<_, Identifier>("SELECT * FROM identifiers WHERE phone_hash = $1")
                .bind(phone_hash)
                .fetch_optional(pool)
                .await?;
        Ok(identifier)
    }

    /// Check if phone hash exists
    pub async fn exists(phone_hash: &str, pool: &PgPool) -> Result<bool> {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM identifiers WHERE phone_hash = $1)",
        )
        .bind(phone_hash)
        .fetch_one(pool)
        .await?;
        Ok(exists)
    }

    /// Create identifier for a member
    pub async fn create(
        member_id: Uuid,
        phone_hash: String,
        is_admin: bool,
        pool: &PgPool,
    ) -> Result<Self> {
        let identifier = sqlx::query_as::<_, Identifier>(
            r#"
            INSERT INTO identifiers (member_id, phone_hash, is_admin)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(member_id)
        .bind(phone_hash)
        .bind(is_admin)
        .fetch_one(pool)
        .await?;
        Ok(identifier)
    }
}

// =============================================================================
// Utility Functions
// =============================================================================

/// Hash a phone number using SHA256
///
/// Phone numbers are hashed for privacy - we never store raw phone numbers.
/// The hash is used as a lookup key in the identifiers table.
pub fn hash_phone_number(phone_number: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(phone_number.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phone_hash_consistency() {
        let hash1 = hash_phone_number("+1234567890");
        let hash2 = hash_phone_number("+1234567890");
        assert_eq!(hash1, hash2, "Same phone should produce same hash");
    }

    #[test]
    fn test_phone_hash_uniqueness() {
        let hash1 = hash_phone_number("+1234567890");
        let hash2 = hash_phone_number("+9876543210");
        assert_ne!(
            hash1, hash2,
            "Different phones should have different hashes"
        );
    }

    #[test]
    fn test_phone_hash_format() {
        let hash = hash_phone_number("+1234567890");
        assert_eq!(hash.len(), 64, "SHA256 hash should be 64 hex characters");
        assert!(
            hash.chars().all(|c| c.is_ascii_hexdigit()),
            "Hash should only contain hex digits"
        );
    }
}
