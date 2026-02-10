use crate::domains::social_profile::models::SocialProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialProfileData {
    pub id: String,
    pub organization_id: String,
    pub platform: String,
    pub handle: String,
    pub url: Option<String>,
    pub scrape_frequency_hours: i32,
    pub last_scraped_at: Option<String>,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<SocialProfile> for SocialProfileData {
    fn from(sp: SocialProfile) -> Self {
        Self {
            id: sp.id.to_string(),
            organization_id: sp.organization_id.to_string(),
            platform: sp.platform,
            handle: sp.handle,
            url: sp.url,
            scrape_frequency_hours: sp.scrape_frequency_hours,
            last_scraped_at: sp.last_scraped_at.map(|dt| dt.to_rfc3339()),
            active: sp.active,
            created_at: sp.created_at.to_rfc3339(),
            updated_at: sp.updated_at.to_rfc3339(),
        }
    }
}
