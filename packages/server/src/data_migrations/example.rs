//! Example data migration demonstrating the pattern
//!
//! This is a template migration that shows how to implement the DataMigration trait.
//! Copy this file and modify for your specific migration needs.

use super::{DataMigration, MigrationContext, MigrationResult, VerifyResult};
use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

/// Example migration that demonstrates the pattern
///
/// This example shows how to:
/// 1. Query for items needing migration
/// 2. Process items in batches with stable cursoring
/// 3. Handle dry-run mode
/// 4. Verify completion
pub struct ExampleMigration;

#[async_trait]
impl DataMigration for ExampleMigration {
    fn name(&self) -> &'static str {
        "example_migration"
    }

    fn description(&self) -> &'static str {
        "Example migration demonstrating the pattern"
    }

    async fn estimate(&self, db: &PgPool) -> Result<i64> {
        // Count items that need migration
        // Replace with your actual query
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM organizations
            WHERE updated_at < created_at  -- Example condition
            "#,
        )
        .fetch_one(db)
        .await?;

        Ok(count.0)
    }

    async fn find_work(&self, cursor: Option<Uuid>, limit: i64, db: &PgPool) -> Result<Vec<Uuid>> {
        // Find items to migrate, ordered by id for stable cursoring
        // The cursor is the last processed id (exclusive)
        let ids: Vec<(Uuid,)> = match cursor {
            Some(c) => {
                sqlx::query_as(
                    r#"
                    SELECT id
                    FROM organizations
                    WHERE updated_at < created_at
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
                    FROM organizations
                    WHERE updated_at < created_at
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
        // Check if this item still needs migration (idempotency check)
        let needs_migration: Option<(bool,)> = sqlx::query_as(
            r#"
            SELECT updated_at < created_at as needs_migration
            FROM organizations
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&ctx.db_pool)
        .await?;

        match needs_migration {
            None => {
                // Item no longer exists
                return Ok(MigrationResult::Skipped);
            }
            Some((false,)) => {
                // Already migrated
                return Ok(MigrationResult::Skipped);
            }
            Some((true,)) => {
                // Needs migration
            }
        }

        // Handle dry-run mode
        if ctx.dry_run {
            return Ok(MigrationResult::WouldMigrate);
        }

        // Perform the actual migration
        sqlx::query(
            r#"
            UPDATE organizations
            SET updated_at = created_at
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&ctx.db_pool)
        .await?;

        Ok(MigrationResult::Migrated)
    }

    async fn verify(&self, db: &PgPool) -> Result<VerifyResult> {
        // Check if any items still need migration
        let remaining: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM organizations
            WHERE updated_at < created_at
            "#,
        )
        .fetch_one(db)
        .await?;

        if remaining.0 == 0 {
            Ok(VerifyResult::Passed)
        } else {
            Ok(VerifyResult::Incomplete {
                remaining: remaining.0,
            })
        }
    }

    fn batch_size(&self) -> i64 {
        100
    }

    fn error_budget(&self) -> f64 {
        0.01 // 1% error rate allowed
    }
}
