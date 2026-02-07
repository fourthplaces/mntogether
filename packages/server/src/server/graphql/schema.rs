//! GraphQL schema definition.

use super::context::GraphQLContext;
use juniper::{EmptySubscription, FieldError, FieldResult, RootNode};
use sqlx::PgPool;
use tracing::{error, info};
use uuid::Uuid;

// Common types
use crate::common::{ContainerId, PaginationArgs, PostId, ScheduleId, WebsiteId};

// Domain actions
use crate::domains::auth::activities as auth_activities;
use crate::domains::chatrooms::activities as chatroom_activities;
use crate::domains::crawling::activities as crawling_activities;
use crate::domains::discovery::activities as discovery_activities;
use crate::domains::discovery::models::{
    DiscoveryFilterRule, DiscoveryQuery, DiscoveryRun, DiscoveryRunResult,
};
use crate::domains::extraction::activities as extraction_activities;
use crate::domains::member::activities as member_activities;
use crate::domains::posts::activities as post_activities;
use crate::domains::providers::activities as provider_activities;
use crate::domains::website::activities as website_activities;
use crate::domains::website::activities::approval as website_approval_activities;

// Domain data types (GraphQL types)
use crate::domains::chatrooms::data::{ContainerData, MessageData};
use crate::domains::chatrooms::events::ChatEvent;
use crate::domains::extraction::data::{
    ExtractionData, ExtractionPageData, SubmitUrlInput, SubmitUrlResult, TriggerExtractionInput,
    TriggerExtractionResult,
};
use crate::domains::member::data::{MemberConnection, MemberData};
use crate::domains::member::events::MemberEvent;
use crate::domains::posts::data::post_report::{
    PostReport as PostReportData, PostReportDetail as PostReportDetailData,
};
use crate::domains::posts::data::types::RepostResult;
use crate::domains::posts::data::PostData;
use crate::domains::posts::data::{
    BusinessInfo, EditPostInput, NearbyPostType, PostConnection, PostStatusData, PostType,
    ScrapeJobResult, SubmitPostInput, SubmitResourceLinkInput, SubmitResourceLinkResult,
};
use crate::domains::providers::data::{
    ProviderConnection, ProviderData, SubmitProviderInput, UpdateProviderInput,
};
use crate::domains::providers::events::ProviderEvent;
use crate::domains::providers::models::Provider;
use crate::domains::website::data::{WebsiteConnection, WebsiteData};
use crate::domains::website::events::WebsiteEvent;
use crate::domains::website::data::{WebsiteAssessmentData, WebsiteSearchResultData};

// Sync proposal types
use crate::common::{MemberId, SyncBatchId, SyncProposalId};
use crate::domains::posts::activities::post_sync_handler::PostProposalHandler;
use crate::domains::sync::activities::proposal_actions;
use crate::domains::sync::{SyncBatch, SyncProposal};

// Domain models (for queries)
use crate::domains::chatrooms::models::{Container, Message};
use crate::domains::member::models::member::Member;
use crate::domains::posts::models::post_report::PostReportRecord;
use crate::domains::posts::models::{BusinessPost, Post};
use crate::domains::locations::models::ZipCode;
use crate::domains::schedules::models::Schedule;
use crate::domains::tag::TagData;
use crate::domains::website::models::{Website, WebsiteAssessment};

#[derive(juniper::GraphQLInputObject)]
pub struct TagInput {
    pub kind: String,
    pub value: String,
}

/// Result of running discovery search
#[derive(Debug, Clone, juniper::GraphQLObject)]
pub struct DiscoverySearchResult {
    pub queries_run: i32,
    pub total_results: i32,
    pub websites_created: i32,
    pub websites_filtered: i32,
    pub run_id: Uuid,
}

/// A discovery query (admin-managed search query)
#[derive(Debug, Clone, juniper::GraphQLObject)]
pub struct DiscoveryQueryData {
    pub id: Uuid,
    pub query_text: String,
    pub category: Option<String>,
    pub is_active: bool,
    pub created_at: String,
}

/// A discovery filter rule
#[derive(Debug, Clone, juniper::GraphQLObject)]
pub struct DiscoveryFilterRuleData {
    pub id: Uuid,
    pub query_id: Option<Uuid>,
    pub rule_text: String,
    pub sort_order: i32,
    pub is_active: bool,
}

/// A discovery run (execution of the pipeline)
#[derive(Debug, Clone, juniper::GraphQLObject)]
pub struct DiscoveryRunData {
    pub id: Uuid,
    pub queries_executed: i32,
    pub total_results: i32,
    pub websites_created: i32,
    pub websites_filtered: i32,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub trigger_type: String,
}

/// A single result from a discovery run (lineage)
#[derive(Debug, Clone, juniper::GraphQLObject)]
pub struct DiscoveryRunResultData {
    pub id: Uuid,
    pub run_id: Uuid,
    pub query_id: Uuid,
    pub domain: String,
    pub url: String,
    pub title: Option<String>,
    pub snippet: Option<String>,
    pub relevance_score: Option<f64>,
    pub filter_result: String,
    pub filter_reason: Option<String>,
    pub website_id: Option<Uuid>,
    pub discovered_at: String,
}

/// Result of post deduplication
#[derive(Debug, Clone, juniper::GraphQLObject)]
pub struct DeduplicationResult {
    pub job_id: Uuid,
    pub duplicates_found: i32,
    pub posts_merged: i32,
    pub posts_deleted: i32,
}

/// Result of generating missing embeddings
#[derive(Debug, Clone, juniper::GraphQLObject)]
pub struct GenerateEmbeddingsResult {
    pub processed: i32,
    pub failed: i32,
    pub remaining: i32,
}

/// Result of semantic post search
#[derive(Debug, Clone, juniper::GraphQLObject)]
pub struct PostSearchResultData {
    pub post_id: Uuid,
    pub title: String,
    pub description: String,
    pub category: String,
    pub post_type: String,
    pub similarity: f64,
}

/// Result of backfilling post embeddings
#[derive(Debug, Clone, juniper::GraphQLObject)]
pub struct BackfillPostEmbeddingsResult {
    pub processed: i32,
    pub failed: i32,
    pub remaining: i32,
}

/// Result of backfilling post locations
#[derive(Debug, Clone, juniper::GraphQLObject)]
pub struct BackfillPostLocationsResult {
    pub processed: i32,
    pub failed: i32,
    pub remaining: i32,
}

// =============================================================================
// Helper functions
// =============================================================================

/// Convert anyhow::Error to juniper FieldError for thin resolvers
fn to_field_error(e: anyhow::Error) -> FieldError {
    FieldError::new(e.to_string(), juniper::Value::null())
}

/// Convert a Post to PostType, loading business_info for business listings
async fn post_to_post_type(post: Post, pool: &PgPool) -> PostType {
    let mut post_type = PostType::from(post.clone());

    if post.post_type == "business" {
        if let Ok(Some(business)) = BusinessPost::find_by_post_id(post.id, pool).await {
            post_type.business_info = Some(BusinessInfo {
                accepts_donations: business.accepts_donations,
                donation_link: business.donation_link,
                gift_cards_available: business.gift_cards_available,
                gift_card_link: business.gift_card_link,
                online_ordering_link: business.online_ordering_link,
                delivery_available: business.delivery_available,
                proceeds_percentage: business.proceeds_percentage,
                proceeds_beneficiary_id: business.proceeds_beneficiary_id,
                proceeds_description: business.proceeds_description,
                impact_statement: business.impact_statement,
            });
        }
    }

    post_type
}

// =============================================================================
// Schedule GraphQL types
// =============================================================================

/// GraphQL type for a schedule entry
#[derive(Debug, Clone, juniper::GraphQLObject)]
#[graphql(context = GraphQLContext)]
pub struct ScheduleData {
    pub id: Uuid,
    pub schedulable_type: String,
    pub schedulable_id: Uuid,
    pub dtstart: Option<String>,
    pub dtend: Option<String>,
    pub rrule: Option<String>,
    pub exdates: Option<String>,
    pub is_all_day: bool,
    pub duration_minutes: Option<i32>,
    pub day_of_week: Option<i32>,
    pub opens_at: Option<String>,
    pub closes_at: Option<String>,
    pub timezone: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Schedule> for ScheduleData {
    fn from(s: Schedule) -> Self {
        Self {
            id: s.id.into_uuid(),
            schedulable_type: s.schedulable_type,
            schedulable_id: s.schedulable_id,
            dtstart: s.dtstart.map(|d| d.to_rfc3339()),
            dtend: s.dtend.map(|d| d.to_rfc3339()),
            rrule: s.rrule,
            exdates: s.exdates,
            is_all_day: s.is_all_day,
            duration_minutes: s.duration_minutes,
            day_of_week: s.day_of_week,
            opens_at: s.opens_at.map(|t| t.format("%H:%M").to_string()),
            closes_at: s.closes_at.map(|t| t.format("%H:%M").to_string()),
            timezone: s.timezone,
            notes: s.notes,
            created_at: s.created_at.to_rfc3339(),
            updated_at: s.updated_at.to_rfc3339(),
        }
    }
}

/// Input for creating or updating a schedule
#[derive(Debug, Clone, juniper::GraphQLInputObject)]
pub struct ScheduleInput {
    pub dtstart: Option<String>,
    pub dtend: Option<String>,
    pub rrule: Option<String>,
    pub exdates: Option<String>,
    pub opens_at: Option<String>,
    pub closes_at: Option<String>,
    pub day_of_week: Option<i32>,
    pub timezone: Option<String>,
    pub is_all_day: Option<bool>,
    pub duration_minutes: Option<i32>,
    pub notes: Option<String>,
}

/// Extract a 5-digit zip code from location text
fn extract_zip_from_text(text: &str) -> Option<String> {
    let re = regex::Regex::new(r"\b(\d{5})\b").ok()?;
    re.find(text).map(|m| m.as_str().to_string())
}

/// Extract a city name from location text (assumes "City, ST" or "City, State" pattern)
fn extract_city_from_text(text: &str) -> Option<String> {
    // Try "City, MN" or "City, Minnesota" pattern
    let re = regex::Regex::new(r"(?i)^([A-Za-z\s]+),\s*(?:MN|Minnesota)").ok()?;
    re.captures(text)
        .map(|c| c.get(1).unwrap().as_str().trim().to_string())
}

pub struct Query;

#[juniper::graphql_object(context = GraphQLContext)]
impl Query {
    // =========================================================================
    // Post Queries
    // =========================================================================

    /// Get a list of listings with filters
    /// Get paginated listings with cursor-based pagination (Relay spec)
    ///
    /// Arguments:
    /// - status: Filter by post status (default: active)
    /// - first: Return first N items (forward pagination)
    /// - after: Return items after this cursor (forward pagination)
    /// - last: Return last N items (backward pagination)
    /// - before: Return items before this cursor (backward pagination)
    async fn listings(
        ctx: &GraphQLContext,
        status: Option<PostStatusData>,
        first: Option<i32>,
        after: Option<String>,
        last: Option<i32>,
        before: Option<String>,
    ) -> FieldResult<PostConnection> {
        let status_filter = match status {
            Some(PostStatusData::Active) | None => "active",
            Some(PostStatusData::PendingApproval) => "pending_approval",
            Some(PostStatusData::Rejected) => "rejected",
            Some(PostStatusData::Expired) => "expired",
            Some(PostStatusData::Filled) => "filled",
        };

        let pagination_args = PaginationArgs {
            first,
            after,
            last,
            before,
        };
        let validated = pagination_args
            .validate()
            .map_err(|e| FieldError::new(e, juniper::Value::null()))?;

        let connection = post_activities::get_posts_paginated(status_filter, &validated, ctx.deps())
            .await
            .map_err(|e| {
                error!("Failed to get paginated posts: {}", e);
                FieldError::new("Failed to get posts", juniper::Value::null())
            })?;

        Ok(connection)
    }

    /// Get a single listing by ID
    async fn listing(ctx: &GraphQLContext, id: Uuid) -> FieldResult<Option<PostType>> {
        let post_id = PostId::from_uuid(id);
        let post = Post::find_by_id(post_id, &ctx.db_pool).await.ok().flatten();

        match post {
            Some(p) => Ok(Some(post_to_post_type(p, &ctx.db_pool).await)),
            None => Ok(None),
        }
    }

