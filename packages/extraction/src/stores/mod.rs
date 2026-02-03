//! Storage implementations for the extraction library.
//!
//! Available backends:
//! - `MemoryStore` - In-memory storage (always available)
//! - `SqliteStore` - SQLite file-based storage (requires `sqlite` feature)
//! - `PostgresStore` - PostgreSQL storage (requires `postgres` feature)

pub mod memory;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

pub use memory::MemoryStore;

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteStore;

#[cfg(feature = "postgres")]
pub use postgres::PostgresStore;
