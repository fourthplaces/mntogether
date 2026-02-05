//! Background jobs for the crawling domain.
//!
//! These jobs provide:
//! - Job tracking via the jobs table
//! - Status queries for UI display
//! - Error message storage for debugging
//!
//! The crawl pipeline is split into independent, retriable jobs:
//! 1. CrawlWebsiteJob - Ingest website pages
//! 2. ExtractPostsJob - Run three-pass extraction on ingested pages
//! 3. SyncPostsJob - Write extracted posts to database
//!
//! Job handlers are in effects/job_handlers.rs and are registered with the JobRunner.

mod crawl_website;
mod extract_posts;
mod job_info;
mod regenerate_posts;
mod sync_posts;

pub use crawl_website::CrawlWebsiteJob;
pub use extract_posts::ExtractPostsJob;
pub use job_info::JobInfo;
pub use regenerate_posts::RegeneratePostsJob;
pub use sync_posts::SyncPostsJob;
