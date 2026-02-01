//! Resource domain effects (side effects and business logic)

pub mod deduplication;
pub mod sync;

pub use deduplication::{DedupAction, deduplicate_resource};
pub use sync::{ExtractedResourceInput, sync_resources, SyncResult};
