//! Data migration: Normalize website URLs to just domain names
//!
//! Converts URLs like "https://www.dhhmn.com/" to "dhhmn.com"
//!
//! This migration:
//! 1. Finds all websites with URLs that aren't normalized (contain protocol, www, or path)
//! 2. Normalizes each URL to just the domain
//! 3. Handles duplicates by merging (updates foreign keys, deletes the duplicate)

use super::{DataMigration, MigrationContext, MigrationResult, VerifyResult};
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

pub struct NormalizeWebsiteUrlsMigration;

#[async_trait]
impl DataMigration for NormalizeWebsiteUrlsMigration {
    fn name(&self) -> &'static str {
        "normalize_website_urls"
    }

    fn description(&self) -> &'static str {
        "Convert website URLs to normalized domain format (e.g., https://www.example.com/ -> example.com)"
    }

    async fn estimate(&self, db: &PgPool) -> Result<i64> {
        // Count websites where domain is not normalized
        // A domain is not normalized if it:
        // - Contains "://" (has protocol)
        // - Starts with "www."
        // - Contains "/" after the domain
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM websites
            WHERE domain ~ '^https?://'
               OR domain ~ '^www\.'
               OR domain ~ '/.'
            "#,
        )
        .fetch_one(db)
        .await?;

        Ok(count.0)
    }

    async fn find_work(&self, cursor: Option<Uuid>, limit: i64, db: &PgPool) -> Result<Vec<Uuid>> {
        let ids: Vec<(Uuid,)> = match cursor {
            Some(c) => {
                sqlx::query_as(
                    r#"
                    SELECT id
                    FROM websites
                    WHERE (domain ~ '^https?://' OR domain ~ '^www\.' OR domain ~ '/.')
                      AND id > $1
                    ORDER BY id
                    LIMIT $2
                    "#,
                )
                .bind(c)
                .bind(limit)
                .fetch_all(db)
                .await?
            }
            None => {
                sqlx::query_as(
                    r#"
                    SELECT id
                    FROM websites
                    WHERE domain ~ '^https?://' OR domain ~ '^www\.' OR domain ~ '/.'
                    ORDER BY id
                    LIMIT $1
                    "#,
                )
                .bind(limit)
                .fetch_all(db)
                .await?
            }
        };

        Ok(ids.into_iter().map(|(id,)| id).collect())
    }

    async fn execute_one(&self, id: Uuid, ctx: &MigrationContext) -> Result<MigrationResult> {
        // Get current domain
        let row: Option<(String,)> = sqlx::query_as("SELECT domain FROM websites WHERE id = $1")
            .bind(id)
            .fetch_optional(&ctx.db_pool)
            .await?;

        let current_domain = match row {
            Some((domain,)) => domain,
            None => return Ok(MigrationResult::Skipped), // Row no longer exists
        };

        // Normalize the domain
        let normalized = normalize_domain(&current_domain)?;

        // Check if already normalized
        if normalized == current_domain {
            return Ok(MigrationResult::Skipped);
        }

        info!(
            website_id = %id,
            from = %current_domain,
            to = %normalized,
            "Normalizing website domain"
        );

        if ctx.dry_run {
            // Check if this would create a duplicate
            let existing: Option<(Uuid,)> =
                sqlx::query_as("SELECT id FROM websites WHERE domain = $1 AND id != $2")
                    .bind(&normalized)
                    .bind(id)
                    .fetch_optional(&ctx.db_pool)
                    .await?;

            if existing.is_some() {
                info!(
                    website_id = %id,
                    normalized_domain = %normalized,
                    "Would merge into existing website (duplicate)"
                );
            }

            return Ok(MigrationResult::WouldMigrate);
        }

        // Check if normalizing would create a duplicate
        let existing: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM websites WHERE domain = $1 AND id != $2")
                .bind(&normalized)
                .bind(id)
                .fetch_optional(&ctx.db_pool)
                .await?;

        if let Some((existing_id,)) = existing {
            // Merge this website into the existing one
            merge_websites(id, existing_id, &ctx.db_pool).await?;
        } else {
            // Just update the domain
            sqlx::query("UPDATE websites SET domain = $1, updated_at = NOW() WHERE id = $2")
                .bind(&normalized)
                .bind(id)
                .execute(&ctx.db_pool)
                .await?;
        }

        Ok(MigrationResult::Migrated)
    }

    async fn verify(&self, db: &PgPool) -> Result<VerifyResult> {
        // Check if any domains are still not normalized
        let remaining: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM websites
            WHERE domain ~ '^https?://'
               OR domain ~ '^www\.'
               OR domain ~ '/.'
            "#,
        )
        .fetch_one(db)
        .await?;

        if remaining.0 == 0 {
            // Also check for duplicates
            let duplicates: (i64,) = sqlx::query_as(
                r#"
                SELECT COUNT(*) FROM (
                    SELECT domain FROM websites
                    GROUP BY domain
                    HAVING COUNT(*) > 1
                ) AS dups
                "#,
            )
            .fetch_one(db)
            .await?;

            if duplicates.0 > 0 {
                return Ok(VerifyResult::Failed {
                    issues: vec![format!(
                        "{} duplicate domains found after normalization",
                        duplicates.0
                    )],
                });
            }

            Ok(VerifyResult::Passed)
        } else {
            Ok(VerifyResult::Incomplete {
                remaining: remaining.0,
            })
        }
    }

    fn batch_size(&self) -> i64 {
        50 // Smaller batches due to potential merges
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

    let parsed = url::Url::parse(&with_protocol)
        .with_context(|| format!("Failed to parse domain: {}", input))?;

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

/// Merge website `from_id` into website `to_id`
///
/// This updates all foreign key references and then deletes the duplicate.
async fn merge_websites(from_id: Uuid, to_id: Uuid, db: &PgPool) -> Result<()> {
    info!(
        from_id = %from_id,
        to_id = %to_id,
        "Merging duplicate website"
    );

    // Update listings to point to the kept website
    sqlx::query("UPDATE posts SET website_id = $1 WHERE website_id = $2")
        .bind(to_id)
        .bind(from_id)
        .execute(db)
        .await?;

    // Update website_snapshots
    sqlx::query("UPDATE website_snapshots SET website_id = $1 WHERE website_id = $2")
        .bind(to_id)
        .bind(from_id)
        .execute(db)
        .await?;

    // Update website_assessments
    sqlx::query("UPDATE website_assessments SET website_id = $1 WHERE website_id = $2")
        .bind(to_id)
        .bind(from_id)
        .execute(db)
        .await?;

    // Update website_research
    sqlx::query("UPDATE website_research SET website_id = $1 WHERE website_id = $2")
        .bind(to_id)
        .bind(from_id)
        .execute(db)
        .await?;

    // Update post_website_sync (handle potential unique constraint)
    // First, delete any sync records that would conflict
    sqlx::query(
        r#"
        DELETE FROM post_website_sync lws1
        USING post_website_sync lws2
        WHERE lws1.website_id = $1
          AND lws2.website_id = $2
          AND lws1.post_id = lws2.post_id
        "#,
    )
    .bind(from_id)
    .bind(to_id)
    .execute(db)
    .await?;

    // Now update the remaining records
    sqlx::query("UPDATE post_website_sync SET website_id = $1 WHERE website_id = $2")
        .bind(to_id)
        .bind(from_id)
        .execute(db)
        .await?;

    // Delete the duplicate website
    sqlx::query("DELETE FROM websites WHERE id = $1")
        .bind(from_id)
        .execute(db)
        .await?;

    Ok(())
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
