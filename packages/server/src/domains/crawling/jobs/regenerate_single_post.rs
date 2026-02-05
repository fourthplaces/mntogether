//! RegenerateSinglePostJob - Background job for regenerating a single post.
//!
//! Re-runs extraction on the post's source page(s) and updates it with fresh data.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::kernel::jobs::{CommandMeta, JobPriority};

/// Job to regenerate a single post from its source extraction pages.
///
/// This re-runs the three-pass extraction on the post's source URL content
/// and updates the post with the best matching result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegenerateSinglePostJob {
    /// The post to regenerate
    pub post_id: Uuid,
    /// The user requesting the regeneration (for auth check)
    pub visitor_id: Uuid,
}

impl RegenerateSinglePostJob {
    /// The job type identifier used in the jobs table.
    pub const JOB_TYPE: &'static str = "regenerate_single_post";

    /// Create a new regenerate single post job.
    pub fn new(post_id: Uuid, visitor_id: Uuid) -> Self {
        Self {
            post_id,
            visitor_id,
        }
    }
}

impl CommandMeta for RegenerateSinglePostJob {
    fn command_type(&self) -> &'static str {
        Self::JOB_TYPE
    }

    fn reference_id(&self) -> Option<Uuid> {
        Some(self.post_id)
    }

    fn max_retries(&self) -> i32 {
        3
    }

    fn priority(&self) -> JobPriority {
        JobPriority::Normal
    }
}
