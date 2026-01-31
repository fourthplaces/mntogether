//! Data migration framework for surgical database transformations
//!
//! This module provides the infrastructure for running resumable, batch-oriented
//! data migrations with progress tracking, error budgets, and verification.
//!
//! # Architecture
//!
//! Data migrations are different from schema migrations (sqlx):
//! - Schema migrations change the database structure
//! - Data migrations transform data within existing structures
//!
//! # Usage
//!
//! 1. Implement the `DataMigration` trait for your migration
//! 2. Register it in the `MIGRATIONS` list
//! 3. Run via `./dev.sh migrate <name>`
//!
//! # Example
//!
//! ```rust,ignore
//! pub struct NormalizeEmailsMigration;
//!
//! #[async_trait]
//! impl DataMigration for NormalizeEmailsMigration {
//!     fn name(&self) -> &'static str { "normalize_emails" }
//!
//!     async fn estimate(&self, db: &PgPool) -> Result<i64> {
//!         sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE email != LOWER(email)")
//!             .fetch_one(db).await.map_err(Into::into)
//!     }
//!
//!     async fn find_work(&self, cursor: Option<Uuid>, limit: i64, db: &PgPool) -> Result<Vec<Uuid>> {
//!         // Query for items needing migration, ordered by id for stable cursoring
//!     }
//!
//!     async fn execute_one(&self, id: Uuid, ctx: &MigrationContext) -> Result<MigrationResult> {
//!         if ctx.dry_run {
//!             return Ok(MigrationResult::WouldMigrate);
//!         }
//!         // Perform the migration
//!         Ok(MigrationResult::Migrated)
//!     }
//!
//!     async fn verify(&self, db: &PgPool) -> Result<VerifyResult> {
//!         // Verify migration is complete
//!     }
//! }
//! ```

pub mod example;
pub mod normalize_website_urls;
mod workflow;

pub use workflow::MigrationWorkflow;

use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

/// Result of executing a single item migration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationResult {
    /// Item was successfully migrated
    Migrated,
    /// Item was skipped (already migrated or not applicable)
    Skipped,
    /// Dry-run: item would have been migrated
    WouldMigrate,
    /// Dry-run: item would have been skipped
    WouldSkip,
}

/// Result of verification check
#[derive(Debug)]
pub enum VerifyResult {
    /// All items have been migrated
    Passed,
    /// Some items remain to be migrated
    Incomplete { remaining: i64 },
    /// Verification failed with issues
    Failed { issues: Vec<String> },
}

/// Context passed to migration execution
pub struct MigrationContext {
    /// Database connection pool
    pub db_pool: PgPool,
    /// Whether this is a dry-run (no mutations)
    pub dry_run: bool,
}

/// Trait for implementing data migrations
///
/// Each migration must be:
/// - Idempotent: running multiple times produces the same result
/// - Resumable: can continue from where it left off via cursor
/// - Verifiable: can check that migration completed correctly
#[async_trait]
pub trait DataMigration: Send + Sync + 'static {
    /// Unique name for this migration (used as key in workflow table)
    fn name(&self) -> &'static str;

    /// Optional description shown in migration list
    fn description(&self) -> &'static str {
        ""
    }

    /// Estimate total items to migrate
    async fn estimate(&self, db: &PgPool) -> Result<i64>;

    /// Find the next batch of items to migrate
    ///
    /// Must return items ordered by id for stable cursoring.
    /// The cursor is the last processed id (exclusive).
    async fn find_work(&self, cursor: Option<Uuid>, limit: i64, db: &PgPool) -> Result<Vec<Uuid>>;

    /// Execute migration for a single item
    async fn execute_one(&self, id: Uuid, ctx: &MigrationContext) -> Result<MigrationResult>;

    /// Verify that the migration is complete
    async fn verify(&self, db: &PgPool) -> Result<VerifyResult>;

    /// Batch size for processing items (default: 100)
    fn batch_size(&self) -> i64 {
        100
    }

    /// Maximum acceptable error rate before stopping (default: 1%)
    fn error_budget(&self) -> f64 {
        0.01
    }
}

/// Registry entry for a migration
pub struct MigrationEntry {
    pub migration: Box<dyn DataMigration>,
}

impl MigrationEntry {
    pub fn new<M: DataMigration>(m: M) -> Self {
        Self {
            migration: Box::new(m),
        }
    }
}

/// Get all registered migrations
///
/// Add new migrations to this function.
pub fn all_migrations() -> Vec<MigrationEntry> {
    vec![
        // Register migrations here:
        MigrationEntry::new(example::ExampleMigration),
        MigrationEntry::new(normalize_website_urls::NormalizeWebsiteUrlsMigration),
    ]
}

/// Find a migration by name
pub fn find_migration(name: &str) -> Option<MigrationEntry> {
    all_migrations().into_iter().find(|e| e.migration.name() == name)
}
