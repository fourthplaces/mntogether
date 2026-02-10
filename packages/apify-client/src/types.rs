use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Input for the apify/instagram-post-scraper actor.
#[derive(Debug, Clone, Serialize)]
pub struct InstagramScraperInput {
    pub username: Vec<String>,
    #[serde(rename = "resultsLimit")]
    pub results_limit: u32,
}

/// A single Instagram post from the Apify dataset.
#[derive(Debug, Clone, Deserialize)]
pub struct InstagramPost {
    pub caption: Option<String>,
    #[serde(rename = "ownerUsername")]
    pub owner_username: Option<String>,
    #[serde(rename = "ownerFullName")]
    pub owner_full_name: Option<String>,
    pub url: String,
    #[serde(rename = "shortCode")]
    pub short_code: Option<String>,
    #[serde(rename = "displayUrl")]
    pub display_url: Option<String>,
    #[serde(rename = "likesCount")]
    pub likes_count: Option<i64>,
    #[serde(rename = "commentsCount")]
    pub comments_count: Option<i64>,
    pub timestamp: Option<DateTime<Utc>>,
    #[serde(rename = "type")]
    pub post_type: Option<String>,
    pub mentions: Option<Vec<String>>,
    #[serde(rename = "locationName")]
    pub location_name: Option<String>,
}

/// Wrapper for Apify API responses.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
}

/// Apify actor run metadata.
#[derive(Debug, Clone, Deserialize)]
pub struct RunData {
    pub id: String,
    pub status: String,
    #[serde(rename = "defaultDatasetId")]
    pub default_dataset_id: String,
    #[serde(rename = "startedAt")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(rename = "finishedAt")]
    pub finished_at: Option<DateTime<Utc>>,
}