    /// Get published posts (for volunteers)
    async fn published_posts(
        _ctx: &GraphQLContext,
        _limit: Option<i32>,
    ) -> FieldResult<Vec<PostData>> {
        // The announcement model was removed
        Ok(vec![])
    }

    /// Get posts for a specific listing
    async fn posts_for_post(_ctx: &GraphQLContext, _post_id: Uuid) -> FieldResult<Vec<PostData>> {
        // The announcement model was removed
        Ok(vec![])
    }

    /// Get a single post by ID
    async fn post(ctx: &GraphQLContext, id: Uuid) -> FieldResult<Option<PostData>> {
        let post_id = PostId::from_uuid(id);

        match Post::find_by_id(post_id, &ctx.db_pool).await {
            Ok(Some(post)) => Ok(Some(PostData::from(post))),
            Ok(None) => Ok(None),
            Err(_) => Ok(None),
        }
    }

    /// Get all listing reports (admin only)
    async fn post_reports(
        ctx: &GraphQLContext,
        status: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> FieldResult<Vec<PostReportDetailData>> {
        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        if !user.is_admin {
            return Err(FieldError::new(
                "Admin access required",
                juniper::Value::null(),
            ));
        }

        let limit = limit.unwrap_or(50) as i64;
        let offset = offset.unwrap_or(0) as i64;

        let reports = match status.as_deref() {
            Some("pending") | None => {
                PostReportRecord::query_pending(limit, offset, &ctx.db_pool).await
            }
            _ => PostReportRecord::query_all(limit, offset, &ctx.db_pool).await,
        }
        .map_err(|e| {
            FieldError::new(
                format!("Failed to fetch reports: {}", e),
                juniper::Value::null(),
            )
        })?;

        Ok(reports.into_iter().map(|r| r.into()).collect())
    }

    /// Get reports for a specific listing (admin only)
    async fn reports_for_post(
        ctx: &GraphQLContext,
        post_id: Uuid,
    ) -> FieldResult<Vec<PostReportData>> {
        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        if !user.is_admin {
            return Err(FieldError::new(
                "Admin access required",
                juniper::Value::null(),
            ));
        }

        let post_id = PostId::from_uuid(post_id);
        let reports = PostReportRecord::query_for_post(post_id, &ctx.db_pool)
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to fetch reports: {}", e),
                    juniper::Value::null(),
                )
            })?;

