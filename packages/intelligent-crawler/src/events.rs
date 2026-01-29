use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

/// Aggregate identity for event routing
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AggregateKey {
    Resource(Uuid),
    Page(Uuid),
    Extraction(Uuid),
}

/// Source of a flag decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagSource {
    Ai,
    Rule,
    Manual,
}

/// Source of page discovery
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoverySource {
    DirectSubmission,
    Crawl,
}

/// Scrape operation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrapeStatus {
    Ok,
    Failed,
    Blocked,
}

/// Events produced by effect handlers (facts about what happened)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CrawlerEvent {
    // ============================================================================
    // Resource events
    // ============================================================================
    ResourceSubmitted {
        resource_id: Uuid,
        url: Url,
        submitted_by: Option<String>,
    },

    DiscoveryStarted {
        resource_id: Uuid,
        crawl_session_id: Uuid,
    },

    DiscoveryCompleted {
        resource_id: Uuid,
        crawl_session_id: Uuid,
    },

    DiscoveryFailed {
        resource_id: Uuid,
        crawl_session_id: Uuid,
        error: String,
    },

    RediscoveryScheduled {
        resource_id: Uuid,
        scheduled_for: DateTime<Utc>,
    },

    // ============================================================================
    // Page events
    // ============================================================================
    PageDiscovered {
        resource_id: Uuid,
        page_id: Uuid,
        url: Url,
        crawl_session_id: Uuid,
        crawl_depth: i32,
        discovered_via: DiscoverySource,
    },

    PageFlagged {
        page_id: Uuid,
        url: Url,
        flagged_by: FlagSource,
        confidence: f32,
        reason: String,
    },

    PageUnflagged {
        page_id: Uuid,
        url: Url,
        reason: String,
    },

    PageFlaggingFailed {
        page_id: Uuid,
        error: String,
    },

    RefreshScheduled {
        page_id: Uuid,
        scheduled_for: DateTime<Utc>,
    },

    PageContentChanged {
        page_id: Uuid,
        old_content_hash: String,
        new_content_hash: String,
        scrape_status: ScrapeStatus,
    },

    PageContentUnchanged {
        page_id: Uuid,
        content_hash: String,
    },

    // ============================================================================
    // Extraction events
    // ============================================================================
    ExtractionStarted {
        page_id: Uuid,
        extraction_run_id: Uuid,
    },

    DataExtracted {
        page_id: Uuid,
        extraction_run_id: Uuid,
        extraction_id: Uuid,
        data: serde_json::Value,
        confidence: f32,
        fingerprint_hint: Option<String>,
    },

    ExtractionCompleted {
        page_id: Uuid,
        extraction_run_id: Uuid,
        items_found: usize,
    },

    ExtractionFailed {
        page_id: Uuid,
        extraction_run_id: Uuid,
        error: String,
    },

    // ============================================================================
    // Flywheel events
    // ============================================================================
    RefreshCompleted {
        pages_checked: usize,
        pages_changed: usize,
    },
}

impl CrawlerEvent {
    /// Get the aggregate this event belongs to (for routing to state machines)
    pub fn aggregate_key(&self) -> AggregateKey {
        match self {
            // Resource events
            CrawlerEvent::ResourceSubmitted { resource_id, .. } =>
                AggregateKey::Resource(*resource_id),
            CrawlerEvent::DiscoveryStarted { resource_id, .. } =>
                AggregateKey::Resource(*resource_id),
            CrawlerEvent::DiscoveryCompleted { resource_id, .. } =>
                AggregateKey::Resource(*resource_id),
            CrawlerEvent::DiscoveryFailed { resource_id, .. } =>
                AggregateKey::Resource(*resource_id),
            CrawlerEvent::RediscoveryScheduled { resource_id, .. } =>
                AggregateKey::Resource(*resource_id),

            // Page events
            CrawlerEvent::PageDiscovered { page_id, .. } =>
                AggregateKey::Page(*page_id),
            CrawlerEvent::PageFlagged { page_id, .. } =>
                AggregateKey::Page(*page_id),
            CrawlerEvent::PageUnflagged { page_id, .. } =>
                AggregateKey::Page(*page_id),
            CrawlerEvent::PageFlaggingFailed { page_id, .. } =>
                AggregateKey::Page(*page_id),
            CrawlerEvent::RefreshScheduled { page_id, .. } =>
                AggregateKey::Page(*page_id),
            CrawlerEvent::PageContentChanged { page_id, .. } =>
                AggregateKey::Page(*page_id),
            CrawlerEvent::PageContentUnchanged { page_id, .. } =>
                AggregateKey::Page(*page_id),
            CrawlerEvent::ExtractionStarted { page_id, .. } =>
                AggregateKey::Page(*page_id),
            CrawlerEvent::ExtractionCompleted { page_id, .. } =>
                AggregateKey::Page(*page_id),
            CrawlerEvent::ExtractionFailed { page_id, .. } =>
                AggregateKey::Page(*page_id),

            // Extraction events
            CrawlerEvent::DataExtracted { extraction_id, .. } =>
                AggregateKey::Extraction(*extraction_id),

            // Global events (no specific aggregate)
            CrawlerEvent::RefreshCompleted { .. } =>
                AggregateKey::Resource(Uuid::nil()), // Special case: global event
        }
    }
}
