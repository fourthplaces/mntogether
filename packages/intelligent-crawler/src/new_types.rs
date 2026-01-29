use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;
use uuid::Uuid;

// Re-export enums from events (single source of truth)
pub use crate::events::{FlagSource, DiscoverySource, ScrapeStatus};

// ============================================================================
// ENUMS (type-safe states)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryStatus {
    Pending,
    Discovering,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagStatus {
    Pending,
    Flagged,
    Unflagged,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitDecision {
    Allow,
    DenyHourlyLimit,
    DenyConcurrency,
}

// ============================================================================
// CORE TYPES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub url: Url,
    pub domain: String,
    pub submitted_by: Option<String>,
    pub discovery_version: i32,
    pub discovery_status: DiscoveryStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPage {
    pub url: Url,
    pub domain: String,
    pub markdown: String,
    pub html: Option<String>, // Only stored if flagged
    pub content_hash: String,
    pub flag_status: FlagStatus,
    pub flagged_by: Option<FlagSource>,
    pub flag_confidence: Option<f32>,
    pub flag_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PageContent {
    pub url: Url,
    pub html: Option<String>, // Optional (some fetchers don't keep HTML)
    pub markdown: String,
    pub content_hash: String,
}

impl PageContent {
    /// Calculate content hash from markdown
    pub fn calculate_hash(markdown: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(markdown.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Create PageContent from markdown
    pub fn from_markdown(url: Url, markdown: String) -> Self {
        let content_hash = Self::calculate_hash(&markdown);
        Self {
            url,
            html: None,
            markdown,
            content_hash,
        }
    }

    /// Create PageContent from HTML and markdown
    pub fn from_html_and_markdown(url: Url, html: String, markdown: String) -> Self {
        let content_hash = Self::calculate_hash(&markdown);
        Self {
            url,
            html: Some(html),
            markdown,
            content_hash,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagDecision {
    pub should_flag: bool,
    pub confidence: f32,
    pub reason: String,
    pub source: FlagSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagResult {
    pub status: FlagStatus,
    pub source: Option<FlagSource>,
    pub confidence: Option<f32>,
    pub reason: Option<String>,
}

/// Opaque extraction record (crawler doesn't interpret schema)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawExtraction {
    pub extraction_run_id: Uuid,  // Link to extraction run
    pub page_id: Uuid,              // Which page it came from
    pub page_url: Url,
    pub data: serde_json::Value,    // Opaque JSON - adapter interprets
    pub confidence: f32,
    pub fingerprint_hint: Option<String>, // Optional hint for adapter
}

#[derive(Debug, Clone)]
pub struct ResourcePageEdge<ResourceId, PageId> {
    pub resource_id: ResourceId,
    pub page_id: PageId,
    pub crawl_session_id: Uuid,
    pub crawl_depth: i32,
    pub discovered_via: DiscoverySource,
}

#[derive(Debug, Clone)]
pub struct ExtractionRun<PageId> {
    pub page_id: PageId,
    pub page_content_hash: String,
    pub extractor_version: String,
    pub prompt_version: String,
    pub model: String,
}

// ============================================================================
// CONFIG & RESULTS
// ============================================================================

#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    pub max_depth: usize,
    pub same_domain_only: bool,
}

#[derive(Debug, Clone)]
pub struct DiscoveryResult<PageId> {
    pub pages_discovered: Vec<PageId>,
    pub pages_flagged: Vec<PageId>,
}

#[derive(Debug, Clone)]
pub struct ExtractionResult<ExtractionId> {
    pub run_id: ExtractionId,
    pub extractions: Vec<ExtractionId>,
}

#[derive(Debug, Clone)]
pub struct RefreshStats {
    pub pages_checked: usize,
    pub pages_changed: usize,
    pub extractions_triggered: usize,
}

#[derive(Debug, Clone)]
pub struct ExtractionStats {
    pub items_found: usize,
    pub items_created: usize,
    pub items_updated: usize,
}

#[derive(Debug, Clone)]
pub struct UpsertResult<PageId> {
    pub page_id: PageId,
    pub was_inserted: bool,
}

#[derive(Debug, Clone)]
pub struct PageToRefresh<PageId> {
    pub page_id: PageId,
    pub url: Url,
    pub content_hash: String,
    pub base_interval_hours: i32,
    pub jitter_hours: i32,
}
