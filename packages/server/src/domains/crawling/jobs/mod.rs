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
//! Jobs chain via Seesaw events, enabling independent retries.

mod crawl_website;
mod executor;
mod extract_posts;
mod regenerate_posts;
mod sync_posts;

pub use crawl_website::CrawlWebsiteJob;
pub use executor::{
    execute_crawl_website_job, execute_extract_posts_job, execute_regenerate_posts_job,
    execute_sync_posts_job, JobExecutionResult, JobInfo,
};
pub use extract_posts::ExtractPostsJob;
pub use regenerate_posts::RegeneratePostsJob;
pub use sync_posts::SyncPostsJob;
