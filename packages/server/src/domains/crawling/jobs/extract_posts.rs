//! ExtractPostsJob - Background job for extracting posts from ingested pages.
//!
//! This job runs the three-pass extraction pipeline:
//! 1. Batch narrative extraction (multiple LLM calls)
//! 2. Deduplicate & merge posts
//! 3. Agentic investigation to find contact info

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::kernel::jobs::{CommandMeta, JobPriority};

/// Job to extract posts from already-ingested website pages.
///
/// Triggered by `WebsiteIngested` event. Runs the three-pass extraction
/// and emits `PostsExtractedFromPages` on completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractPostsJob {
    /// The website to extract posts from
    pub website_id: Uuid,
    /// Parent job ID for tracking job chains
    pub parent_job_id: Option<Uuid>,
}

impl ExtractPostsJob {
    /// The job type identifier used in the jobs table.
    pub const JOB_TYPE: &'static str = "extract_posts";

    /// Create a new extract posts job.
    pub fn new(website_id: Uuid) -> Self {
        Self {
            website_id,
            parent_job_id: None,
        }
    }

    /// Create a new extract posts job with parent job reference.
    pub fn with_parent(website_id: Uuid, parent_job_id: Uuid) -> Self {
        Self {
            website_id,
            parent_job_id: Some(parent_job_id),
        }
    }
}

impl CommandMeta for ExtractPostsJob {
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

    #[test]
    fn test_job_creation() {
        let website_id = Uuid::new_v4();
        let job = ExtractPostsJob::new(website_id);

        assert_eq!(job.website_id, website_id);
        assert!(job.parent_job_id.is_none());
    }

    #[test]
    fn test_job_with_parent() {
        let website_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        let job = ExtractPostsJob::with_parent(website_id, parent_id);

        assert_eq!(job.website_id, website_id);
        assert_eq!(job.parent_job_id, Some(parent_id));
    }

    #[test]
    fn test_command_meta() {
        let job = ExtractPostsJob::new(Uuid::new_v4());

        assert_eq!(job.command_type(), "extract_posts");
        assert_eq!(job.reference_id(), Some(job.website_id));
        assert_eq!(job.max_retries(), 3);
    }

    #[test]
    fn test_serialization() {
        let job = ExtractPostsJob::with_parent(Uuid::new_v4(), Uuid::new_v4());
        let json = serde_json::to_string(&job).unwrap();
        let deserialized: ExtractPostsJob = serde_json::from_str(&json).unwrap();

        assert_eq!(job.website_id, deserialized.website_id);
        assert_eq!(job.parent_job_id, deserialized.parent_job_id);
    }
}