        Ok(reports.into_iter().map(|r| r.into()).collect())
    }

    /// Search for posts near a zip code (proximity search)
    ///
    /// Arguments:
    /// - zip_code: Center zip code to search from (must be in reference table)
    /// - radius_miles: Search radius in miles (default: 25, max: 100)
    /// - limit: Maximum results to return (default: 50, max: 200)
    async fn search_posts_nearby(
        ctx: &GraphQLContext,
        zip_code: String,
        radius_miles: Option<f64>,
        limit: Option<i32>,
    ) -> FieldResult<Vec<NearbyPostType>> {
        let radius = radius_miles.unwrap_or(25.0).min(100.0);
        let limit = limit.unwrap_or(50).min(200);

        // Validate zip exists in reference table
        let center = ZipCode::find_by_code(&zip_code, &ctx.db_pool)
            .await
            .map_err(to_field_error)?
            .ok_or_else(|| {
                FieldError::new(
                    format!("Zip code '{}' not found in reference table", zip_code),
                    juniper::Value::null(),
                )
            })?;

        let _ = center; // validated existence

        let results = Post::find_near_zip(&zip_code, radius, limit, &ctx.db_pool)
            .await
            .map_err(to_field_error)?;

        let nearby_posts: Vec<NearbyPostType> = results
            .into_iter()
            .map(|pwd| {
                let post_type = PostType {
                    id: pwd.id.into_uuid(),
                    title: pwd.title,
                    tldr: pwd.tldr,
                    description: pwd.description,
                    description_markdown: pwd.description_markdown,
                    post_type: pwd.post_type,
                    category: pwd.category,
                    status: match pwd.status.as_str() {
                        "pending_approval" => PostStatusData::PendingApproval,
                        "active" => PostStatusData::Active,
                        "rejected" => PostStatusData::Rejected,
                        "expired" => PostStatusData::Expired,
                        "filled" => PostStatusData::Filled,
                        _ => PostStatusData::PendingApproval,
                    },
                    urgency: pwd.urgency,
                    location: pwd.location,
                    submission_type: pwd.submission_type,
                    source_url: pwd.source_url,
                    website_id: pwd.website_id.map(|id| id.into_uuid()),
                    created_at: pwd.created_at,
                    business_info: None,
                };
                NearbyPostType {
                    post: post_type,
                    distance_miles: pwd.distance_miles,
                    zip_code: pwd.zip_code,
                    city: pwd.location_city,
                }
            })
            .collect();

        Ok(nearby_posts)
    }

    // =========================================================================
    // Member Queries
    // =========================================================================

    /// Get a member by ID (admin only)
    async fn member(ctx: &GraphQLContext, id: String) -> FieldResult<Option<MemberData>> {
        ctx.require_admin()?;

        info!("get_member query called: {}", id);
        let member_id = Uuid::parse_str(&id)?;
        let member = Member::find_by_id(member_id, &ctx.db_pool).await?;
        Ok(Some(MemberData::from(member)))
    }

    /// Get paginated members with cursor-based pagination (Relay spec)
    ///
    /// Arguments:
    /// - first: Return first N items (forward pagination)
    /// - after: Return items after this cursor (forward pagination)
    /// - last: Return last N items (backward pagination)
    /// - before: Return items before this cursor (backward pagination)
    async fn members(
        ctx: &GraphQLContext,
        first: Option<i32>,
        after: Option<String>,
        last: Option<i32>,
        before: Option<String>,
    ) -> FieldResult<MemberConnection> {
        ctx.require_admin()?;

        let pagination_args = PaginationArgs {
            first,
            after,
            last,
            before,
        };
        let validated = pagination_args
            .validate()
            .map_err(|e| FieldError::new(e, juniper::Value::null()))?;

        let connection = member_activities::get_members_paginated(&validated, ctx.deps())
            .await
            .map_err(|e| {
                error!("Failed to get paginated members: {}", e);
                FieldError::new(e.to_string(), juniper::Value::null())
            })?;

        Ok(connection)
    }

    /// Search posts using semantic similarity (public)
    ///
    /// Arguments:
    /// - query: Natural language search query
    /// - threshold: Minimum similarity score (0-1, default: 0.6)
    /// - limit: Maximum results to return (default: 20)
    async fn search_posts_semantic(
        ctx: &GraphQLContext,
        query: String,
        threshold: Option<f64>,
        limit: Option<i32>,
    ) -> FieldResult<Vec<PostSearchResultData>> {
        let threshold = threshold.unwrap_or(0.6) as f32;
        let limit = limit.unwrap_or(20);

        // Generate embedding for the query
        let query_embedding = ctx
            .openai_client
            .create_embedding(&query, "text-embedding-3-small")
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to create embedding: {}", e),
                    juniper::Value::null(),
                )
            })?;

        // Search posts by embedding similarity
        let results = Post::search_by_similarity(&query_embedding, threshold, limit, &ctx.db_pool)
            .await
            .map_err(|e| {
                FieldError::new(format!("Search failed: {}", e), juniper::Value::null())
            })?;

        Ok(results
            .into_iter()
            .map(|r| PostSearchResultData {
                post_id: r.post_id.into_uuid(),
                title: r.title,
                description: r.description,
                category: r.category,
                post_type: r.post_type,
                similarity: r.similarity,
            })
            .collect())
    }

    // =========================================================================
    // Website Queries
    // =========================================================================

    /// Get all websites with optional status filter (admin only)
    /// Get paginated websites with cursor-based pagination (Relay spec)
    ///
    /// Arguments:
    /// - status: Filter by website status (pending_review, approved, rejected, suspended)
    /// - first: Return first N items (forward pagination)
    /// - after: Return items after this cursor (forward pagination)
    /// - last: Return last N items (backward pagination)
    /// - before: Return items before this cursor (backward pagination)
    async fn websites(
        ctx: &GraphQLContext,
        status: Option<String>,
        first: Option<i32>,
        after: Option<String>,
        last: Option<i32>,
        before: Option<String>,
    ) -> FieldResult<WebsiteConnection> {
        ctx.require_admin()?;

        let pagination_args = PaginationArgs {
            first,
            after,
            last,
            before,
        };
        let validated = pagination_args
            .validate()
            .map_err(|e| FieldError::new(e, juniper::Value::null()))?;

        let status_ref = status.as_deref();

        let connection =
            website_activities::get_websites_paginated(status_ref, &validated, ctx.deps())
                .await
                .map_err(|e| {
                    error!("Failed to get paginated websites: {}", e);
                    FieldError::new(e.to_string(), juniper::Value::null())
                })?;

        Ok(connection)
    }

    /// Get a single website by ID (admin only)
    async fn website(ctx: &GraphQLContext, id: Uuid) -> FieldResult<Option<WebsiteData>> {
        ctx.require_admin()?;

        let website_id = WebsiteId::from_uuid(id);

        match Website::find_by_id(website_id, &ctx.db_pool).await {
            Ok(website) => Ok(Some(WebsiteData::from(website))),
            Err(_) => Ok(None),
        }
    }

    /// Get websites pending review (for admin approval queue)
    async fn pending_websites(ctx: &GraphQLContext) -> FieldResult<Vec<WebsiteData>> {
        ctx.require_admin()?;

        let websites = website_activities::get_pending_websites(ctx.deps())
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to fetch pending websites: {}", e),
                    juniper::Value::null(),
                )
            })?;

        Ok(websites.into_iter().map(WebsiteData::from).collect())
    }

    /// Get the latest assessment for a website (admin only)
    async fn website_assessment(
        ctx: &GraphQLContext,
        website_id: String,
    ) -> FieldResult<Option<WebsiteAssessmentData>> {
        ctx.require_admin()?;

        let website_uuid = Uuid::parse_str(&website_id).map_err(|e| {
            FieldError::new(format!("Invalid website ID: {}", e), juniper::Value::null())
        })?;

        let assessment = WebsiteAssessment::find_latest_by_website_id(website_uuid, &ctx.db_pool)
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to find assessment: {}", e),
                    juniper::Value::null(),
                )
            })?;

        Ok(assessment.map(WebsiteAssessmentData::from))
    }

    /// Search websites semantically using natural language queries
    async fn search_websites(
        ctx: &GraphQLContext,
        query: String,
        limit: Option<i32>,
        threshold: Option<f64>,
    ) -> FieldResult<Vec<WebsiteSearchResultData>> {
        let limit = limit.unwrap_or(10) as i32;
        let threshold = threshold.unwrap_or(0.7) as f32;

        // Generate embedding for the query
        let query_embedding = ctx
            .openai_client
            .create_embedding(&query, "text-embedding-3-small")
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to create embedding: {}", e),
                    juniper::Value::null(),
                )
            })?;

        // Search websites by embedding similarity
        let results = WebsiteAssessment::search_by_similarity(
            &query_embedding,
            threshold,
            limit,
            &ctx.db_pool,
        )
        .await
        .map_err(|e| FieldError::new(format!("Search failed: {}", e), juniper::Value::null()))?;

        Ok(results
            .into_iter()
            .map(WebsiteSearchResultData::from)
            .collect())
    }

    // =========================================================================
    // Post Revision Queries
    // =========================================================================

    /// Get all pending revisions (admin only)
    ///
    /// Revisions are draft posts created when AI updates detect changes.
    /// They await review before being applied to the original post.
    async fn pending_revisions(
        ctx: &GraphQLContext,
        website_id: Option<Uuid>,
    ) -> FieldResult<Vec<PostType>> {
        ctx.require_admin()?;

        let website_id = website_id.map(WebsiteId::from_uuid);
        let revisions = post_activities::get_pending_revisions(website_id, &ctx.db_pool)
            .await
            .map_err(to_field_error)?;

        let mut result = Vec::with_capacity(revisions.len());
        for post in revisions {
            result.push(post_to_post_type(post, &ctx.db_pool).await);
        }
        Ok(result)
    }

    /// Get the revision for a specific post (if any exists) (admin only)
    async fn revision_for_post(
        ctx: &GraphQLContext,
        post_id: Uuid,
    ) -> FieldResult<Option<PostType>> {
        ctx.require_admin()?;

        let post_id = PostId::from_uuid(post_id);
        let revision = post_activities::get_revision_for_post(post_id, &ctx.db_pool)
            .await
            .map_err(to_field_error)?;

        match revision {
            Some(post) => Ok(Some(post_to_post_type(post, &ctx.db_pool).await)),
            None => Ok(None),
        }
    }

    // =========================================================================
    // Extraction Page Queries
    // =========================================================================

    /// Get an extraction page by URL
    async fn extraction_page(
        ctx: &GraphQLContext,
        url: String,
    ) -> FieldResult<Option<ExtractionPageData>> {
        let page = ExtractionPageData::find_by_url(&url, &ctx.db_pool)
            .await
            .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))?;
        Ok(page)
    }

    /// Get extraction pages for a domain
    ///
    /// Returns pages from the extraction_pages table for the given domain.
    async fn extraction_pages(
        ctx: &GraphQLContext,
        domain: String,
        limit: Option<i32>,
    ) -> FieldResult<Vec<ExtractionPageData>> {
        let limit = limit.unwrap_or(50);
        let pages = ExtractionPageData::find_by_domain(&domain, limit, &ctx.db_pool)
            .await
            .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))?;
        Ok(pages)
    }

    /// Count extraction pages for a domain
    async fn extraction_pages_count(ctx: &GraphQLContext, domain: String) -> FieldResult<i32> {
        let count = ExtractionPageData::count_by_domain(&domain, &ctx.db_pool)
            .await
            .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))?;
        Ok(count)
    }

    // =========================================================================
    // Chatroom Queries
    // =========================================================================

    /// Get a chat container by ID
    async fn container(ctx: &GraphQLContext, id: String) -> FieldResult<Option<ContainerData>> {
        let container_id = ContainerId::parse(&id)?;

        match Container::find_by_id(container_id, &ctx.db_pool).await {
            Ok(container) => Ok(Some(ContainerData::from(container))),
            Err(_) => Ok(None),
        }
    }

    /// Get messages for a chat container
    async fn messages(ctx: &GraphQLContext, container_id: String) -> FieldResult<Vec<MessageData>> {
        let container_id = ContainerId::parse(&container_id)?;
        let messages = Message::find_by_container(container_id, &ctx.db_pool).await?;
        Ok(messages.into_iter().map(MessageData::from).collect())
    }

    /// Get recent AI chat containers
    async fn recent_chats(
        ctx: &GraphQLContext,
        limit: Option<i32>,
    ) -> FieldResult<Vec<ContainerData>> {
        let limit = limit.unwrap_or(20) as i64;
        let containers = Container::find_recent(limit, &ctx.db_pool).await?;
        Ok(containers.into_iter().map(ContainerData::from).collect())
    }

    // =========================================================================
    // Provider Queries
    // =========================================================================

    /// Get a provider by ID (admin only)
    async fn provider(ctx: &GraphQLContext, id: String) -> FieldResult<Option<ProviderData>> {
        ctx.require_admin()?;

        info!("get_provider query called: {}", id);

        let provider = provider_activities::get_provider(id, ctx.deps())
            .await
            .map_err(to_field_error)?;

        Ok(provider.map(ProviderData::from))
    }

    /// Get all providers with optional filters
    /// Get paginated providers with cursor-based pagination (Relay spec)
    async fn providers(
        ctx: &GraphQLContext,
        status: Option<String>,
        first: Option<i32>,
        after: Option<String>,
        last: Option<i32>,
        before: Option<String>,
    ) -> FieldResult<ProviderConnection> {
        ctx.require_admin()?;

        let pagination_args = PaginationArgs {
            first,
            after,
            last,
            before,
        };
        let validated = pagination_args
            .validate()
            .map_err(|e| FieldError::new(e, juniper::Value::null()))?;

        let status_ref = status.as_deref();

        let connection =
            provider_activities::get_providers_paginated(status_ref, &validated, ctx.deps())
                .await
                .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))?;

        Ok(connection)
    }

    /// Get all pending providers (for admin approval queue)
    async fn pending_providers(ctx: &GraphQLContext) -> FieldResult<Vec<ProviderData>> {
        ctx.require_admin()?;

        let providers = provider_activities::get_pending_providers(ctx.deps())
            .await
            .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))?;

        Ok(providers.into_iter().map(ProviderData::from).collect())
    }

    // =========================================================================
    // Discovery Queries
    // =========================================================================

    /// Get all discovery queries (admin only)
    async fn discovery_queries(
        ctx: &GraphQLContext,
        include_inactive: Option<bool>,
    ) -> FieldResult<Vec<DiscoveryQueryData>> {
        ctx.require_admin()?;

        let queries = if include_inactive.unwrap_or(false) {
            DiscoveryQuery::find_all(&ctx.db_pool)
                .await
                .map_err(to_field_error)?
        } else {
            DiscoveryQuery::find_active(&ctx.db_pool)
                .await
                .map_err(to_field_error)?
        };

        Ok(queries
            .into_iter()
            .map(|q| DiscoveryQueryData {
                id: q.id,
                query_text: q.query_text,
                category: q.category,
                is_active: q.is_active,
                created_at: q.created_at.to_rfc3339(),
            })
            .collect())
    }

    /// Get filter rules (admin only). Pass queryId for per-query rules, null for global rules.
    async fn discovery_filter_rules(
        ctx: &GraphQLContext,
        query_id: Option<Uuid>,
    ) -> FieldResult<Vec<DiscoveryFilterRuleData>> {
        ctx.require_admin()?;

        let rules = DiscoveryFilterRule::find_all_for_query(query_id, &ctx.db_pool)
            .await
            .map_err(to_field_error)?;

        Ok(rules
            .into_iter()
            .map(|r| DiscoveryFilterRuleData {
                id: r.id,
                query_id: r.query_id,
                rule_text: r.rule_text,
                sort_order: r.sort_order,
                is_active: r.is_active,
            })
            .collect())
    }

    /// Get recent discovery runs (admin only)
    async fn discovery_runs(
        ctx: &GraphQLContext,
        limit: Option<i32>,
    ) -> FieldResult<Vec<DiscoveryRunData>> {
        ctx.require_admin()?;

        let runs = DiscoveryRun::find_recent(limit.unwrap_or(20), &ctx.db_pool)
            .await
            .map_err(to_field_error)?;

        Ok(runs
            .into_iter()
            .map(|r| DiscoveryRunData {
                id: r.id,
                queries_executed: r.queries_executed,
                total_results: r.total_results,
                websites_created: r.websites_created,
                websites_filtered: r.websites_filtered,
                started_at: r.started_at.to_rfc3339(),
                completed_at: r.completed_at.map(|t| t.to_rfc3339()),
                trigger_type: r.trigger_type,
            })
            .collect())
    }

    /// Get results for a specific discovery run (admin only)
    async fn discovery_run_results(
        ctx: &GraphQLContext,
        run_id: Uuid,
    ) -> FieldResult<Vec<DiscoveryRunResultData>> {
        ctx.require_admin()?;

        let results = DiscoveryRunResult::find_by_run(run_id, &ctx.db_pool)
            .await
            .map_err(to_field_error)?;

        Ok(results.into_iter().map(run_result_to_data).collect())
    }

    /// Get discovery sources for a website (reverse lineage, admin only)
    async fn website_discovery_sources(
        ctx: &GraphQLContext,
        website_id: Uuid,
    ) -> FieldResult<Vec<DiscoveryRunResultData>> {
        ctx.require_admin()?;

        let results = DiscoveryRunResult::find_by_website(website_id, &ctx.db_pool)
            .await
            .map_err(to_field_error)?;

        Ok(results.into_iter().map(run_result_to_data).collect())
    }

    // =========================================================================
    // Sync Proposal Queries
    // =========================================================================

    /// Get sync batches (admin only). Filter by status or get all recent.
    async fn sync_batches(
        ctx: &GraphQLContext,
        status: Option<String>,
        limit: Option<i32>,
    ) -> FieldResult<Vec<SyncBatchData>> {
        ctx.require_admin()?;

        let batches = match status.as_deref() {
            Some("pending") => SyncBatch::find_pending(&ctx.db_pool)
                .await
                .map_err(to_field_error)?,
            _ => SyncBatch::find_recent(limit.unwrap_or(50), &ctx.db_pool)
                .await
                .map_err(to_field_error)?,
        };

        Ok(batches.into_iter().map(sync_batch_to_data).collect())
    }

    /// Get a single sync batch by ID (admin only)
    async fn sync_batch(ctx: &GraphQLContext, id: Uuid) -> FieldResult<Option<SyncBatchData>> {
        ctx.require_admin()?;

        let batch_id = SyncBatchId::from(id);
        let batch = SyncBatch::find_by_id(batch_id, &ctx.db_pool)
            .await
            .map_err(to_field_error)?;

        Ok(batch.map(sync_batch_to_data))
    }

    /// Get all proposals for a batch (admin only)
    async fn sync_proposals(
        ctx: &GraphQLContext,
        batch_id: Uuid,
    ) -> FieldResult<Vec<SyncProposalData>> {
        ctx.require_admin()?;

        let batch_id = SyncBatchId::from(batch_id);
        let proposals = SyncProposal::find_by_batch(batch_id, &ctx.db_pool)
            .await
            .map_err(to_field_error)?;

        let mut result = Vec::with_capacity(proposals.len());
        for p in proposals {
            result.push(enrich_proposal(p, &ctx.db_pool).await);
        }
        Ok(result)
    }

    // =========================================================================
    // Schedule / Event Queries
    // =========================================================================

    /// Get upcoming events: posts tagged `post_type: event` with schedules,
    /// ordered by next occurrence (computed from rrule).
    async fn upcoming_events(
        ctx: &GraphQLContext,
        first: Option<i32>,
    ) -> FieldResult<Vec<PostType>> {
        let limit = first.unwrap_or(20).min(100) as usize;

        // Load all schedules attached to event-tagged posts
        let schedules = Post::find_event_schedules(&ctx.db_pool)
            .await
            .map_err(to_field_error)?;

        // Group by post, find earliest next occurrence per post
        let mut post_next: std::collections::HashMap<Uuid, chrono::DateTime<chrono::Utc>> =
            std::collections::HashMap::new();

        for schedule in &schedules {
            if let Some(next) = schedule.next_occurrences(1).into_iter().next() {
                let entry = post_next.entry(schedule.schedulable_id).or_insert(next);
                if next < *entry {
                    *entry = next;
                }
            }
        }

        // Sort by next occurrence
        let mut sorted: Vec<_> = post_next.into_iter().collect();
        sorted.sort_by_key(|(_, next)| *next);
        sorted.truncate(limit);

        // Batch-load all posts in one query
        let post_ids: Vec<Uuid> = sorted.iter().map(|(id, _)| *id).collect();
        let loaded = Post::find_by_ids(&post_ids, &ctx.db_pool)
            .await
            .map_err(to_field_error)?;

        // Index by ID to preserve sort order
        let post_map: std::collections::HashMap<Uuid, Post> = loaded
            .into_iter()
            .map(|p| (p.id.into_uuid(), p))
            .collect();

        // Batch-load business info for business-type posts
        let business_post_ids: Vec<Uuid> = post_map
            .values()
            .filter(|p| p.post_type == "business")
            .map(|p| p.id.into_uuid())
            .collect();
        let business_map: std::collections::HashMap<Uuid, BusinessPost> = if business_post_ids.is_empty() {
            std::collections::HashMap::new()
        } else {
            BusinessPost::find_by_post_ids(&business_post_ids, &ctx.db_pool)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|b| (b.post_id.into_uuid(), b))
                .collect()
        };

        // Build PostType vec in sorted order
        let posts: Vec<PostType> = sorted
            .iter()
            .filter_map(|(id, _)| {
                post_map.get(id).map(|post| {
                    let mut pt = PostType::from(post.clone());
                    if let Some(business) = business_map.get(id) {
                        pt.business_info = Some(BusinessInfo {
                            accepts_donations: business.accepts_donations,
                            donation_link: business.donation_link.clone(),
                            gift_cards_available: business.gift_cards_available,
                            gift_card_link: business.gift_card_link.clone(),
                            online_ordering_link: business.online_ordering_link.clone(),
                            delivery_available: business.delivery_available,
                            proceeds_percentage: business.proceeds_percentage,
                            proceeds_beneficiary_id: business.proceeds_beneficiary_id,
                            proceeds_description: business.proceeds_description.clone(),
                            impact_statement: business.impact_statement.clone(),
                        });
                    }
                    pt
                })
            })
            .collect();

        Ok(posts)
    }

    /// Get schedules for a specific entity
    async fn schedules_for_entity(
        ctx: &GraphQLContext,
        schedulable_type: String,
        schedulable_id: Uuid,
    ) -> FieldResult<Vec<ScheduleData>> {
        let schedules = Schedule::find_for_entity(&schedulable_type, schedulable_id, &ctx.db_pool)
            .await
            .map_err(to_field_error)?;

        Ok(schedules.into_iter().map(ScheduleData::from).collect())
    }
}

