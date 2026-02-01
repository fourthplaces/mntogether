pub mod organization;

pub use organization::{OrganizationData, TagData};

// Re-export website data from the website domain for backward compatibility
pub use crate::domains::website::data::{WebsiteData, WebsiteSnapshotData};
