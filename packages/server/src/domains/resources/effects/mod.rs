//! Resource domain effects (side effects and business logic)

pub mod deduplication;
pub mod handlers;
pub mod sync;

pub use deduplication::{deduplicate_resource, DedupAction};
pub use handlers::resource_effect;
pub use sync::{sync_resources, ExtractedResourceInput, SyncResult};
