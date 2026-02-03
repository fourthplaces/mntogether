//! Website domain models.

#![allow(deprecated)] // Re-exports deprecated WebsiteSnapshot

pub mod website;
pub mod website_assessment;
pub mod website_research;

pub use website::*;
pub use website_assessment::*;
pub use website_research::*;

// WebsiteSnapshot has been moved to the crawling domain
// Re-export for backward compatibility
// Note: WebsiteSnapshot is deprecated - use extraction library's site_url filtering instead
#[allow(deprecated)]
pub use crate::domains::crawling::models::{WebsiteSnapshot, WebsiteSnapshotId};
