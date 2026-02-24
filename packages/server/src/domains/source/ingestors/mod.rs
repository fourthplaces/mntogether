//! Social platform ingestors implementing the extraction library's Ingestor trait.
//!
//! Each ingestor wraps Apify scraping for a specific platform and outputs
//! uniform RawPage objects, making social content indistinguishable from
//! website content for downstream extraction.

mod facebook;
mod instagram;
mod x;

pub use facebook::FacebookIngestor;
pub use instagram::InstagramIngestor;
pub use x::XIngestor;
