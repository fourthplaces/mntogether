//! CrawlWebsiteJob - Background job for website crawling.
//!
//! This replaces the synchronous `ingest_website` action with a background job.
//! GraphQL mutations enqueue this job and return immediately with a job_id.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::kernel::jobs::{CommandMeta, JobPriority};

/// Job to crawl a website and extract posts.
///
/// # Usage
///
/// ```ignore
/// let job = CrawlWebsiteJob::new(website_id, visitor_id, true);
/// let result = job_queue.enqueue(job).await?;
/// // Returns immediately with job_id
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlWebsiteJob {
    /// The website to crawl
    pub website_id: Uuid,
    /// The user requesting the crawl (for auth check)
    pub visitor_id: Uuid,
    /// Whether to use Firecrawl (true) or basic HTTP (false)
    pub use_firecrawl: bool,
}

impl CrawlWebsiteJob {
    /// The job type identifier used in the jobs table.
    pub const JOB_TYPE: &'static str = "crawl_website";

    /// Create a new crawl website job.
    pub fn new(website_id: Uuid, visitor_id: Uuid, use_firecrawl: bool) -> Self {
        Self {
            website_id,
            visitor_id,
            use_firecrawl,
        }
    }
}

impl CommandMeta for CrawlWebsiteJob {
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
        let job = CrawlWebsiteJob::new(website_id, visitor_id, true);

        assert_eq!(job.website_id, website_id);
        assert_eq!(job.visitor_id, visitor_id);
        assert!(job.use_firecrawl);
    }

    #[test]
    fn test_command_meta() {
        let job = CrawlWebsiteJob::new(Uuid::new_v4(), Uuid::new_v4(), false);

        assert_eq!(job.command_type(), "crawl_website");
        assert_eq!(job.reference_id(), Some(job.website_id));
        assert_eq!(job.max_retries(), 3);
    }

    #[test]
    fn test_serialization() {
        let job = CrawlWebsiteJob::new(Uuid::new_v4(), Uuid::new_v4(), true);
        let json = serde_json::to_string(&job).unwrap();
        let deserialized: CrawlWebsiteJob = serde_json::from_str(&json).unwrap();

        assert_eq!(job.website_id, deserialized.website_id);
        assert_eq!(job.visitor_id, deserialized.visitor_id);
        assert_eq!(job.use_firecrawl, deserialized.use_firecrawl);
    }
}
