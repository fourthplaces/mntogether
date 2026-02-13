use crate::domains::posts::models::post_report::{PostReportRecord, PostReportWithDetails};
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
    pub post_title: String,
    pub post_type: String,
    pub post_status: String,
    pub report_count_for_post: i64,
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
            post_title: record.post_title,
            post_type: record.post_type,
            post_status: record.post_status,
            report_count_for_post: record.report_count_for_post,
        }
    }
}
