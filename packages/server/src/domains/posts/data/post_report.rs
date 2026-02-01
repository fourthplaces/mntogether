use crate::domains::posts::models::post_report::{
    PostReportRecord, PostReportWithDetails,
};
use crate::server::graphql::context::GraphQLContext;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostReport {
    pub id: String,
    pub post_id: String,
    pub reason: String,
    pub category: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution_notes: Option<String>,
    pub action_taken: Option<String>,
}

#[juniper::graphql_object(Context = GraphQLContext)]
impl PostReport {
    fn id(&self) -> &str {
        &self.id
    }

    fn post_id(&self) -> &str {
        &self.post_id
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
pub struct PostReportDetail {
    pub id: String,
    pub post_id: String,
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
impl PostReportDetail {
    fn id(&self) -> &str {
        &self.id
    }

    fn post_id(&self) -> &str {
        &self.post_id
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

impl From<PostReportRecord> for PostReport {
    fn from(record: PostReportRecord) -> Self {
        Self {
            id: record.id.to_string(),
            post_id: record.post_id.to_string(),
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

impl From<PostReportWithDetails> for PostReportDetail {
    fn from(record: PostReportWithDetails) -> Self {
        Self {
            id: record.id.to_string(),
            post_id: record.post_id.to_string(),
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
