use crate::domains::listings::models::listing_report::{
    ListingReportRecord, ListingReportWithDetails,
};
use crate::server::graphql::context::GraphQLContext;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingReport {
    pub id: String,
    pub listing_id: String,
    pub reason: String,
    pub category: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution_notes: Option<String>,
    pub action_taken: Option<String>,
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl ListingReport {
    fn id(&self) -> &str {
        &self.id
    }

    fn listing_id(&self) -> &str {
        &self.listing_id
    }

    fn reason(&self) -> &str {
        &self.reason
    }

    fn category(&self) -> &str {
        &self.category
    }

    fn status(&self) -> &str {
        &self.status
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn resolved_at(&self) -> Option<DateTime<Utc>> {
        self.resolved_at
    }

    fn resolution_notes(&self) -> Option<&str> {
        self.resolution_notes.as_deref()
    }

    fn action_taken(&self) -> Option<&str> {
        self.action_taken.as_deref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingReportDetail {
    pub id: String,
    pub listing_id: String,
    pub reason: String,
    pub category: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution_notes: Option<String>,
    pub action_taken: Option<String>,
    pub listing_title: String,
    pub organization_name: String,
    pub listing_type: String,
    pub listing_status: String,
    pub report_count_for_listing: i64,
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl ListingReportDetail {
    fn id(&self) -> &str {
        &self.id
    }

    fn listing_id(&self) -> &str {
        &self.listing_id
    }

    fn reason(&self) -> &str {
        &self.reason
    }

    fn category(&self) -> &str {
        &self.category
    }

    fn status(&self) -> &str {
        &self.status
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn resolved_at(&self) -> Option<DateTime<Utc>> {
        self.resolved_at
    }

    fn resolution_notes(&self) -> Option<&str> {
        self.resolution_notes.as_deref()
    }

    fn action_taken(&self) -> Option<&str> {
        self.action_taken.as_deref()
    }

    fn listing_title(&self) -> &str {
        &self.listing_title
    }

    fn organization_name(&self) -> &str {
        &self.organization_name
    }

    fn listing_type(&self) -> &str {
        &self.listing_type
    }

    fn listing_status(&self) -> &str {
        &self.listing_status
    }

    fn report_count_for_listing(&self) -> i32 {
        self.report_count_for_listing as i32
    }
}

impl From<ListingReportRecord> for ListingReport {
    fn from(record: ListingReportRecord) -> Self {
        Self {
            id: record.id.to_string(),
            listing_id: record.listing_id.to_string(),
            reason: record.reason,
            category: record.category,
            status: record.status,
            created_at: record.created_at,
            resolved_at: record.resolved_at,
            resolution_notes: record.resolution_notes,
            action_taken: record.action_taken,
        }
    }
}

impl From<ListingReportWithDetails> for ListingReportDetail {
    fn from(record: ListingReportWithDetails) -> Self {
        Self {
            id: record.id.to_string(),
            listing_id: record.listing_id.to_string(),
            reason: record.reason,
            category: record.category,
            status: record.status,
            created_at: record.created_at,
            resolved_at: record.resolved_at,
            resolution_notes: record.resolution_notes,
            action_taken: record.action_taken,
            listing_title: record.listing_title,
            organization_name: record.organization_name,
            listing_type: record.listing_type,
            listing_status: record.listing_status,
            report_count_for_listing: record.report_count_for_listing,
        }
    }
}
