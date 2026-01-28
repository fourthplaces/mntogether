use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

/// Identifier - maps hashed phone numbers or emails to members
///
/// Identifiers (phone numbers or emails) are hashed for privacy.
/// We never store raw identifiers in plaintext.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Identifier {
    pub id: Uuid,
    pub member_id: Uuid,
    pub phone_hash: String, // Actually stores hash of phone number OR email
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

/// Hash an identifier (phone number or email) using SHA256
///
/// Identifiers are hashed for privacy - we never store raw identifiers.
/// The hash is used as a lookup key in the identifiers table.
///
/// Note: Function named `hash_phone_number` for backward compatibility,
/// but it works for any string identifier (phone or email).
pub fn hash_phone_number(phone_number: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(phone_number.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Check if an identifier (email or phone) should be granted admin privileges
///
/// Returns true if the identifier is in the admin_identifiers list.
/// Supports both emails and phone numbers.
///
/// - For emails: case-insensitive matching
/// - For phone numbers: exact match (E.164 format)
pub fn is_admin_identifier(identifier: &str, admin_identifiers: &[String]) -> bool {
    admin_identifiers.iter().any(|admin_id| {
        // Case-insensitive match for emails
        if identifier.contains('@') && admin_id.contains('@') {
            admin_id.eq_ignore_ascii_case(identifier)
        } else {
            // Exact match for phone numbers
            admin_id == identifier
        }
    })
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

    #[test]
    fn test_email_hash_works() {
        // Function works for emails too, not just phones
        let hash1 = hash_phone_number("user@example.com");
        let hash2 = hash_phone_number("user@example.com");
        assert_eq!(hash1, hash2, "Same email should produce same hash");

        let hash3 = hash_phone_number("other@example.com");
        assert_ne!(
            hash1, hash3,
            "Different emails should have different hashes"
        );
    }

    #[test]
    fn test_is_admin_identifier_email() {
        let admin_identifiers = vec![
            "admin@example.com".to_string(),
            "owner@example.com".to_string(),
        ];

        assert!(is_admin_identifier("admin@example.com", &admin_identifiers));
        assert!(is_admin_identifier("owner@example.com", &admin_identifiers));
        assert!(!is_admin_identifier("user@example.com", &admin_identifiers));
    }

    #[test]
    fn test_is_admin_identifier_case_insensitive() {
        let admin_identifiers = vec!["Admin@Example.com".to_string()];

        assert!(is_admin_identifier("admin@example.com", &admin_identifiers));
        assert!(is_admin_identifier("ADMIN@EXAMPLE.COM", &admin_identifiers));
        assert!(is_admin_identifier("Admin@Example.com", &admin_identifiers));
    }

    #[test]
    fn test_is_admin_identifier_phone() {
        let admin_identifiers = vec!["+1234567890".to_string(), "+15551234567".to_string()];

        // Phone numbers match exactly
        assert!(is_admin_identifier("+1234567890", &admin_identifiers));
        assert!(is_admin_identifier("+15551234567", &admin_identifiers));
        assert!(!is_admin_identifier("+9876543210", &admin_identifiers));
    }

    #[test]
    fn test_is_admin_identifier_mixed() {
        let admin_identifiers = vec![
            "admin@example.com".to_string(),
            "+1234567890".to_string(),
        ];

        // Both emails and phones work
        assert!(is_admin_identifier("admin@example.com", &admin_identifiers));
        assert!(is_admin_identifier("+1234567890", &admin_identifiers));
        assert!(!is_admin_identifier("user@example.com", &admin_identifiers));
        assert!(!is_admin_identifier("+9876543210", &admin_identifiers));
    }
}
