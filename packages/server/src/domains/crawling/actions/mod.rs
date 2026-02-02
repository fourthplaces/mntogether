//! Crawling domain actions
//!
//! Business logic extracted from effects for reusability and testability.

pub mod authorization;
pub mod build_pages;
pub mod crawl_website;
pub mod extract_posts;
pub mod regenerate_page;
pub mod sync_posts;
pub mod website_context;

pub use authorization::check_crawl_authorization;
pub use build_pages::{
    build_pages_to_summarize, build_page_to_summarize_from_snapshot,
    fetch_single_page_context, SinglePageContext,
};
pub use crawl_website::{crawl_website_pages, get_crawl_priorities, store_crawled_pages};
pub use extract_posts::{extract_posts_from_pages, update_page_extraction_status, ExtractionResult};
pub use sync_posts::{llm_deduplicate_website_posts, sync_and_deduplicate_posts, SyncAndDedupResult};
pub use regenerate_page::{regenerate_posts_for_page, regenerate_summary_for_page};
pub use website_context::{fetch_approved_website, fetch_snapshots_as_crawled_pages};
