//! SyncPostsJob - Background job for syncing extracted posts to the database.
//!
//! This job takes extracted posts and writes them to the database using either:
//! - Simple delete-and-replace sync (fast, destructive)
//! - LLM-based intelligent diff sync (smart, preserves data)

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::ExtractedPost;
use crate::kernel::jobs::{CommandMeta, JobPriority};

/// Job to sync extracted posts to the database.
///
/// Triggered by `PostsExtractedFromPages` event. Writes posts to the database
/// and emits `PostsSynced` on completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPostsJob {
    /// The website these posts belong to
    pub website_id: Uuid,
    /// The extracted posts to sync (JSONB serialized)
    pub extracted_posts: Vec<ExtractedPost>,
    /// Whether to use LLM-based intelligent sync (true) or simple delete-and-replace (false)
    pub use_llm_sync: bool,
    /// Parent job ID for tracking job chains
    pub parent_job_id: Option<Uuid>,
}

impl SyncPostsJob {
    /// The job type identifier used in the jobs table.
    pub const JOB_TYPE: &'static str = "sync_posts";

    /// Create a new sync posts job with simple sync.
    pub fn new(website_id: Uuid, extracted_posts: Vec<ExtractedPost>) -> Self {
        Self {
            website_id,
            extracted_posts,
            use_llm_sync: false,
            parent_job_id: None,
        }
    }

    /// Create a new sync posts job with LLM sync.
    pub fn with_llm_sync(website_id: Uuid, extracted_posts: Vec<ExtractedPost>) -> Self {
        Self {
            website_id,
            extracted_posts,
            use_llm_sync: true,
            parent_job_id: None,
        }
    }

    /// Set the parent job ID for job chain tracking.
    pub fn with_parent(mut self, parent_job_id: Uuid) -> Self {
        self.parent_job_id = Some(parent_job_id);
        self
    }
}

impl CommandMeta for SyncPostsJob {
    fn command_type(&self) -> &'static str {
        Self::JOB_TYPE
    }

    fn reference_id(&self) -> Option<Uuid> {
        Some(self.website_id)
    }

    fn max_retries(&self) -> i32 {
        3
    }

    fn priority(&self) -> JobPriority {
        JobPriority::Normal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_post() -> ExtractedPost {
        ExtractedPost {
            title: "Test Post".to_string(),
            tldr: "A test post".to_string(),
            description: "This is a test post".to_string(),
            contact: None,
            location: None,
            urgency: None,
            confidence: None,
            audience_roles: vec!["recipient".to_string()],
            source_page_snapshot_id: None,
        }
    }

    #[test]
    fn test_job_creation() {
        let website_id = Uuid::new_v4();
        let posts = vec![sample_post()];
        let job = SyncPostsJob::new(website_id, posts.clone());

        assert_eq!(job.website_id, website_id);
        assert_eq!(job.extracted_posts.len(), 1);
        assert!(!job.use_llm_sync);
        assert!(job.parent_job_id.is_none());
    }

    #[test]
    fn test_job_with_llm_sync() {
        let website_id = Uuid::new_v4();
        let posts = vec![sample_post()];
        let job = SyncPostsJob::with_llm_sync(website_id, posts);

        assert!(job.use_llm_sync);
    }

    #[test]
    fn test_job_with_parent() {
        let website_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        let posts = vec![sample_post()];
        let job = SyncPostsJob::new(website_id, posts).with_parent(parent_id);

        assert_eq!(job.parent_job_id, Some(parent_id));
    }

    #[test]
    fn test_command_meta() {
        let job = SyncPostsJob::new(Uuid::new_v4(), vec![]);

        assert_eq!(job.command_type(), "sync_posts");
        assert_eq!(job.reference_id(), Some(job.website_id));
        assert_eq!(job.max_retries(), 3);
    }

    #[test]
    fn test_serialization() {
        let job = SyncPostsJob::with_llm_sync(Uuid::new_v4(), vec![sample_post()])
            .with_parent(Uuid::new_v4());
        let json = serde_json::to_string(&job).unwrap();
        let deserialized: SyncPostsJob = serde_json::from_str(&json).unwrap();

        assert_eq!(job.website_id, deserialized.website_id);
        assert_eq!(job.use_llm_sync, deserialized.use_llm_sync);
        assert_eq!(job.parent_job_id, deserialized.parent_job_id);
        assert_eq!(job.extracted_posts.len(), deserialized.extracted_posts.len());
    }
}
