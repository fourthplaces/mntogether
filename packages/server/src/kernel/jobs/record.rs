//! Record trait for CRUD operations.
//!
//! Simple trait for database models that need standard operations.

use anyhow::Result;
use async_trait::async_trait;

use crate::kernel::ServerKernel;

/// Trait for database records with CRUD operations.
#[async_trait]
pub trait Record: Sized + Send + Sync {
    /// The table name for this record type.
    const TABLE: &'static str;

    /// The ID type for this record.
    type Id;

    /// Find a record by its ID.
    async fn find_by_id(id: Self::Id, db: &sqlx::PgPool) -> Result<Self>;

    /// Insert a new record.
    async fn insert(&self, kernel: &ServerKernel) -> Result<Self>;

    /// Update an existing record.
    async fn update(&self, kernel: &ServerKernel) -> Result<Self>;

    /// Delete a record.
    async fn delete(&self, kernel: &ServerKernel) -> Result<()>;

    /// Re-read a record from the database.
    async fn read(&self, kernel: &ServerKernel) -> Result<Self>;
}
