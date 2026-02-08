//! Website data types.

use crate::domains::website::models::Website;
use serde::{Deserialize, Serialize};

/// API representation of a website (for scraping/monitoring)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteData {
    pub id: String,
    pub domain: String,
    pub last_scraped_at: Option<String>,
    pub scrape_frequency_hours: i32,
    pub active: bool,
    pub status: String,
    pub submitted_by: Option<String>,
    pub submitter_type: Option<String>,
    pub created_at: String,
    // Crawl tracking fields
    pub crawl_status: Option<String>,
    pub crawl_attempt_count: Option<i32>,
    pub max_crawl_retries: Option<i32>,
    pub last_crawl_started_at: Option<String>,
    pub last_crawl_completed_at: Option<String>,
    pub pages_crawled_count: Option<i32>,
    pub max_pages_per_crawl: Option<i32>,
}

impl From<Website> for WebsiteData {
    fn from(website: Website) -> Self {
        Self {
            id: website.id.to_string(),
            domain: website.domain,
            last_scraped_at: website.last_scraped_at.map(|dt| dt.to_rfc3339()),
            scrape_frequency_hours: website.scrape_frequency_hours,
            active: website.active,
            status: website.status,
            submitted_by: website.submitted_by.map(|id| id.to_string()),
            submitter_type: website.submitter_type,
            created_at: website.created_at.to_rfc3339(),
            // Crawl tracking fields
            crawl_status: website.crawl_status,
            crawl_attempt_count: website.crawl_attempt_count,
            max_crawl_retries: website.max_crawl_retries,
            last_crawl_started_at: website.last_crawl_started_at.map(|dt| dt.to_rfc3339()),
            last_crawl_completed_at: website.last_crawl_completed_at.map(|dt| dt.to_rfc3339()),
            pages_crawled_count: website.pages_crawled_count,
            max_pages_per_crawl: website.max_pages_per_crawl,
        }
    }
}

// ============================================================================
// Relay Pagination Types
// ============================================================================

/// Edge containing a website and its cursor (Relay spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteEdge {
    pub node: WebsiteData,
    pub cursor: String,
}

/// Connection type for paginated websites (Relay spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteConnection {
    pub edges: Vec<WebsiteEdge>,
    pub page_info: crate::common::PageInfo,
    pub total_count: i32,
}
