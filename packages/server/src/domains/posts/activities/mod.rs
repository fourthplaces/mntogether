//! Posts domain activities - entry-point business logic
//!
//! Called from Restate virtual objects.
//! Activities are self-contained: they take raw input, handle ID parsing,
//! auth checks, and return final models/results.

pub mod backfill;
pub mod core;
pub mod create_post;
pub mod deduplication;
pub mod expire_scheduled_posts;
pub mod llm_sync;
pub mod post_discovery;
pub mod post_extraction;
pub mod post_operations;
pub mod post_sync_handler;
pub mod reports;
pub mod resource_link_creation;
pub mod resource_link_extraction;
pub mod resource_link_scraping;
pub mod revision_actions;
pub mod schedule;
pub mod scoring;
pub mod scraping;
pub mod search;
pub mod sync_operations;
pub mod sync_utils;
pub mod syncing;
pub mod tags;
pub mod upcoming_events;

// Re-export for convenience
pub use core::*;
pub use create_post::{create_extracted_post, tag_post_from_extracted};
pub use deduplication::{
    deduplicate_cross_source_all_orgs, deduplicate_posts, CrossSourceDedupResult,
    DeduplicationRunResult,
};
pub use llm_sync::{llm_sync_posts, LlmSyncResult};
pub use post_sync_handler::PostProposalHandler;
pub use reports::ReportCreated;
pub use reports::*;
pub use revision_actions::{
    approve_revision, count_pending_revisions, get_pending_revisions, get_revision_for_post,
    reject_revision,
};
pub use scoring::{score_post_by_id, score_post_relevance, RelevanceScore};
pub use scraping::{submit_resource_link, ResourceLinkSubmission};
pub use sync_operations::{
    apply_delete, apply_insert, apply_merge, apply_update, MergeArgs, SyncOpResult,
};
pub use tags::{add_post_tag, remove_post_tag, update_post_tags, TagInput};
