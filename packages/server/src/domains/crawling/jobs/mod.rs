//! Background jobs for the crawling domain.
//!
//! These jobs provide:
//! - Job tracking via the jobs table
//! - Status queries for UI display
//! - Error message storage for debugging
//!
//! Currently jobs run synchronously with tracking. True async execution
//! can be added later when the JobWorker infrastructure is complete.

mod crawl_website;
mod executor;
mod regenerate_posts;

pub use crawl_website::CrawlWebsiteJob;
pub use executor::{
    execute_crawl_website_job, execute_regenerate_posts_job, JobExecutionResult, JobInfo,
};
pub use regenerate_posts::RegeneratePostsJob;
