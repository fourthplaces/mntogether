//! Custom ingestors for the crawling domain.
//!
//! These ingestors implement the extraction library's `Ingestor` trait
//! but read from server-specific data sources.

pub mod page_snapshot;

pub use page_snapshot::PageSnapshotIngestor;