/// Convert a SyncBatch model to GraphQL data type
fn sync_batch_to_data(b: SyncBatch) -> SyncBatchData {
    SyncBatchData {
        id: b.id.into_uuid(),
        resource_type: b.resource_type,
        source_id: b.source_id,
        status: b.status,
        summary: b.summary,
        proposal_count: b.proposal_count,
        approved_count: b.approved_count,
        rejected_count: b.rejected_count,
        created_at: b.created_at.to_rfc3339(),
        reviewed_at: b.reviewed_at.map(|t| t.to_rfc3339()),
    }
}

/// Convert a SyncProposal to GraphQL data (without enrichment, for mutation returns)
fn sync_proposal_to_data(p: SyncProposal) -> SyncProposalData {
    SyncProposalData {
        id: p.id.into_uuid(),
        batch_id: p.batch_id.into_uuid(),
        operation: p.operation,
        status: p.status,
        entity_type: p.entity_type,
        draft_entity_id: p.draft_entity_id,
        target_entity_id: p.target_entity_id,
        reason: p.reason,
        reviewed_by: p.reviewed_by,
        reviewed_at: p.reviewed_at.map(|t| t.to_rfc3339()),
        created_at: p.created_at.to_rfc3339(),
        draft_title: None,
        target_title: None,
        merge_source_ids: vec![],
        merge_source_titles: vec![],
    }
}

/// Look up a post title by UUID, returning None on any failure
async fn lookup_post_title(id: Uuid, pool: &PgPool) -> Option<String> {
    let post_id = PostId::from_uuid(id);
    Post::find_by_id(post_id, pool)
        .await
        .ok()
        .flatten()
        .map(|p| {
            if p.title.is_empty() {
                format!("Untitled ({})", p.post_type)
            } else {
                p.title
            }
        })
}

/// Enrich a SyncProposal with human-readable titles and merge sources
async fn enrich_proposal(p: SyncProposal, pool: &PgPool) -> SyncProposalData {
    use crate::domains::sync::SyncProposalMergeSource;

    let draft_title = match p.draft_entity_id {
        Some(id) => lookup_post_title(id, pool).await,
        None => None,
    };

    let target_title = match p.target_entity_id {
        Some(id) => lookup_post_title(id, pool).await,
        None => None,
    };

    let merge_sources = SyncProposalMergeSource::find_by_proposal(p.id, pool)
        .await
        .unwrap_or_default();

    let merge_source_ids: Vec<Uuid> = merge_sources.iter().map(|s| s.source_entity_id).collect();

    let mut merge_source_titles = Vec::new();
    for id in &merge_source_ids {
        if let Some(title) = lookup_post_title(*id, pool).await {
            merge_source_titles.push(title);
        }
    }

    SyncProposalData {
        id: p.id.into_uuid(),
        batch_id: p.batch_id.into_uuid(),
        operation: p.operation,
        status: p.status,
        entity_type: p.entity_type,
        draft_entity_id: p.draft_entity_id,
        target_entity_id: p.target_entity_id,
        reason: p.reason,
        reviewed_by: p.reviewed_by,
        reviewed_at: p.reviewed_at.map(|t| t.to_rfc3339()),
        created_at: p.created_at.to_rfc3339(),
        draft_title,
        target_title,
        merge_source_ids,
        merge_source_titles,
    }
}

/// Convert a DiscoveryRunResult model to GraphQL data type
fn run_result_to_data(r: crate::domains::discovery::DiscoveryRunResult) -> DiscoveryRunResultData {
    DiscoveryRunResultData {
        id: r.id,
        run_id: r.run_id,
        query_id: r.query_id,
        domain: r.domain,
        url: r.url,
        title: r.title,
        snippet: r.snippet,
        relevance_score: r.relevance_score,
        filter_result: r.filter_result,
        filter_reason: r.filter_reason,
        website_id: r.website_id,
        discovered_at: r.discovered_at.to_rfc3339(),
    }
}

pub struct Mutation;

#[juniper::graphql_object(context = GraphQLContext)]
impl Mutation {
    // =========================================================================
    // Post Mutations
    // =========================================================================

    /// Crawl a website (multi-page) to discover and extract listings (admin only)
    ///
    /// Starts a durable Restate workflow that orchestrates the full crawl pipeline.
    async fn crawl_website(ctx: &GraphQLContext, website_id: Uuid) -> FieldResult<ScrapeJobResult> {
        use crate::domains::crawling::workflows::CrawlWebsiteRequest;

        info!(website_id = %website_id, "Starting crawl website workflow");

        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        // Authorization check (workflow assumes this is done)
        use crate::common::auth::{Actor, AdminCapability};
        use crate::common::MemberId;

        Actor::new(MemberId::from_uuid(user.member_id.into_uuid()), user.is_admin)
            .can(AdminCapability::TriggerScraping)
            .check(ctx.deps())
            .await
            .map_err(|e| to_field_error(e.into()))?;

        let visitor_id = user.member_id.into_uuid();

        // Start workflow (async - doesn't wait for completion)
        let workflow_id = ctx
            .workflow_client
            .start_workflow(
                "CrawlWebsite",
                "run",
                CrawlWebsiteRequest {
                    website_id,
                    visitor_id,
                    use_firecrawl: true,
                },
            )
            .await
            .map_err(to_field_error)?;

        Ok(ScrapeJobResult {
            job_id: workflow_id.parse().unwrap_or(uuid::Uuid::new_v4()),
            source_id: website_id,
            status: "started".to_string(),
            message: Some("Crawl workflow started".to_string()),
        })
    }

    /// Discover website pages using Tavily search instead of traditional crawling.
    /// Uses site-scoped search queries to find relevant content pages.
    async fn discover_website(
        ctx: &GraphQLContext,
        website_id: Uuid,
    ) -> FieldResult<ScrapeJobResult> {
        info!(website_id = %website_id, "Discovering website via Tavily search");

        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let event = crawling_activities::discover_website(
            website_id,
            user.member_id.into_uuid(),
            user.is_admin,
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event)
            .await
            .map_err(to_field_error)?;

        Ok(ScrapeJobResult {
            job_id: Uuid::new_v4(),
            source_id: website_id,
            status: "processing".to_string(),
            message: Some("Discovery started".to_string()),
        })
    }

    /// Submit a listing from a member (public, goes to pending_approval)
    async fn submit_post(
        ctx: &GraphQLContext,
        input: SubmitPostInput,
        member_id: Option<Uuid>,
    ) -> FieldResult<PostType> {
        use crate::domains::posts::events::PostEvent;

        let event = post_activities::submit_post(input.clone(), member_id, ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event.clone())
            .await
            .map_err(to_field_error)?;

        // Extract post_id from event and find the post
        let post_id = match event {
            PostEvent::PostEntryCreated { post_id, .. } => post_id,
            _ => {
                return Err(FieldError::new(
                    "Unexpected event type",
                    juniper::Value::null(),
                ))
            }
        };

        let post = Post::find_by_id(post_id, &ctx.db_pool)
            .await
            .map_err(to_field_error)?
            .ok_or_else(|| FieldError::new("Post not found", juniper::Value::null()))?;

        Ok(PostType::from(post))
    }

    /// Submit a resource link (URL) for scraping (public)
    async fn submit_resource_link(
        ctx: &GraphQLContext,
        input: SubmitResourceLinkInput,
    ) -> FieldResult<SubmitResourceLinkResult> {
        use crate::domains::posts::events::PostEvent;

        let url = input.url.clone();

        let event =
            post_activities::submit_resource_link(input.url, input.submitter_contact, ctx.deps())
                .await
                .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event.clone())
            .await
            .map_err(to_field_error)?;

