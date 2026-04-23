//! Posts domain activities - entry-point business logic
//!
//! Called from HTTP handlers.
//! Activities are self-contained: they take raw input, handle ID parsing,
//! auth checks, and return final models/results.

pub mod backfill;
pub mod content_hash_dedup;
pub mod contacts;
pub mod core;
pub mod create_post;
pub mod expire_scheduled_posts;
pub mod individual_dedup;
pub mod ingest_post;
pub mod organization_dedup;
pub mod post_operations;
pub mod reports;
pub mod revision_actions;
pub mod revision_reflow;
pub mod schedule;
pub mod tag_resolution;
pub mod tags;
pub mod upcoming_events;

// Re-export for convenience
pub use contacts::{add_post_contact, remove_post_contact};
pub use core::*;
pub use create_post::{create_extracted_post, tag_post_from_extracted};
pub use reports::ReportCreated;
pub use reports::*;
pub use revision_actions::{
    approve_revision, count_pending_revisions, get_pending_revisions, get_revision_for_post,
    reject_revision,
};
pub use tags::{add_post_tag, remove_post_tag, update_post_tags, TagInput};
