//! Data migration: Normalize website URLs to just domain names
//!
//! **DEAD CODE**: The `websites` table was dropped in migration 149 (unified sources).
//! This migration ran before that and is no longer applicable.
//! All methods return no-op results.

use super::{DataMigration, MigrationContext, MigrationResult, VerifyResult};
use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

pub struct NormalizeWebsiteUrlsMigration;

#[async_trait]
impl DataMigration for NormalizeWebsiteUrlsMigration {
    fn name(&self) -> &'static str {
        "normalize_website_urls"
    }

    fn description(&self) -> &'static str {
        "Convert website URLs to normalized domain format (no longer applicable — websites table dropped in migration 149)"
    }

    async fn estimate(&self, _db: &PgPool) -> Result<i64> {
        Ok(0)
    }

    async fn find_work(&self, _cursor: Option<Uuid>, _limit: i64, _db: &PgPool) -> Result<Vec<Uuid>> {
        Ok(vec![])
    }

    async fn execute_one(&self, _id: Uuid, _ctx: &MigrationContext) -> Result<MigrationResult> {
        Ok(MigrationResult::Skipped)
    }

    async fn verify(&self, _db: &PgPool) -> Result<VerifyResult> {
        Ok(VerifyResult::Passed)
    }

    fn batch_size(&self) -> i64 {
        50
    }
}

/// Normalize a URL/domain to just the domain name
///
/// Examples:
/// - "https://www.example.org/page" -> "example.org"
/// - "http://www.example.org" -> "example.org"
/// - "www.example.org" -> "example.org"
fn normalize_domain(input: &str) -> Result<String> {
    let input_str = input.trim();

    // If no protocol, try adding https:// to parse it
    let with_protocol = if input_str.starts_with("http://") || input_str.starts_with("https://") {
        input_str.to_string()
    } else {
        format!("https://{}", input_str)
    };

    let parsed = url::Url::parse(&with_protocol)?;

    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("No host in domain: {}", input))?;

    // Normalize: lowercase and strip www. prefix
    let normalized = host
        .to_lowercase()
        .strip_prefix("www.")
        .map(|s| s.to_string())
        .unwrap_or_else(|| host.to_lowercase());

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_domain() {
        // Full URLs with protocol
        assert_eq!(
            normalize_domain("https://www.example.org/page").unwrap(),
            "example.org"
        );
        assert_eq!(
            normalize_domain("http://www.example.org").unwrap(),
            "example.org"
        );
        assert_eq!(
            normalize_domain("https://example.org/path/to/page").unwrap(),
            "example.org"
        );

        // Without protocol
        assert_eq!(normalize_domain("www.example.org").unwrap(), "example.org");
        assert_eq!(normalize_domain("example.org").unwrap(), "example.org");

        // Uppercase should be lowercased
        assert_eq!(
            normalize_domain("https://WWW.EXAMPLE.ORG").unwrap(),
            "example.org"
        );

        // Real-world examples
        assert_eq!(
            normalize_domain("https://www.dhhmn.com/").unwrap(),
            "dhhmn.com"
        );
        assert_eq!(normalize_domain("http://dhhmn.com").unwrap(), "dhhmn.com");

        // Subdomains preserved
        assert_eq!(
            normalize_domain("https://blog.example.org").unwrap(),
            "blog.example.org"
        );
    }
}