        // Map event to appropriate response based on website status
        match event {
            PostEvent::WebsitePendingApproval { .. } => Ok(SubmitResourceLinkResult {
                job_id: Uuid::new_v4(),
                status: "pending_review".to_string(),
                message: format!("Website is pending admin approval: {}", url),
            }),
            PostEvent::WebsiteCreatedFromLink { job_id, .. } => Ok(SubmitResourceLinkResult {
                job_id: job_id.into(),
                status: "processing".to_string(),
                message: format!(
                    "Your submission has been received. We'll process it shortly: {}",
                    url
                ),
            }),
            _ => Err(FieldError::new(
                "Unexpected event type",
                juniper::Value::null(),
            )),
        }
    }

    /// Approve a listing (make it visible to volunteers) (admin only)
    async fn approve_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<PostType> {
        use crate::domains::posts::events::PostEvent;

        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let event = post_activities::approve_post(
            post_id,
            user.member_id.into_uuid(),
            user.is_admin,
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event.clone())
            .await
            .map_err(to_field_error)?;

        let PostEvent::PostApproved { post_id } = event else {
            return Err(FieldError::new(
                "Unexpected event type",
                juniper::Value::null(),
            ));
        };

        let post = Post::find_by_id(post_id, &ctx.db_pool)
            .await
            .map_err(to_field_error)?
            .ok_or_else(|| FieldError::new("Post not found", juniper::Value::null()))?;

        Ok(PostType::from(post))
    }

    /// Edit and approve a listing (fix AI mistakes or improve user content) (admin only)
    async fn edit_and_approve_post(
        ctx: &GraphQLContext,
        post_id: Uuid,
        input: EditPostInput,
    ) -> FieldResult<PostType> {
        use crate::domains::posts::events::PostEvent;

        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let event = post_activities::edit_and_approve_post(
            post_id,
            input,
            user.member_id.into_uuid(),
            user.is_admin,
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event.clone())
            .await
            .map_err(to_field_error)?;

        let PostEvent::PostApproved { post_id } = event else {
            return Err(FieldError::new(
                "Unexpected event type",
                juniper::Value::null(),
            ));
        };

        let post = Post::find_by_id(post_id, &ctx.db_pool)
            .await
            .map_err(to_field_error)?
            .ok_or_else(|| FieldError::new("Post not found", juniper::Value::null()))?;

        Ok(PostType::from(post))
    }

    /// Reject a listing (hide forever) (admin only)
    async fn reject_post(ctx: &GraphQLContext, post_id: Uuid, reason: String) -> FieldResult<bool> {
        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let event = post_activities::reject_post(
            post_id,
            reason,
            user.member_id.into_uuid(),
            user.is_admin,
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event)
            .await
            .map_err(to_field_error)?;

        Ok(true)
    }

    /// Delete a listing (admin only)
    async fn delete_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let event = post_activities::delete_post(
            post_id,
            user.member_id.into_uuid(),
            user.is_admin,
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event)
            .await
            .map_err(to_field_error)?;

        Ok(true)
    }

    /// Repost a listing (create new post for existing active listing) (admin only)
    async fn repost_post(_ctx: &GraphQLContext, _post_id: Uuid) -> FieldResult<RepostResult> {
        Err(FieldError::new(
            "Reposting is not currently supported",
            juniper::Value::null(),
        ))
    }

    /// Expire a post (admin only)
    async fn expire_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<PostData> {
        use crate::domains::posts::events::PostEvent;

        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let event = post_activities::expire_post(
            post_id,
            user.member_id.into_uuid(),
            user.is_admin,
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event.clone())
            .await
            .map_err(to_field_error)?;

        let PostEvent::PostExpired { post_id } = event else {
            return Err(FieldError::new(
                "Unexpected event type",
                juniper::Value::null(),
            ));
        };

        let post = Post::find_by_id(post_id, &ctx.db_pool)
            .await
            .map_err(to_field_error)?
            .ok_or_else(|| FieldError::new("Post not found", juniper::Value::null()))?;

        Ok(PostData::from(post))
    }

    /// Archive a post (admin only)
    async fn archive_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<PostData> {
        use crate::domains::posts::events::PostEvent;

        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let event = post_activities::archive_post(
            post_id,
            user.member_id.into_uuid(),
            user.is_admin,
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event.clone())
            .await
            .map_err(to_field_error)?;

        let PostEvent::PostArchived { post_id } = event else {
            return Err(FieldError::new(
                "Unexpected event type",
                juniper::Value::null(),
            ));
        };

        let post = Post::find_by_id(post_id, &ctx.db_pool)
            .await
            .map_err(to_field_error)?
            .ok_or_else(|| FieldError::new("Post not found", juniper::Value::null()))?;

        Ok(PostData::from(post))
    }

    /// Track post view (analytics - public)
    async fn post_viewed(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
        let event = post_activities::track_post_view(post_id, ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event)
            .await
            .map_err(to_field_error)?;

        Ok(true)
    }

    /// Track post click (analytics - public)
    async fn post_clicked(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
        let event = post_activities::track_post_click(post_id, ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event)
            .await
            .map_err(to_field_error)?;

        Ok(true)
    }

    /// Report a listing (public or authenticated)
    async fn report_post(
        ctx: &GraphQLContext,
        post_id: Uuid,
        reason: String,
        category: String,
        reporter_email: Option<String>,
    ) -> FieldResult<PostReportData> {
        use crate::domains::posts::events::PostEvent;

        let reported_by = ctx.auth_user.as_ref().map(|u| u.member_id.into_uuid());

        let event = post_activities::report_post(
            post_id,
            reported_by,
            reporter_email,
            reason,
            category,
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event.clone())
            .await
            .map_err(to_field_error)?;

        let PostEvent::PostReported {
            report_id,
            post_id: returned_post_id,
        } = event
        else {
            return Err(FieldError::new(
                "Unexpected event type",
                juniper::Value::null(),
            ));
        };

        // Fetch the report from DB
        let reports = PostReportRecord::query_for_post(returned_post_id, &ctx.db_pool)
            .await
            .map_err(to_field_error)?;

        let report = reports
            .into_iter()
            .find(|r| r.id == report_id)
            .ok_or_else(|| FieldError::new("Report not found", juniper::Value::null()))?;

        Ok(report.into())
    }

    /// Resolve a report (admin only)
    async fn resolve_report(
        ctx: &GraphQLContext,
        report_id: Uuid,
        resolution_notes: Option<String>,
        action_taken: String,
    ) -> FieldResult<bool> {
        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let event = post_activities::resolve_report(
            report_id,
            user.member_id.into_uuid(),
            user.is_admin,
            resolution_notes,
            action_taken,
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event)
            .await
            .map_err(to_field_error)?;

        Ok(true)
    }

    /// Dismiss a report (admin only)
    async fn dismiss_report(
        ctx: &GraphQLContext,
        report_id: Uuid,
        resolution_notes: Option<String>,
    ) -> FieldResult<bool> {
        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let event = post_activities::dismiss_report(
            report_id,
            user.member_id.into_uuid(),
            user.is_admin,
            resolution_notes,
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event)
            .await
            .map_err(to_field_error)?;

        Ok(true)
    }

    // =========================================================================
    // Post Revision Mutations
    // =========================================================================

    /// Approve a revision: copy revision fields to original, delete revision (admin only)
    ///
    /// When AI updates detect changes to an existing post, they create a revision
    /// for review. This mutation applies those changes to the original post.
    async fn approve_revision(ctx: &GraphQLContext, revision_id: Uuid) -> FieldResult<PostType> {
        ctx.require_admin()?;

        let revision_id = PostId::from_uuid(revision_id);
        let updated_post = post_activities::approve_revision(revision_id, &ctx.db_pool)
            .await
            .map_err(to_field_error)?;

        Ok(post_to_post_type(updated_post, &ctx.db_pool).await)
    }

    /// Reject a revision: delete revision, original unchanged (admin only)
    ///
    /// Use this when the AI-suggested changes should not be applied.
    async fn reject_revision(ctx: &GraphQLContext, revision_id: Uuid) -> FieldResult<bool> {
        ctx.require_admin()?;

        let revision_id = PostId::from_uuid(revision_id);
        post_activities::reject_revision(revision_id, &ctx.db_pool)
            .await
            .map_err(to_field_error)?;

        Ok(true)
    }

    // =========================================================================
    // Discovery Mutations
    // =========================================================================

    /// Run discovery search manually (admin only)
    async fn run_discovery_search(ctx: &GraphQLContext) -> FieldResult<DiscoverySearchResult> {
        ctx.require_admin()?;

        let user = ctx.auth_user.as_ref().unwrap();
        info!(requested_by = %user.member_id, "Admin triggering manual discovery search");

        let event = discovery_activities::run_discovery("manual", ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event.clone())
            .await
            .map_err(to_field_error)?;

        let crate::domains::discovery::DiscoveryEvent::DiscoveryRunCompleted {
            run_id,
            queries_executed,
            total_results,
            websites_created,
            websites_filtered,
        } = event;

        Ok(DiscoverySearchResult {
            queries_run: queries_executed as i32,
            total_results: total_results as i32,
            websites_created: websites_created as i32,
            websites_filtered: websites_filtered as i32,
            run_id,
        })
    }

    /// Create a new discovery query (admin only)
    async fn create_discovery_query(
        ctx: &GraphQLContext,
        query_text: String,
        category: Option<String>,
    ) -> FieldResult<DiscoveryQueryData> {
        ctx.require_admin()?;
        let user = ctx.auth_user.as_ref().unwrap();

        let query = DiscoveryQuery::create(
            query_text,
            category,
            Some(user.member_id.into_uuid()),
            &ctx.db_pool,
        )
        .await
        .map_err(to_field_error)?;

        Ok(DiscoveryQueryData {
            id: query.id,
            query_text: query.query_text,
            category: query.category,
            is_active: query.is_active,
            created_at: query.created_at.to_rfc3339(),
        })
    }

    /// Update a discovery query (admin only)
    async fn update_discovery_query(
        ctx: &GraphQLContext,
        id: Uuid,
        query_text: String,
        category: Option<String>,
    ) -> FieldResult<DiscoveryQueryData> {
        ctx.require_admin()?;

        let query = DiscoveryQuery::update(id, query_text, category, &ctx.db_pool)
            .await
            .map_err(to_field_error)?;

        Ok(DiscoveryQueryData {
            id: query.id,
            query_text: query.query_text,
            category: query.category,
            is_active: query.is_active,
            created_at: query.created_at.to_rfc3339(),
        })
    }

    /// Toggle a discovery query active/inactive (admin only)
    async fn toggle_discovery_query(
        ctx: &GraphQLContext,
        id: Uuid,
        is_active: bool,
    ) -> FieldResult<DiscoveryQueryData> {
        ctx.require_admin()?;

        let query = DiscoveryQuery::toggle_active(id, is_active, &ctx.db_pool)
            .await
            .map_err(to_field_error)?;

        Ok(DiscoveryQueryData {
            id: query.id,
            query_text: query.query_text,
            category: query.category,
            is_active: query.is_active,
            created_at: query.created_at.to_rfc3339(),
        })
    }

    /// Delete a discovery query (admin only)
    async fn delete_discovery_query(ctx: &GraphQLContext, id: Uuid) -> FieldResult<bool> {
        ctx.require_admin()?;
        DiscoveryQuery::delete(id, &ctx.db_pool)
            .await
            .map_err(to_field_error)?;
        Ok(true)
    }

    /// Create a new filter rule (admin only). Pass queryId for per-query, null for global.
    async fn create_discovery_filter_rule(
        ctx: &GraphQLContext,
        query_id: Option<Uuid>,
        rule_text: String,
    ) -> FieldResult<DiscoveryFilterRuleData> {
        ctx.require_admin()?;
        let user = ctx.auth_user.as_ref().unwrap();

        let rule = DiscoveryFilterRule::create(
            query_id,
            rule_text,
            Some(user.member_id.into_uuid()),
            &ctx.db_pool,
        )
        .await
        .map_err(to_field_error)?;

        Ok(DiscoveryFilterRuleData {
            id: rule.id,
            query_id: rule.query_id,
            rule_text: rule.rule_text,
            sort_order: rule.sort_order,
            is_active: rule.is_active,
        })
    }

    /// Update a filter rule (admin only)
    async fn update_discovery_filter_rule(
        ctx: &GraphQLContext,
        id: Uuid,
        rule_text: String,
    ) -> FieldResult<DiscoveryFilterRuleData> {
        ctx.require_admin()?;

        let rule = DiscoveryFilterRule::update(id, rule_text, &ctx.db_pool)
            .await
            .map_err(to_field_error)?;

        Ok(DiscoveryFilterRuleData {
            id: rule.id,
            query_id: rule.query_id,
            rule_text: rule.rule_text,
            sort_order: rule.sort_order,
            is_active: rule.is_active,
        })
    }

    /// Delete a filter rule (admin only)
    async fn delete_discovery_filter_rule(ctx: &GraphQLContext, id: Uuid) -> FieldResult<bool> {
        ctx.require_admin()?;
        DiscoveryFilterRule::delete(id, &ctx.db_pool)
            .await
            .map_err(to_field_error)?;
        Ok(true)
    }

    /// Generate embedding for a single post (admin only)
    async fn generate_post_embedding(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
        use crate::domains::posts::effects::post_operations;

        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        if !user.is_admin {
            return Err(FieldError::new(
                "Admin authorization required",
                juniper::Value::null(),
            ));
        }

        info!(post_id = %post_id, "Generating post embedding");

        let post_id = PostId::from_uuid(post_id);
        post_operations::generate_post_embedding(
            post_id,
            ctx.server_deps.embedding_service.as_ref(),
            &ctx.db_pool,
        )
        .await
        .map_err(|e| {
            FieldError::new(
                format!("Failed to generate embedding: {}", e),
                juniper::Value::null(),
            )
        })?;

        Ok(true)
    }

    /// Backfill embeddings for posts that don't have them (admin only)
    ///
    /// Arguments:
    /// - limit: Maximum number of posts to process (default: 100)
    async fn backfill_post_embeddings(
        ctx: &GraphQLContext,
        limit: Option<i32>,
    ) -> FieldResult<BackfillPostEmbeddingsResult> {
        use crate::domains::posts::effects::post_operations;

        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        if !user.is_admin {
            return Err(FieldError::new(
                "Admin authorization required",
                juniper::Value::null(),
            ));
        }

        let limit = limit.unwrap_or(100);
        info!(limit = %limit, "Backfilling post embeddings");

        // Find posts without embeddings
        let posts = Post::find_without_embeddings(limit, &ctx.db_pool)
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to find posts: {}", e),
                    juniper::Value::null(),
                )
            })?;

        let mut processed = 0;
        let mut failed = 0;

        for post in posts {
            match post_operations::generate_post_embedding(
                post.id,
                ctx.server_deps.embedding_service.as_ref(),
                &ctx.db_pool,
            )
            .await
            {
                Ok(_) => processed += 1,
                Err(e) => {
                    error!(post_id = %post.id, error = %e, "Failed to generate embedding");
                    failed += 1;
                }
            }
        }

        // Count remaining posts without embeddings
        let remaining_posts = Post::find_without_embeddings(1, &ctx.db_pool)
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to count remaining: {}", e),
                    juniper::Value::null(),
                )
            })?;

        // Get actual count
        let remaining = if remaining_posts.is_empty() {
            0
        } else {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM posts WHERE embedding IS NULL AND deleted_at IS NULL AND status = 'active'"
            )
            .fetch_one(&ctx.db_pool)
            .await
            .unwrap_or(0) as i32
        };

        info!(
            processed = processed,
            failed = failed,
            remaining = remaining,
            "Backfill completed"
        );

        Ok(BackfillPostEmbeddingsResult {
            processed,
            failed,
            remaining,
        })
    }

    /// Backfill location records for existing posts that have location text but no post_locations (admin only)
    ///
    /// Parses existing `location` text field to extract city/state/zip and creates
    /// Location + PostLocation records.
    async fn backfill_post_locations(
        ctx: &GraphQLContext,
        batch_size: Option<i32>,
    ) -> FieldResult<BackfillPostLocationsResult> {
        use crate::domains::locations::models::Location;
        use crate::domains::posts::models::PostLocation;

        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        if !user.is_admin {
            return Err(FieldError::new(
                "Admin authorization required",
                juniper::Value::null(),
            ));
        }

        let batch_size = batch_size.unwrap_or(100).min(500) as i64;
        info!(batch_size = %batch_size, "Backfilling post locations");

        // Find active posts with location text but no post_locations record
        let posts = sqlx::query_as::<_, Post>(
            r#"
            SELECT p.* FROM posts p
            WHERE p.status = 'active'
              AND p.deleted_at IS NULL
              AND p.revision_of_post_id IS NULL
              AND p.location IS NOT NULL
              AND p.location != ''
              AND NOT EXISTS (
                  SELECT 1 FROM post_locations pl WHERE pl.post_id = p.id
              )
            ORDER BY p.created_at DESC
            LIMIT $1
            "#,
        )
        .bind(batch_size)
        .fetch_all(&ctx.db_pool)
        .await
        .map_err(|e| FieldError::new(format!("Failed to find posts: {}", e), juniper::Value::null()))?;

        let mut processed = 0;
        let mut failed = 0;

        for post in &posts {
            let location_text = post.location.as_deref().unwrap_or_default();

            // Try to parse zip code from location text (look for 5-digit pattern)
            let zip = extract_zip_from_text(location_text);
            let city = extract_city_from_text(location_text);

            if zip.is_none() && city.is_none() {
                failed += 1;
                continue;
            }

            match Location::find_or_create_from_extraction(
                city.as_deref(),
                Some("MN"),
                zip.as_deref(),
                None,
                &ctx.db_pool,
            )
            .await
            {
                Ok(loc) => {
                    if PostLocation::create(post.id, loc.id, true, None, &ctx.db_pool)
                        .await
                        .is_ok()
                    {
                        processed += 1;
                    } else {
                        failed += 1;
                    }
                }
                Err(_) => {
                    failed += 1;
                }
            }
        }

        let remaining = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM posts p
            WHERE p.status = 'active' AND p.deleted_at IS NULL AND p.revision_of_post_id IS NULL
              AND p.location IS NOT NULL AND p.location != ''
              AND NOT EXISTS (SELECT 1 FROM post_locations pl WHERE pl.post_id = p.id)
            "#,
        )
        .fetch_one(&ctx.db_pool)
        .await
        .unwrap_or(0) as i32;

        info!(processed, failed, remaining, "Post location backfill completed");

        Ok(BackfillPostLocationsResult {
            processed,
            failed,
            remaining,
        })
    }

    /// Deduplicate posts using embedding similarity (admin only)
    async fn deduplicate_posts(
        ctx: &GraphQLContext,
        _similarity_threshold: Option<f64>,
    ) -> FieldResult<DeduplicationResult> {
        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let event =
            post_activities::deduplicate_posts(user.member_id.into_uuid(), user.is_admin, ctx.deps())
                .await
                .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event)
            .await
            .map_err(to_field_error)?;

        // Deduplication is fire-and-forget, return placeholder result
        Ok(DeduplicationResult {
            job_id: Uuid::new_v4(),
            duplicates_found: 0,
            posts_merged: 0,
            posts_deleted: 0,
        })
    }

    // =========================================================================
    // Auth Mutations
    // =========================================================================

    /// Send OTP verification code via SMS
    async fn send_verification_code(
        ctx: &GraphQLContext,
        phone_number: String,
    ) -> FieldResult<bool> {
        use crate::domains::auth::workflows::{SendOtpRequest, OtpSent};

        let is_phone = phone_number.starts_with('+');
        let is_email = phone_number.contains('@');

        if !is_phone && !is_email {
            return Err(FieldError::new(
                "Must provide either phone number with country code (e.g., +1234567890) or email address",
                juniper::Value::null(),
            ));
        }

        // Invoke Restate workflow
        let result: OtpSent = ctx
            .workflow_client
            .invoke("SendOtp", "run", SendOtpRequest {
                phone_number: phone_number.clone(),
            })
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to send OTP");
                FieldError::new(e.to_string(), juniper::Value::null())
            })?;

        Ok(result.success)
    }

    /// Verify OTP code and create session
    async fn verify_code(
        ctx: &GraphQLContext,
        phone_number: String,
        code: String,
    ) -> FieldResult<String> {
        use crate::domains::auth::workflows::{VerifyOtpRequest, OtpVerified};

        // Invoke Restate workflow
        let result: OtpVerified = ctx
            .workflow_client
            .invoke("VerifyOtp", "run", VerifyOtpRequest {
                phone_number,
                code,
            })
            .await
            .map_err(to_field_error)?;

        Ok(result.token)
    }

    /// Logout (delete session)
    async fn logout(_ctx: &GraphQLContext, _session_token: String) -> FieldResult<bool> {
        // With JWT, logout is client-side only
        Ok(true)
    }

    // =========================================================================
    // Member Mutations
    // =========================================================================

    /// Register a new member (public)
    async fn register_member(
        ctx: &GraphQLContext,
        expo_push_token: String,
        searchable_text: String,
        city: String,
        state: String,
    ) -> FieldResult<MemberData> {
        info!(
            "register_member mutation called for city: {}, {}",
            city, state
        );

        let token_for_lookup = expo_push_token.clone();

        let event = member_activities::register_member(
            expo_push_token,
            searchable_text,
            city,
            state,
            ctx.deps(),
        )
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to register member");
            FieldError::new(e.to_string(), juniper::Value::null())
        })?;

        ctx.queue_engine
            .process(event)
            .await
            .map_err(to_field_error)?;

        // After process(), find member by expo_push_token
        let member = Member::find_by_token(&token_for_lookup, &ctx.db_pool)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to read member after registration");
                FieldError::new(e.to_string(), juniper::Value::null())
            })?
            .ok_or_else(|| {
                FieldError::new(
                    "Member not found after registration",
                    juniper::Value::null(),
                )
            })?;

        Ok(MemberData::from(member))
    }

    /// Update member status (activate/deactivate) (admin only)
    async fn update_member_status(
        ctx: &GraphQLContext,
        member_id: String,
        active: bool,
    ) -> FieldResult<MemberData> {
        ctx.require_admin()?;

        let member_id_uuid = Uuid::parse_str(&member_id)?;

        info!(
            "update_member_status mutation called: {} -> {}",
            member_id_uuid, active
        );

        let event = member_activities::update_member_status(member_id_uuid, active, ctx.deps())
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to update member status");
                FieldError::new(e.to_string(), juniper::Value::null())
            })?;

        ctx.queue_engine
            .process(event.clone())
            .await
            .map_err(to_field_error)?;

        // Extract member_id from event and fetch member data
        let MemberEvent::MemberStatusUpdated {
            member_id: returned_member_id,
            ..
        } = event
        else {
            return Err(FieldError::new(
                "Unexpected event type",
                juniper::Value::null(),
            ));
        };

        let member = Member::find_by_id(returned_member_id, &ctx.db_pool)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to read member after status update");
                FieldError::new(e.to_string(), juniper::Value::null())
            })?;

        Ok(MemberData::from(member))
    }

    // =========================================================================
    // Website Mutations
    // =========================================================================

    /// Approve a website for crawling (admin only)
    async fn approve_website(ctx: &GraphQLContext, website_id: String) -> FieldResult<WebsiteData> {
        info!(website_id = %website_id, "Approving website");

        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        if !user.is_admin {
            return Err(FieldError::new(
                "Admin authorization required",
                juniper::Value::null(),
            ));
        }

        let uuid = Uuid::parse_str(&website_id)
            .map_err(|_| FieldError::new("Invalid website ID", juniper::Value::null()))?;
        let id = WebsiteId::from_uuid(uuid);
        let requested_by = user.member_id;

        let event = website_activities::approve_website(id, requested_by, ctx.deps())
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to approve website: {}", e),
                    juniper::Value::null(),
                )
            })?;

        ctx.queue_engine
            .process(event.clone())
            .await
            .map_err(to_field_error)?;

        // Extract website_id from event and fetch updated website
        let WebsiteEvent::WebsiteApproved {
            website_id: returned_website_id,
            ..
        } = event
        else {
            return Err(FieldError::new(
                "Unexpected event type",
                juniper::Value::null(),
            ));
        };
        let website_id = returned_website_id;

        let website = Website::find_by_id(website_id, &ctx.db_pool)
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to fetch website: {}", e),
                    juniper::Value::null(),
                )
            })?;

        Ok(WebsiteData::from(website))
    }

    /// Reject a website submission (admin only)
    async fn reject_website(
        ctx: &GraphQLContext,
        website_id: String,
        reason: String,
    ) -> FieldResult<WebsiteData> {
        info!(website_id = %website_id, reason = %reason, "Rejecting website");

        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        if !user.is_admin {
            return Err(FieldError::new(
                "Admin authorization required",
                juniper::Value::null(),
            ));
        }

        let uuid = Uuid::parse_str(&website_id)
            .map_err(|_| FieldError::new("Invalid website ID", juniper::Value::null()))?;
        let id = WebsiteId::from_uuid(uuid);
        let requested_by = user.member_id;

        let event = website_activities::reject_website(id, reason, requested_by, ctx.deps())
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to reject website: {}", e),
                    juniper::Value::null(),
                )
            })?;

        ctx.queue_engine
            .process(event.clone())
            .await
            .map_err(to_field_error)?;

        // Extract website_id from event and fetch updated website
        let WebsiteEvent::WebsiteRejected {
            website_id: returned_website_id,
            ..
        } = event
        else {
            return Err(FieldError::new(
                "Unexpected event type",
                juniper::Value::null(),
            ));
        };
        let website_id = returned_website_id;

        let website = Website::find_by_id(website_id, &ctx.db_pool)
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to fetch website: {}", e),
                    juniper::Value::null(),
                )
            })?;

        Ok(WebsiteData::from(website))
    }

    /// Suspend a website (admin only)
    async fn suspend_website(
        ctx: &GraphQLContext,
        website_id: String,
        reason: String,
    ) -> FieldResult<WebsiteData> {
        info!(website_id = %website_id, reason = %reason, "Suspending website");

        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        if !user.is_admin {
            return Err(FieldError::new(
                "Admin authorization required",
                juniper::Value::null(),
            ));
        }

        let uuid = Uuid::parse_str(&website_id)
            .map_err(|_| FieldError::new("Invalid website ID", juniper::Value::null()))?;
        let id = WebsiteId::from_uuid(uuid);
        let requested_by = user.member_id;

        let event = website_activities::suspend_website(id, reason, requested_by, ctx.deps())
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to suspend website: {}", e),
                    juniper::Value::null(),
                )
            })?;

        ctx.queue_engine
            .process(event.clone())
            .await
            .map_err(to_field_error)?;

        // Extract website_id from event and fetch updated website
        let WebsiteEvent::WebsiteSuspended {
            website_id: returned_website_id,
            ..
        } = event
        else {
            return Err(FieldError::new(
                "Unexpected event type",
                juniper::Value::null(),
            ));
        };
        let website_id = returned_website_id;

        let website = Website::find_by_id(website_id, &ctx.db_pool)
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to fetch website: {}", e),
                    juniper::Value::null(),
                )
            })?;

        Ok(WebsiteData::from(website))
    }

    /// Update website crawl settings (admin only)
    async fn update_website_crawl_settings(
        ctx: &GraphQLContext,
        website_id: String,
        max_pages_per_crawl: i32,
    ) -> FieldResult<WebsiteData> {
        info!(website_id = %website_id, max_pages = %max_pages_per_crawl, "Updating crawl settings");

        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        if !user.is_admin {
            return Err(FieldError::new(
                "Admin authorization required",
                juniper::Value::null(),
            ));
        }

        if max_pages_per_crawl < 1 || max_pages_per_crawl > 100 {
            return Err(FieldError::new(
                "Max pages must be between 1 and 100",
                juniper::Value::null(),
            ));
        }

        let uuid = Uuid::parse_str(&website_id)
            .map_err(|_| FieldError::new("Invalid website ID", juniper::Value::null()))?;
        let id = WebsiteId::from_uuid(uuid);

        let event = website_activities::update_crawl_settings(id, max_pages_per_crawl, ctx.deps())
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to update settings: {}", e),
                    juniper::Value::null(),
                )
            })?;

        ctx.queue_engine
            .process(event.clone())
            .await
            .map_err(to_field_error)?;

        // Extract website_id from event and fetch updated website
        let WebsiteEvent::CrawlSettingsUpdated {
            website_id: returned_website_id,
            ..
        } = event
        else {
            return Err(FieldError::new(
                "Unexpected event type",
                juniper::Value::null(),
            ));
        };
        let website_id = returned_website_id;

        let website = Website::find_by_id(website_id, &ctx.db_pool)
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to fetch website: {}", e),
                    juniper::Value::null(),
                )
            })?;

        Ok(WebsiteData::from(website))
    }

    /// Regenerate posts from existing extraction pages (admin only)
    ///
    /// Emits a PostsRegenerationEnqueued event and returns immediately.
    /// The queued regenerate_posts_effect picks it up in the background.
    async fn regenerate_posts(
        ctx: &GraphQLContext,
        website_id: Uuid,
    ) -> FieldResult<ScrapeJobResult> {
        use crate::domains::crawling::events::CrawlEvent;

        info!(website_id = %website_id, "Emitting regenerate posts event");

        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let visitor_id = user.member_id.into_uuid();

        let handle = ctx
            .queue_engine
            .process(CrawlEvent::PostsRegenerationEnqueued {
                website_id,
                visitor_id,
            })
            .await
            .map_err(to_field_error)?;

        Ok(ScrapeJobResult {
            job_id: handle.correlation_id,
            source_id: website_id,
            status: "enqueued".to_string(),
            message: Some("Regeneration enqueued for background processing".to_string()),
        })
    }

    /// Regenerate a single post from its source extraction pages (admin only)
    ///
    /// Emits a SinglePostRegenerationEnqueued event and returns immediately.
    /// The queued regenerate_single_post_effect picks it up in the background.
    async fn regenerate_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<ScrapeJobResult> {
        use crate::domains::crawling::events::CrawlEvent;

        info!(post_id = %post_id, "Emitting regenerate single post event");

        let _user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let handle = ctx
            .queue_engine
            .process(CrawlEvent::SinglePostRegenerationEnqueued { post_id })
            .await
            .map_err(to_field_error)?;

        Ok(ScrapeJobResult {
            job_id: handle.correlation_id,
            source_id: post_id,
            status: "enqueued".to_string(),
            message: Some("Post regeneration enqueued for background processing".to_string()),
        })
    }

    /// Generate a comprehensive assessment report for a website (admin only)
    async fn generate_website_assessment(
        ctx: &GraphQLContext,
        website_id: String,
    ) -> FieldResult<String> {
        ctx.require_admin()?;

        info!(website_id = %website_id, "Generating website assessment");

        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let website_uuid = Uuid::parse_str(&website_id).map_err(|e| {
            FieldError::new(format!("Invalid website ID: {}", e), juniper::Value::null())
        })?;

        use crate::domains::website::events::approval::WebsiteApprovalEvent;

        let event = website_approval_activities::assess_website(
            website_uuid,
            user.member_id.into_uuid(),
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event.clone())
            .await
            .map_err(to_field_error)?;

        match event {
            WebsiteApprovalEvent::WebsiteAssessmentCompleted { assessment_id, .. } => {
                Ok(assessment_id.to_string())
            }
            WebsiteApprovalEvent::WebsiteResearchCreated { job_id, .. } => {
                // Assessment is still processing, return job_id
                Ok(job_id.to_string())
            }
            _ => Err(FieldError::new(
                "Unexpected event from assessment",
                juniper::Value::null(),
            )),
        }
    }

    // =========================================================================
    // Chatroom Mutations
    // =========================================================================

    /// Create a new AI chat container
    async fn create_chat(
        ctx: &GraphQLContext,
        language: Option<String>,
        with_agent: Option<String>,
    ) -> FieldResult<ContainerData> {
        let member_id = ctx.auth_user.as_ref().map(|u| u.member_id);
        let language = language.unwrap_or_else(|| "en".to_string());

        let event = chatroom_activities::create_container(
            language,
            member_id,
            with_agent,
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event.clone())
            .await
            .map_err(to_field_error)?;

        let ChatEvent::ContainerCreated { container, .. } = event else {
            return Err(FieldError::new(
                "Unexpected event type",
                juniper::Value::null(),
            ));
        };

        Ok(ContainerData::from(container))
    }

    /// Send a message to a chat container
    async fn send_message(
        ctx: &GraphQLContext,
        container_id: String,
        content: String,
    ) -> FieldResult<MessageData> {
        let member_id = ctx.auth_user.as_ref().map(|u| u.member_id);
        let container_id = ContainerId::parse(&container_id)?;

        let event =
            chatroom_activities::send_message(container_id, content, member_id, None, ctx.deps())
                .await
                .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event.clone())
            .await
            .map_err(to_field_error)?;

        let ChatEvent::MessageCreated { message } = event else {
            return Err(FieldError::new(
                "Unexpected event type",
                juniper::Value::null(),
            ));
        };

        Ok(MessageData::from(message))
    }

    /// Signal that the user is typing (for real-time indicators)
    async fn signal_typing(ctx: &GraphQLContext, container_id: String) -> FieldResult<bool> {
        use crate::domains::chatrooms::events::TypingEvent;

        let member_id = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?
            .member_id;

        let container_id = ContainerId::parse(&container_id)?;

        ctx.queue_engine
            .process(TypingEvent::Started {
                container_id,
                member_id,
            })
            .await
            .map_err(to_field_error)?;

        Ok(true)
    }

    // =========================================================================
    // Provider Mutations
    // =========================================================================

    /// Submit a new provider (public, goes to pending_review)
    async fn submit_provider(
        ctx: &GraphQLContext,
        input: SubmitProviderInput,
        member_id: Option<Uuid>,
    ) -> FieldResult<ProviderData> {
        info!("submit_provider mutation called: {}", input.name);

        let event = provider_activities::submit_provider(input, member_id, ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event.clone())
            .await
            .map_err(to_field_error)?;

        // Extract provider_id from event and fetch full provider data
        let ProviderEvent::ProviderCreated { provider_id, .. } = event else {
            return Err(FieldError::new(
                "Unexpected event type",
                juniper::Value::null(),
            ));
        };

        let provider = Provider::find_by_id(provider_id, &ctx.deps().db_pool)
            .await
            .map_err(to_field_error)?;

        Ok(ProviderData::from(provider))
    }

    /// Update a provider (admin only)
    /// Update a provider (admin only)
    async fn update_provider(
        ctx: &GraphQLContext,
        provider_id: String,
        input: UpdateProviderInput,
    ) -> FieldResult<ProviderData> {
        ctx.require_admin()?;
        info!("update_provider mutation called: {}", provider_id);

        let provider = provider_activities::update_provider(provider_id, input, ctx.deps())
            .await
            .map_err(to_field_error)?;

        Ok(ProviderData::from(provider))
    }

    /// Approve a provider (admin only)
    async fn approve_provider(
        ctx: &GraphQLContext,
        provider_id: String,
        reviewed_by_id: Uuid,
    ) -> FieldResult<ProviderData> {
        ctx.require_admin()?;
        info!("approve_provider mutation called: {}", provider_id);

        let event = provider_activities::approve_provider(provider_id, reviewed_by_id, ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event.clone())
            .await
            .map_err(to_field_error)?;

        // Extract provider_id from event and fetch full provider data
        let ProviderEvent::ProviderApproved { provider_id, .. } = event else {
            return Err(FieldError::new(
                "Unexpected event type",
                juniper::Value::null(),
            ));
        };

        let provider = Provider::find_by_id(provider_id, &ctx.deps().db_pool)
            .await
            .map_err(to_field_error)?;

        Ok(ProviderData::from(provider))
    }

    /// Reject a provider (admin only)
    async fn reject_provider(
        ctx: &GraphQLContext,
        provider_id: String,
        reason: String,
        reviewed_by_id: Uuid,
    ) -> FieldResult<ProviderData> {
        ctx.require_admin()?;
        info!("reject_provider mutation called: {}", provider_id);

        let event =
            provider_activities::reject_provider(provider_id, reason, reviewed_by_id, ctx.deps())
                .await
                .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event.clone())
            .await
            .map_err(to_field_error)?;

        // Extract provider_id from event and fetch full provider data
        let ProviderEvent::ProviderRejected { provider_id, .. } = event else {
            return Err(FieldError::new(
                "Unexpected event type",
                juniper::Value::null(),
            ));
        };

        let provider = Provider::find_by_id(provider_id, &ctx.deps().db_pool)
            .await
            .map_err(to_field_error)?;

        Ok(ProviderData::from(provider))
    }

    /// Add a tag to a provider (admin only)
    async fn add_provider_tag(
        ctx: &GraphQLContext,
        provider_id: String,
        tag_kind: String,
        tag_value: String,
        display_name: Option<String>,
    ) -> FieldResult<TagData> {
        ctx.require_admin()?;
        info!(
            "add_provider_tag mutation called: {} - {}:{}",
            provider_id, tag_kind, tag_value
        );

        let tag = provider_activities::add_provider_tag(
            provider_id,
            tag_kind,
            tag_value,
            display_name,
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        Ok(TagData::from(tag))
    }

    /// Remove a tag from a provider (admin only)
    async fn remove_provider_tag(
        ctx: &GraphQLContext,
        provider_id: String,
        tag_id: String,
    ) -> FieldResult<bool> {
        ctx.require_admin()?;
        info!(
            "remove_provider_tag mutation called: {} - {}",
            provider_id, tag_id
        );

        provider_activities::remove_provider_tag(provider_id, tag_id, ctx.deps())
            .await
            .map_err(to_field_error)?;

        Ok(true)
    }

    /// Delete a provider (admin only)
    async fn delete_provider(ctx: &GraphQLContext, provider_id: String) -> FieldResult<bool> {
        ctx.require_admin()?;
        info!("delete_provider mutation called: {}", provider_id);

        let event = provider_activities::delete_provider(provider_id, ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine
            .process(event)
            .await
            .map_err(to_field_error)?;

        Ok(true)
    }

    // =========================================================================
    // Listing Tags
    // =========================================================================

    /// Update listing tags (replaces all existing tags with new ones) (admin only)
    async fn update_post_tags(
        ctx: &GraphQLContext,
        post_id: Uuid,
        tags: Vec<TagInput>,
    ) -> FieldResult<PostType> {
        // Convert GraphQL TagInput to action TagInput
        let action_tags: Vec<post_activities::tags::TagInput> = tags
            .into_iter()
            .map(|t| post_activities::tags::TagInput {
                kind: t.kind,
                value: t.value,
            })
            .collect();

        ctx.require_admin()?;

        let post = post_activities::tags::update_post_tags(post_id, action_tags.clone(), &ctx.db_pool)
            .await
            .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))?;

        Ok(PostType::from(post))
    }

    /// Add a single tag to a listing (admin only)
    async fn add_post_tag(
        ctx: &GraphQLContext,
        post_id: Uuid,
        tag_kind: String,
        tag_value: String,
        display_name: Option<String>,
    ) -> FieldResult<TagData> {
        ctx.require_admin()?;

        let tag = post_activities::tags::add_post_tag(
            post_id,
            tag_kind.clone(),
            tag_value.clone(),
            display_name.clone(),
            &ctx.db_pool,
        )
        .await
        .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))?;

        Ok(TagData::from(tag))
    }

    /// Remove a tag from a listing (admin only)
    async fn remove_post_tag(
        ctx: &GraphQLContext,
        post_id: Uuid,
        tag_id: String,
    ) -> FieldResult<bool> {
        ctx.require_admin()?;

        post_activities::tags::remove_post_tag(post_id, tag_id.clone(), &ctx.db_pool)
            .await
            .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))
    }

    // =========================================================================
    // Extraction Mutations (Phase 15-17)
    // =========================================================================

    /// Submit a URL for extraction (public)
    ///
    /// Crawls the URL, extracts content, and returns extraction results.
    /// This is the main entry point for users to submit events, services, etc.
    async fn submit_url(
        ctx: &GraphQLContext,
        input: SubmitUrlInput,
    ) -> FieldResult<SubmitUrlResult> {
        info!(url = %input.url, "Submitting URL for extraction");

        let url = input.url.clone();
        let query = input.query.clone();

        match extraction_activities::submit_url(&url, query.as_deref(), ctx.deps()).await {
            Ok(extractions) => {
                let extraction = extractions.into_iter().next().map(ExtractionData::from);

                Ok(SubmitUrlResult {
                    success: true,
                    url: input.url,
                    extraction,
                    error: None,
                })
            }
            Err(e) => {
                error!(error = %e, "URL submission failed");
                Ok(SubmitUrlResult {
                    success: false,
                    url: input.url,
                    extraction: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Trigger an extraction query (admin only)
    ///
    /// Runs an extraction query against stored content.
    async fn trigger_extraction(
        ctx: &GraphQLContext,
        input: TriggerExtractionInput,
    ) -> FieldResult<TriggerExtractionResult> {
        // Admin check
        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        if !user.is_admin {
            return Err(FieldError::new(
                "Admin access required",
                juniper::Value::null(),
            ));
        }

        info!(query = %input.query, site = ?input.site, "Triggering extraction");

        let query = input.query.clone();
        let site = input.site.clone();

        match extraction_activities::trigger_extraction(&query, site.as_deref(), ctx.deps()).await {
            Ok(extractions) => Ok(TriggerExtractionResult {
                success: true,
                query: input.query,
                site: input.site,
                extractions: extractions.into_iter().map(ExtractionData::from).collect(),
                error: None,
            }),
            Err(e) => {
                error!(error = %e, "Extraction query failed");
                Ok(TriggerExtractionResult {
                    success: false,
                    query: input.query,
                    site: input.site,
                    extractions: vec![],
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Ingest a site for extraction (admin only)
    ///
    /// Crawls and indexes a site for future extraction queries.
    async fn ingest_site(
        ctx: &GraphQLContext,
        site_url: String,
        max_pages: Option<i32>,
    ) -> FieldResult<IngestSiteResult> {
        // Admin check
        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        if !user.is_admin {
            return Err(FieldError::new(
                "Admin access required",
                juniper::Value::null(),
            ));
        }

        info!(site_url = %site_url, max_pages = ?max_pages, "Ingesting site");

        let result = extraction_activities::ingest_site(&site_url, max_pages, ctx.deps())
            .await
            .map_err(to_field_error)?;

        Ok(IngestSiteResult {
            site_url: result.site_url,
            pages_crawled: result.pages_crawled,
            pages_summarized: result.pages_summarized,
            pages_skipped: result.pages_skipped,
        })
    }

    // =========================================================================
    // Sync Proposal Mutations
    // =========================================================================

    /// Approve a single sync proposal (admin only)
    async fn approve_sync_proposal(
        ctx: &GraphQLContext,
        proposal_id: Uuid,
    ) -> FieldResult<SyncProposalData> {
        ctx.require_admin()?;

        let user = ctx.auth_user.as_ref().unwrap();
        let reviewer = MemberId::from(user.member_id.into_uuid());
        let handler = PostProposalHandler;

        let proposal = proposal_actions::approve_proposal(
            SyncProposalId::from(proposal_id),
            reviewer,
            &handler,
            &ctx.db_pool,
        )
        .await
        .map_err(to_field_error)?;

        Ok(sync_proposal_to_data(proposal))
    }

    /// Reject a single sync proposal (admin only)
    async fn reject_sync_proposal(
        ctx: &GraphQLContext,
        proposal_id: Uuid,
    ) -> FieldResult<SyncProposalData> {
        ctx.require_admin()?;

        let user = ctx.auth_user.as_ref().unwrap();
        let reviewer = MemberId::from(user.member_id.into_uuid());
        let handler = PostProposalHandler;

        let proposal = proposal_actions::reject_proposal(
            SyncProposalId::from(proposal_id),
            reviewer,
            &handler,
            &ctx.db_pool,
        )
        .await
        .map_err(to_field_error)?;

        Ok(sync_proposal_to_data(proposal))
    }

    /// Approve all pending proposals in a batch (admin only)
    async fn approve_sync_batch(
        ctx: &GraphQLContext,
        batch_id: Uuid,
    ) -> FieldResult<SyncBatchData> {
        ctx.require_admin()?;

        let user = ctx.auth_user.as_ref().unwrap();
        let reviewer = MemberId::from(user.member_id.into_uuid());
        let handler = PostProposalHandler;

        let batch = proposal_actions::approve_batch(
            SyncBatchId::from(batch_id),
            reviewer,
            &handler,
            &ctx.db_pool,
        )
        .await
        .map_err(to_field_error)?;

        Ok(sync_batch_to_data(batch))
    }

    /// Reject all pending proposals in a batch (admin only)
    async fn reject_sync_batch(ctx: &GraphQLContext, batch_id: Uuid) -> FieldResult<SyncBatchData> {
        ctx.require_admin()?;

        let user = ctx.auth_user.as_ref().unwrap();
        let reviewer = MemberId::from(user.member_id.into_uuid());
        let handler = PostProposalHandler;

        let batch = proposal_actions::reject_batch(
            SyncBatchId::from(batch_id),
            reviewer,
            &handler,
            &ctx.db_pool,
        )
        .await
        .map_err(to_field_error)?;

        Ok(sync_batch_to_data(batch))
    }

    // =========================================================================
    // Schedule Mutations (admin only)
    // =========================================================================

    /// Add a schedule to a post (admin only)
    async fn add_post_schedule(
        ctx: &GraphQLContext,
        post_id: Uuid,
        input: ScheduleInput,
    ) -> FieldResult<ScheduleData> {
        ctx.require_admin()?;

        let timezone = input.timezone.as_deref().unwrap_or("America/Chicago");

        let schedule = if let Some(ref rrule) = input.rrule {
            // Recurring schedule
            let dtstart = input.dtstart
                .as_ref()
                .map(|s| s.parse::<chrono::DateTime<chrono::Utc>>())
                .transpose()
                .map_err(|e| FieldError::new(format!("Invalid dtstart: {}", e), juniper::Value::null()))?;

            let opens_at = input.opens_at
                .as_ref()
                .map(|s| chrono::NaiveTime::parse_from_str(s, "%H:%M"))
                .transpose()
                .map_err(|e| FieldError::new(format!("Invalid opens_at: {}", e), juniper::Value::null()))?;

            let closes_at = input.closes_at
                .as_ref()
                .map(|s| chrono::NaiveTime::parse_from_str(s, "%H:%M"))
                .transpose()
                .map_err(|e| FieldError::new(format!("Invalid closes_at: {}", e), juniper::Value::null()))?;

            Schedule::create_recurring(
                "post",
                post_id,
                dtstart.unwrap_or_else(chrono::Utc::now),
                rrule,
                input.duration_minutes,
                opens_at,
                closes_at,
                input.day_of_week,
                timezone,
                input.notes.as_deref(),
                &ctx.db_pool,
            )
            .await
            .map_err(to_field_error)?
        } else if input.day_of_week.is_some() && input.dtstart.is_none() {
            // Operating hours
            let opens_at = input.opens_at
                .as_ref()
                .map(|s| chrono::NaiveTime::parse_from_str(s, "%H:%M"))
                .transpose()
                .map_err(|e| FieldError::new(format!("Invalid opens_at: {}", e), juniper::Value::null()))?;

            let closes_at = input.closes_at
                .as_ref()
                .map(|s| chrono::NaiveTime::parse_from_str(s, "%H:%M"))
                .transpose()
                .map_err(|e| FieldError::new(format!("Invalid closes_at: {}", e), juniper::Value::null()))?;

            Schedule::create_operating_hours(
                "post",
                post_id,
                input.day_of_week.unwrap(),
                opens_at,
                closes_at,
                timezone,
                input.notes.as_deref(),
                &ctx.db_pool,
            )
            .await
            .map_err(to_field_error)?
        } else {
            // One-off event
            let dtstart = input.dtstart
                .as_ref()
                .ok_or_else(|| FieldError::new("dtstart is required for one-off events", juniper::Value::null()))?
                .parse::<chrono::DateTime<chrono::Utc>>()
                .map_err(|e| FieldError::new(format!("Invalid dtstart: {}", e), juniper::Value::null()))?;

            let dtend = input.dtend
                .as_ref()
                .ok_or_else(|| FieldError::new("dtend is required for one-off events", juniper::Value::null()))?
                .parse::<chrono::DateTime<chrono::Utc>>()
                .map_err(|e| FieldError::new(format!("Invalid dtend: {}", e), juniper::Value::null()))?;

            let is_all_day = input.is_all_day.unwrap_or(false);

            Schedule::create_one_off(
                "post",
                post_id,
                dtstart,
                dtend,
                is_all_day,
                timezone,
                input.notes.as_deref(),
                &ctx.db_pool,
            )
            .await
            .map_err(to_field_error)?
        };

        Ok(ScheduleData::from(schedule))
    }

    /// Update a schedule (admin only)
    async fn update_schedule(
        ctx: &GraphQLContext,
        schedule_id: Uuid,
        input: ScheduleInput,
    ) -> FieldResult<ScheduleData> {
        ctx.require_admin()?;

        let sid = ScheduleId::from(schedule_id);

        let dtstart = input.dtstart
            .as_ref()
            .map(|s| s.parse::<chrono::DateTime<chrono::Utc>>())
            .transpose()
            .map_err(|e| FieldError::new(format!("Invalid dtstart: {}", e), juniper::Value::null()))?;

        let dtend = input.dtend
            .as_ref()
            .map(|s| s.parse::<chrono::DateTime<chrono::Utc>>())
            .transpose()
            .map_err(|e| FieldError::new(format!("Invalid dtend: {}", e), juniper::Value::null()))?;

        let opens_at = input.opens_at
            .as_ref()
            .map(|s| chrono::NaiveTime::parse_from_str(s, "%H:%M"))
            .transpose()
            .map_err(|e| FieldError::new(format!("Invalid opens_at: {}", e), juniper::Value::null()))?;

        let closes_at = input.closes_at
            .as_ref()
            .map(|s| chrono::NaiveTime::parse_from_str(s, "%H:%M"))
            .transpose()
            .map_err(|e| FieldError::new(format!("Invalid closes_at: {}", e), juniper::Value::null()))?;

        let schedule = Schedule::update(
            sid,
            dtstart,
            dtend,
            input.rrule.as_deref(),
            input.exdates.as_deref(),
            opens_at,
            closes_at,
            input.day_of_week,
            input.is_all_day,
            input.duration_minutes,
            input.timezone.as_deref(),
            input.notes.as_deref(),
            &ctx.db_pool,
        )
        .await
        .map_err(to_field_error)?;

        Ok(ScheduleData::from(schedule))
    }

    /// Delete a schedule (admin only)
    async fn delete_schedule(
        ctx: &GraphQLContext,
        schedule_id: Uuid,
    ) -> FieldResult<bool> {
        ctx.require_admin()?;

        let sid = ScheduleId::from(schedule_id);
        Schedule::delete(sid, &ctx.db_pool)
            .await
            .map_err(to_field_error)?;

        Ok(true)
    }
}

/// Result of site ingestion
#[derive(Debug, Clone, juniper::GraphQLObject)]
pub struct IngestSiteResult {
    pub site_url: String,
    pub pages_crawled: i32,
    pub pages_summarized: i32,
    pub pages_skipped: i32,
}

/// A sync batch (group of AI-proposed changes)
#[derive(Debug, Clone, juniper::GraphQLObject)]
pub struct SyncBatchData {
    pub id: Uuid,
    pub resource_type: String,
    pub source_id: Option<Uuid>,
    pub status: String,
    pub summary: Option<String>,
    pub proposal_count: i32,
    pub approved_count: i32,
    pub rejected_count: i32,
    pub created_at: String,
    pub reviewed_at: Option<String>,
}

/// A single sync proposal (AI-proposed operation)
#[derive(Debug, Clone, juniper::GraphQLObject)]
pub struct SyncProposalData {
    pub id: Uuid,
    pub batch_id: Uuid,
    pub operation: String,
    pub status: String,
    pub entity_type: String,
    pub draft_entity_id: Option<Uuid>,
    pub target_entity_id: Option<Uuid>,
    pub reason: Option<String>,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<String>,
    pub created_at: String,
    /// Human-readable title of the draft entity (e.g. post title)
    pub draft_title: Option<String>,
    /// Human-readable title of the target entity
    pub target_title: Option<String>,
    /// For merge operations: IDs of entities being absorbed
    pub merge_source_ids: Vec<Uuid>,
    /// For merge operations: titles of entities being absorbed
    pub merge_source_titles: Vec<String>,
}

pub type Schema = RootNode<'static, Query, Mutation, EmptySubscription<GraphQLContext>>;

pub fn create_schema() -> Schema {
    Schema::new(Query, Mutation, EmptySubscription::new())
}
