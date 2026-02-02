//! Readable trait for models that can be read from the database by ID
//!
//! This trait enables the ReadResult pattern for deferred database reads.

use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;

/// Trait for models that can be read from the database by their ID.
///
/// This enables the `ReadResult<T>` pattern where actions return a deferred
/// read handle instead of the model directly. The actual database read happens
/// after effects have settled.
///
/// # Example
///
/// ```rust,ignore
/// impl Readable for Member {
///     type Id = Uuid;
///
///     async fn read_by_id(id: Self::Id, pool: &PgPool) -> Result<Option<Self>> {
///         Member::find_by_id(id, pool).await.map(Some)
///     }
/// }
/// ```
#[async_trait]
pub trait Readable: Sized + Send + 'static {
    /// The type of the ID used to look up this model
    type Id: Send + Sync + Clone + 'static;

    /// Read the model from the database by its ID.
    /// Returns None if not found.
    async fn read_by_id(id: Self::Id, pool: &PgPool) -> Result<Option<Self>>;
}
