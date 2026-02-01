//! Resource domain models

pub mod resource;
pub mod resource_source;
pub mod resource_tag;
pub mod resource_version;

pub use resource::{Resource, ResourceStatus};
pub use resource_source::ResourceSource;
pub use resource_tag::ResourceTag;
pub use resource_version::{ChangeReason, DedupDecision, ResourceVersion};
