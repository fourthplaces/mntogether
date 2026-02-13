//! Crawling domain data types
//!
//! Simple, serializable types returned by crawling activities.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::impl_restate_serde;

/// Result of ingesting a website
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteIngested {
    pub website_id: Uuid,
    pub job_id: Uuid,
    pub pages_crawled: usize,
    pub pages_summarized: usize,
}

impl_restate_serde!(WebsiteIngested);

/// Result of extracting narratives from pages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativesExtracted {
    pub narratives_count: usize,
    pub page_urls: Vec<String>,
}

impl_restate_serde!(NarrativesExtracted);

/// Result of syncing posts to database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostsSynced {
    pub website_id: Uuid,
    pub posts_synced: usize,
}

impl_restate_serde!(PostsSynced);
