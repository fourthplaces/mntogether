//! RegeneratePostsJob - Background job for regenerating posts from existing snapshots.
//!
//! This replaces the synchronous `regenerate_posts` action with a background job.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::kernel::jobs::{CommandMeta, JobPriority};

/// Job to regenerate posts from existing page snapshots.
///
/// This re-runs extraction on already-crawled pages without re-fetching them.
/// Useful after improving extraction logic or fixing bugs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegeneratePostsJob {
    /// The website to regenerate posts for
    pub website_id: Uuid,
    /// The user requesting the regeneration (for auth check)
    pub visitor_id: Uuid,
}

impl RegeneratePostsJob {
    /// The job type identifier used in the jobs table.
    pub const JOB_TYPE: &'static str = "regenerate_posts";

    /// Create a new regenerate posts job.
    pub fn new(website_id: Uuid, visitor_id: Uuid) -> Self {
        Self {
            website_id,
            visitor_id,
        }
    }
}

impl CommandMeta for RegeneratePostsJob {
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
        let visitor_id = Uuid::new_v4();
        let job = RegeneratePostsJob::new(website_id, visitor_id);

        assert_eq!(job.website_id, website_id);
        assert_eq!(job.visitor_id, visitor_id);
    }

    #[test]
    fn test_command_meta() {
        let job = RegeneratePostsJob::new(Uuid::new_v4(), Uuid::new_v4());

        assert_eq!(job.command_type(), "regenerate_posts");
        assert_eq!(job.reference_id(), Some(job.website_id));
        assert_eq!(job.max_retries(), 3);
    }
}
