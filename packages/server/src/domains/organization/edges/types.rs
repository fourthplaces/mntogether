use crate::domains::organization::models::{
    source::OrganizationSource, NeedStatus, OrganizationNeed,
};
use chrono::{DateTime, Utc};
use juniper::{GraphQLEnum, GraphQLInputObject, GraphQLObject};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// GraphQL type for organization need
#[derive(Debug, Clone, GraphQLObject)]
#[graphql(description = "A volunteer opportunity from an organization")]
pub struct Need {
    pub id: Uuid,
    pub organization_name: String,
    pub title: String,
    pub tldr: Option<String>,
    pub description: String,
    pub description_markdown: Option<String>,
    pub contact_info: Option<ContactInfo>,
    pub urgency: Option<String>,
    pub status: NeedStatusData,
    pub location: Option<String>,
    pub submission_type: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<OrganizationNeed> for Need {
    fn from(need: OrganizationNeed) -> Self {
        let contact_info = need
            .contact_info
            .and_then(|json| serde_json::from_value(json).ok());

        Self {
            id: need.id.into_uuid(),
            organization_name: need.organization_name,
            title: need.title,
            tldr: need.tldr,
            description: need.description,
            description_markdown: need.description_markdown,
            contact_info,
            urgency: need.urgency,
            status: match need.status.as_str() {
                "pending_approval" => NeedStatusData::PendingApproval,
                "active" => NeedStatusData::Active,
                "rejected" => NeedStatusData::Rejected,
                "expired" => NeedStatusData::Expired,
                _ => NeedStatusData::PendingApproval, // default fallback
            },
            location: need.location,
            submission_type: need.submission_type,
            created_at: need.created_at,
        }
    }
}

/// Contact information
#[derive(Debug, Clone, GraphQLObject, Serialize, Deserialize)]
pub struct ContactInfo {
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
}

/// Need status for GraphQL
#[derive(Debug, Clone, Copy, GraphQLEnum)]
pub enum NeedStatusData {
    PendingApproval,
    Active,
    Rejected,
    Expired,
}

impl From<NeedStatus> for NeedStatusData {
    fn from(status: NeedStatus) -> Self {
        match status {
            NeedStatus::PendingApproval => Self::PendingApproval,
            NeedStatus::Active => Self::Active,
            NeedStatus::Rejected => Self::Rejected,
            NeedStatus::Expired => Self::Expired,
        }
    }
}

/// Input for editing a need before approval
#[derive(Debug, Clone, GraphQLInputObject)]
pub struct EditNeedInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub description_markdown: Option<String>,
    pub tldr: Option<String>,
    pub contact_info: Option<ContactInfoInput>,
    pub urgency: Option<String>,
    pub location: Option<String>,
}

/// Input for user-submitted needs
#[derive(Debug, Clone, GraphQLInputObject)]
pub struct SubmitNeedInput {
    pub organization_name: String,
    pub title: String,
    pub description: String,
    pub contact_info: Option<ContactInfoInput>,
    pub urgency: Option<String>,
    pub location: Option<String>,
}

#[derive(Debug, Clone, GraphQLInputObject, Serialize, Deserialize)]
pub struct ContactInfoInput {
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
}

/// Result of scraping an organization source
#[derive(Debug, Clone, GraphQLObject)]
pub struct ScrapeResult {
    pub source_id: Uuid,
    pub new_needs_count: i32,
    pub changed_needs_count: i32,
    pub disappeared_needs_count: i32,
}

/// Result of starting an async scrape job
#[derive(Debug, Clone, GraphQLObject)]
pub struct ScrapeJobResult {
    pub job_id: Uuid,
    pub source_id: Uuid,
    pub status: String,
}

/// Connection type for paginated needs
#[derive(Debug, Clone, GraphQLObject)]
pub struct NeedConnection {
    pub nodes: Vec<Need>,
    pub total_count: i32,
    pub has_next_page: bool,
}

/// GraphQL type for organization source (website to scrape)
#[derive(Debug, Clone, GraphQLObject)]
#[graphql(description = "A website source for scraping volunteer opportunities")]
pub struct OrganizationSourceData {
    pub id: Uuid,
    pub organization_name: String,
    pub source_url: String,
    pub last_scraped_at: Option<DateTime<Utc>>,
    pub scrape_frequency_hours: i32,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<OrganizationSource> for OrganizationSourceData {
    fn from(source: OrganizationSource) -> Self {
        Self {
            id: source.id.into_uuid(),
            organization_name: source.organization_name,
            source_url: source.source_url,
            last_scraped_at: source.last_scraped_at,
            scrape_frequency_hours: source.scrape_frequency_hours,
            active: source.active,
            created_at: source.created_at,
        }
    }
}

/// Input for submitting a resource link from the public
#[derive(Debug, Clone, GraphQLInputObject)]
pub struct SubmitResourceLinkInput {
    pub url: String,
    pub context: Option<String>,
    pub submitter_contact: Option<String>,
}

/// Result of submitting a resource link
#[derive(Debug, Clone, GraphQLObject)]
pub struct SubmitResourceLinkResult {
    pub job_id: Uuid,
    pub status: String,
    pub message: String,
}
