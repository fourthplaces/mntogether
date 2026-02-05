//! Type definitions for GraphQL API responses
//!
//! These mirror the TypeScript types from web-next/lib/types.ts

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Common Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: Option<bool>,
    pub start_cursor: Option<String>,
    pub end_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResult<T> {
    pub nodes: Vec<T>,
    pub page_info: PageInfo,
    pub total_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: String,
    pub kind: String,
    pub value: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    pub email: Option<String>,
    pub phone: Option<String>,
    pub website: Option<String>,
}

// ============================================================================
// Post/Listing Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PostType {
    Service,
    Opportunity,
    Business,
    Professional,
}

impl PostType {
    pub fn label(&self) -> &'static str {
        match self {
            PostType::Service => "Service",
            PostType::Opportunity => "Opportunity",
            PostType::Business => "Business",
            PostType::Professional => "Professional",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            PostType::Service => "\u{1F3E5}",      // 🏥
            PostType::Opportunity => "\u{1F91D}", // 🤝
            PostType::Business => "\u{1F3EA}",    // 🏪
            PostType::Professional => "\u{1F464}", // 👤
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ListingStatus {
    PendingApproval,
    Active,
    Rejected,
    Expired,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapacityStatus {
    Accepting,
    Paused,
    AtCapacity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SubmissionType {
    Scraped,
    Manual,
    UserSubmitted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Urgency {
    Urgent,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Post {
    pub id: String,
    pub organization_name: String,
    pub title: String,
    pub tldr: Option<String>,
    pub description: String,
    pub description_markdown: Option<String>,
    pub post_type: Option<PostType>,
    pub category: Option<String>,
    pub capacity_status: Option<CapacityStatus>,
    pub urgency: Option<Urgency>,
    pub status: ListingStatus,
    pub location: Option<String>,
    pub source_url: Option<String>,
    pub submission_type: Option<SubmissionType>,
    pub tags: Option<Vec<Tag>>,
    pub created_at: String,
    pub updated_at: Option<String>,
}

// ============================================================================
// Organization Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessInfo {
    pub proceeds_percentage: Option<f64>,
    pub proceeds_beneficiary_id: Option<String>,
    pub donation_link: Option<String>,
    pub gift_card_link: Option<String>,
    pub online_store_url: Option<String>,
    pub is_cause_driven: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub summary: Option<String>,
    pub website: Option<String>,
    pub phone: Option<String>,
    pub primary_address: Option<String>,
    pub verified: Option<bool>,
    pub contact_info: Option<ContactInfo>,
    pub location: Option<String>,
    pub business_info: Option<BusinessInfo>,
    pub tags: Option<Vec<Tag>>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationMatch {
    pub organization: Organization,
    pub similarity_score: f64,
}

// ============================================================================
// Website Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebsiteStatus {
    PendingReview,
    Approved,
    Rejected,
    Suspended,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Website {
    pub id: String,
    pub url: Option<String>,
    pub domain: String,
    pub status: WebsiteStatus,
    pub submitted_by: Option<String>,
    pub submitter_type: Option<String>,
    pub last_scraped_at: Option<String>,
    pub snapshots_count: Option<i32>,
    pub listings_count: Option<i32>,
    pub listings: Option<Vec<Post>>,
    pub created_at: String,
}

// ============================================================================
// Chat Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatContainer {
    pub id: String,
    pub container_type: String,
    pub language: Option<String>,
    pub created_at: String,
    pub last_activity_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub container_id: String,
    pub role: String,
    pub content: String,
    pub author_id: Option<String>,
    pub moderation_status: Option<String>,
    pub parent_message_id: Option<String>,
    pub sequence_number: Option<i32>,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub edited_at: Option<String>,
}

// ============================================================================
// Resource Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResourceStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub id: String,
    pub website_id: Option<String>,
    pub title: String,
    pub content: String,
    pub location: Option<String>,
    pub status: ResourceStatus,
    pub organization_name: Option<String>,
    pub has_embedding: Option<bool>,
    pub source_urls: Option<Vec<String>>,
    pub tags: Option<Vec<Tag>>,
    pub created_at: String,
    pub updated_at: Option<String>,
}

// ============================================================================
// Job Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobResult {
    pub job_id: String,
    pub status: String,
    pub message: Option<String>,
}

// ============================================================================
// Auth Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub member_id: Uuid,
    pub phone_number: String,
    pub is_admin: bool,
}

// ============================================================================
// GraphQL Response Wrappers
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPublishedPostsResponse {
    pub published_posts: Vec<Post>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetListingsResponse {
    pub listings: PaginatedResult<Post>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetWebsitesResponse {
    pub websites: PaginatedResult<Website>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchOrganizationsResponse {
    pub search_organizations_semantic: Vec<OrganizationMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMessagesResponse {
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRecentChatsResponse {
    pub recent_chats: Vec<ChatContainer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateChatResponse {
    pub create_chat: ChatContainer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageResponse {
    pub send_message: ChatMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitResourceLinkResponse {
    pub submit_resource_link: JobResult,
}
