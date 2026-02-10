use serde::{Deserialize, Serialize};

use crate::domains::source::models::Source;

/// API representation of a source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceData {
    pub id: String,
    pub source_type: String,
    pub url: Option<String>,
    pub organization_id: Option<String>,
    pub status: String,
    pub active: bool,
    pub scrape_frequency_hours: i32,
    pub last_scraped_at: Option<String>,
    pub submitted_by: Option<String>,
    pub submitter_type: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Source> for SourceData {
    fn from(source: Source) -> Self {
        Self {
            id: source.id.to_string(),
            source_type: source.source_type,
            url: source.url,
            organization_id: source.organization_id.map(|id| id.to_string()),
            status: source.status,
            active: source.active,
            scrape_frequency_hours: source.scrape_frequency_hours,
            last_scraped_at: source.last_scraped_at.map(|dt| dt.to_rfc3339()),
            submitted_by: source.submitted_by.map(|id| id.to_string()),
            submitter_type: source.submitter_type,
            created_at: source.created_at.to_rfc3339(),
            updated_at: source.updated_at.to_rfc3339(),
        }
    }
}

/// Edge containing a source and its cursor (Relay spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceEdge {
    pub node: SourceData,
    pub cursor: String,
}

/// Connection type for paginated sources (Relay spec)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConnection {
    pub edges: Vec<SourceEdge>,
    pub page_info: crate::common::PageInfo,
    pub total_count: i32,
}
