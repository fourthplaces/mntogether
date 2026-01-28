use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Organization source - a website we monitor for volunteer needs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationSource {
    pub id: Uuid,
    pub organization_name: String,
    pub source_url: String,
    pub last_scraped_at: Option<DateTime<Utc>>,
    pub scrape_frequency_hours: i32,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}
