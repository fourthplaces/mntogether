//! Posts service (stateless)
//!
//! Cross-post operations: list, search, submit, backfill, deduplicate.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::common::auth::restate_auth::{optional_auth, require_admin};
use crate::common::PaginationArgs;
use crate::domains::locations::models::ZipCode;
use crate::domains::posts::activities;
use crate::domains::posts::data::types::SubmitPostInput;
use crate::domains::posts::models::post_report::{PostReportRecord, PostReportWithDetails};
use crate::domains::posts::models::post::PostFilters;
use crate::domains::posts::models::Post;
use crate::domains::schedules::models::Schedule;
use crate::domains::tag::models::tag::Tag;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

use crate::domains::posts::restate::virtual_objects::post::{PostResult, PostTagResult};

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPostsRequest {
    pub status: Option<String>,
    pub source_type: Option<String>,
    pub source_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub search: Option<String>,
    pub zip_code: Option<String>,
    pub radius_miles: Option<f64>,
    pub first: Option<i32>,
    pub offset: Option<i32>,
    pub after: Option<String>,
    pub last: Option<i32>,
    pub before: Option<String>,
}

impl_restate_serde!(ListPostsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NearbySearchRequest {
    pub zip_code: String,
    pub radius_miles: Option<f64>,
    pub limit: Option<i32>,
}

impl_restate_serde!(NearbySearchRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchRequest {
    pub query: String,
    pub threshold: Option<f32>,
    pub limit: Option<i32>,
}

impl_restate_serde!(SemanticSearchRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitPostRequest {
    pub title: String,
    pub description: String,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub contact_website: Option<String>,
    pub urgency: Option<String>,
    pub location: Option<String>,
}

impl_restate_serde!(SubmitPostRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitResourceLinkRequest {
    pub url: String,
    pub submitter_contact: Option<String>,
}

impl_restate_serde!(SubmitResourceLinkRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillRequest {
    pub limit: Option<i32>,
}

impl_restate_serde!(BackfillRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicateRequest {
    pub similarity_threshold: Option<f64>,
}

impl_restate_serde!(DeduplicateRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpcomingEventsRequest {
    pub limit: Option<i32>,
}

impl_restate_serde!(UpcomingEventsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingRevisionsRequest {
    pub source_type: Option<String>,
    pub source_id: Option<Uuid>,
}

impl_restate_serde!(PendingRevisionsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListReportsRequest {
    pub status: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl_restate_serde!(ListReportsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicListRequest {
    pub post_type: Option<String>,
    pub category: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl_restate_serde!(PublicListRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPostsByOrganizationRequest {
    pub organization_id: Uuid,
}

impl_restate_serde!(ListPostsByOrganizationRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicFiltersRequest {}

impl_restate_serde!(PublicFiltersRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicateCrossSourceRequest {}

impl_restate_serde!(DeduplicateCrossSourceRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostStatsRequest {
    pub status: Option<String>,
}

impl_restate_serde!(PostStatsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchScorePostsRequest {
    pub limit: Option<i32>,
}

impl_restate_serde!(BatchScorePostsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillLocationsRequest {
    pub batch_size: Option<i32>,
}

impl_restate_serde!(BackfillLocationsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulesForEntityRequest {
    pub schedulable_type: String,
    pub schedulable_id: Uuid,
}

impl_restate_serde!(SchedulesForEntityRequest);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostListResult {
    pub posts: Vec<PostResult>,
    pub total_count: i32,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

impl_restate_serde!(PostListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NearbyPostResult {
    pub post: PostResult,
    pub distance_miles: f64,
}

impl_restate_serde!(NearbyPostResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NearbySearchResults {
    pub results: Vec<NearbyPostResult>,
}

impl_restate_serde!(NearbySearchResults);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub post_id: Uuid,
    pub title: String,
    pub description: String,
    pub category: String,
    pub post_type: String,
    pub similarity: f64,
}

impl_restate_serde!(SearchResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchResults {
    pub results: Vec<SearchResult>,
}

impl_restate_serde!(SemanticSearchResults);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingRevisionsResult {
    pub posts: Vec<PostResult>,
}

impl_restate_serde!(PendingRevisionsResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitPostResult {
    pub post_id: Uuid,
}

impl_restate_serde!(SubmitPostResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLinkSubmitResult {
    pub job_id: Uuid,
    pub status: String,
}

impl_restate_serde!(ResourceLinkSubmitResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillResult {
    pub processed: i32,
    pub failed: i32,
    pub remaining: i32,
}

impl_restate_serde!(BackfillResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicateResult {
    pub duplicates_found: i32,
    pub posts_merged: i32,
    pub posts_deleted: i32,
}

impl_restate_serde!(DeduplicateResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicateCrossSourceResult {
    pub batches_created: i32,
    pub total_proposals: i32,
    pub orgs_processed: i32,
}

impl_restate_serde!(DeduplicateCrossSourceResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportDetailResult {
    pub id: Uuid,
    pub post_id: Uuid,
    pub reason: String,
    pub category: String,
    pub status: String,
    pub reporter_email: Option<String>,
    pub resolution_notes: Option<String>,
}

impl_restate_serde!(ReportDetailResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportListResult {
    pub reports: Vec<ReportDetailResult>,
}

impl_restate_serde!(ReportListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPostResult {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub status: String,
    pub location: Option<String>,
    pub source_url: Option<String>,
}

impl_restate_serde!(EventPostResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpcomingEventsResult {
    pub events: Vec<EventPostResult>,
}

impl_restate_serde!(UpcomingEventsResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleResult {
    pub id: Uuid,
    pub schedulable_type: String,
    pub schedulable_id: Uuid,
    pub day_of_week: Option<i32>,
    pub opens_at: Option<String>,
    pub closes_at: Option<String>,
    pub timezone: String,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub notes: Option<String>,
    pub rrule: Option<String>,
}

impl_restate_serde!(ScheduleResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleListResult {
    pub schedules: Vec<ScheduleResult>,
}

impl_restate_serde!(ScheduleListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrgentNoteInfo {
    pub content: String,
    pub cta_text: Option<String>,
}

impl_restate_serde!(UrgentNoteInfo);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicPostResult {
    pub id: Uuid,
    pub title: String,
    pub summary: Option<String>,
    pub description: String,
    pub location: Option<String>,
    pub source_url: Option<String>,
    pub post_type: String,
    pub category: String,
    pub created_at: String,
    pub published_at: Option<String>,
    pub tags: Vec<PublicTagResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub urgent_notes: Vec<UrgentNoteInfo>,
}

impl_restate_serde!(PublicPostResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicTagResult {
    pub kind: String,
    pub value: String,
    pub display_name: Option<String>,
    pub color: Option<String>,
}

impl_restate_serde!(PublicTagResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicListResult {
    pub posts: Vec<PublicPostResult>,
    pub total_count: i32,
}

impl_restate_serde!(PublicListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterOption {
    pub value: String,
    pub display_name: String,
    pub count: i32,
}

impl_restate_serde!(FilterOption);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostTypeOption {
    pub value: String,
    pub display_name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub emoji: Option<String>,
}

impl_restate_serde!(PostTypeOption);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicFiltersResult {
    pub categories: Vec<FilterOption>,
    pub post_types: Vec<PostTypeOption>,
}

impl_restate_serde!(PublicFiltersResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostStatsResult {
    pub total: i64,
    pub services: i64,
    pub opportunities: i64,
    pub businesses: i64,
    pub user_submitted: i64,
    pub scraped: i64,
}

impl_restate_serde!(PostStatsResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillLocationsResult {
    pub processed: i32,
    pub failed: i32,
    pub remaining: i32,
}

impl_restate_serde!(BackfillLocationsResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchScorePostsResult {
    pub scored: i32,
    pub failed: i32,
    pub remaining: i32,
}

impl_restate_serde!(BatchScorePostsResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpireStalePostsRequest {}

impl_restate_serde!(ExpireStalePostsRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpireStalePostsResult {
    pub expired_count: u64,
}

impl_restate_serde!(ExpireStalePostsResult);

// =============================================================================
// Service definition
// =============================================================================

#[restate_sdk::service]
#[name = "Posts"]
pub trait PostsService {
    async fn list(req: ListPostsRequest) -> Result<PostListResult, HandlerError>;
    async fn search_nearby(req: NearbySearchRequest) -> Result<NearbySearchResults, HandlerError>;
    async fn search_semantic(req: SemanticSearchRequest) -> Result<SemanticSearchResults, HandlerError>;
    async fn submit(req: SubmitPostRequest) -> Result<SubmitPostResult, HandlerError>;
    async fn submit_resource_link(
        req: SubmitResourceLinkRequest,
    ) -> Result<ResourceLinkSubmitResult, HandlerError>;
    async fn backfill_embeddings(req: BackfillRequest) -> Result<BackfillResult, HandlerError>;
    async fn deduplicate(req: DeduplicateRequest) -> Result<DeduplicateResult, HandlerError>;
    async fn list_pending_revisions(
        req: PendingRevisionsRequest,
    ) -> Result<PendingRevisionsResult, HandlerError>;
    async fn list_reports(req: ListReportsRequest) -> Result<ReportListResult, HandlerError>;
    async fn upcoming_events(req: UpcomingEventsRequest) -> Result<UpcomingEventsResult, HandlerError>;
    async fn schedules_for_entity(
        req: SchedulesForEntityRequest,
    ) -> Result<ScheduleListResult, HandlerError>;
    async fn backfill_locations(
        req: BackfillLocationsRequest,
    ) -> Result<BackfillLocationsResult, HandlerError>;
    async fn list_by_organization(
        req: ListPostsByOrganizationRequest,
    ) -> Result<PostListResult, HandlerError>;
    async fn public_list(req: PublicListRequest) -> Result<PublicListResult, HandlerError>;
    async fn public_filters(req: PublicFiltersRequest) -> Result<PublicFiltersResult, HandlerError>;
    async fn deduplicate_cross_source(
        req: DeduplicateCrossSourceRequest,
    ) -> Result<DeduplicateCrossSourceResult, HandlerError>;
    async fn expire_stale_posts(
        req: ExpireStalePostsRequest,
    ) -> Result<ExpireStalePostsResult, HandlerError>;
    async fn stats(req: PostStatsRequest) -> Result<PostStatsResult, HandlerError>;
    async fn batch_score_posts(
        req: BatchScorePostsRequest,
    ) -> Result<BatchScorePostsResult, HandlerError>;
}

// =============================================================================
// Implementation
// =============================================================================

pub struct PostsServiceImpl {
    deps: Arc<ServerDeps>,
}

impl PostsServiceImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl PostsService for PostsServiceImpl {
    async fn list(
        &self,
        _ctx: Context<'_>,
        req: ListPostsRequest,
    ) -> Result<PostListResult, HandlerError> {
        let filters = PostFilters {
            status: req.status.as_deref(),
            source_type: req.source_type.as_deref(),
            source_id: req.source_id,
            agent_id: req.agent_id,
            search: req.search.as_deref(),
        };

        // Branch: zip-based proximity filtering vs standard listing
        if let Some(ref zip_code) = req.zip_code {
            let radius = req.radius_miles.unwrap_or(25.0).min(100.0);
            let limit = req.first.unwrap_or(20);
            let offset = req.offset.unwrap_or(0);

            let (results, total_count, has_more) = activities::get_posts_near_zip(
                zip_code,
                radius,
                &filters,
                limit,
                offset,
                &self.deps,
            )
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

            // Batch-load tags for returned posts
            let post_ids: Vec<uuid::Uuid> = results.iter().map(|r| r.id.into_uuid()).collect();
            let tag_rows = Tag::find_for_post_ids(&post_ids, &self.deps.db_pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

            let mut tags_by_post: std::collections::HashMap<uuid::Uuid, Vec<PostTagResult>> =
                std::collections::HashMap::new();
            for row in tag_rows {
                tags_by_post
                    .entry(row.taggable_id)
                    .or_default()
                    .push(PostTagResult {
                        id: row.tag.id.into_uuid(),
                        kind: row.tag.kind,
                        value: row.tag.value,
                        display_name: row.tag.display_name,
                        color: row.tag.color,
                    });
            }

            Ok(PostListResult {
                posts: results
                    .into_iter()
                    .map(|pwd| {
                        let id = pwd.id.into_uuid();
                        PostResult {
                            id,
                            title: pwd.title,
                            description: pwd.description,
                            description_markdown: pwd.description_markdown,
                            summary: pwd.summary,
                            status: pwd.status,
                            post_type: pwd.post_type,
                            category: pwd.category,
                            urgency: pwd.urgency,
                            location: pwd.location,
                            source_url: pwd.source_url,
                            submission_type: pwd.submission_type,
                            created_at: pwd.created_at.to_rfc3339(),
                            updated_at: pwd.updated_at.to_rfc3339(),
                            published_at: pwd.published_at.map(|dt| dt.to_rfc3339()),
                            tags: Some(tags_by_post.remove(&id).unwrap_or_default()),
                            submitted_by: None,
                            schedules: None,
                            contacts: None,
                            organization_id: None,
                            organization_name: None,
                            has_urgent_notes: None,
                            urgent_notes: None,
                            distance_miles: Some(pwd.distance_miles),
                            relevance_score: None,
                            relevance_breakdown: None,
                        }
                    })
                    .collect(),
                total_count,
                has_next_page: has_more,
                has_previous_page: offset > 0,
            })
        } else {
            // Standard listing path (unchanged)
            let pagination_args = PaginationArgs {
                first: req.first,
                after: req.after,
                last: req.last,
                before: req.before,
            };
            let validated = pagination_args
                .validate()
                .map_err(|e| TerminalError::new(e))?;

            let connection =
                activities::get_posts_paginated(&filters, &validated, &self.deps)
                    .await
                    .map_err(|e| TerminalError::new(e.to_string()))?;

            // Batch-load tags for returned posts
            let post_ids: Vec<uuid::Uuid> = connection.edges.iter().map(|e| e.node.id).collect();
            let tag_rows = Tag::find_for_post_ids(&post_ids, &self.deps.db_pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

            let mut tags_by_post: std::collections::HashMap<uuid::Uuid, Vec<PostTagResult>> =
                std::collections::HashMap::new();
            for row in tag_rows {
                tags_by_post
                    .entry(row.taggable_id)
                    .or_default()
                    .push(PostTagResult {
                        id: row.tag.id.into_uuid(),
                        kind: row.tag.kind,
                        value: row.tag.value,
                        display_name: row.tag.display_name,
                        color: row.tag.color,
                    });
            }

            Ok(PostListResult {
                posts: connection
                    .edges
                    .into_iter()
                    .map(|e| {
                        let id = e.node.id;
                        PostResult {
                            id,
                            title: e.node.title,
                            description: e.node.description,
                            description_markdown: e.node.description_markdown,
                            summary: e.node.summary,
                            status: format!("{:?}", e.node.status),
                            post_type: e.node.post_type,
                            category: e.node.category,
                            urgency: e.node.urgency,
                            location: e.node.location,
                            source_url: e.node.source_url,
                            submission_type: e.node.submission_type,
                            created_at: e.node.created_at.to_rfc3339(),
                            updated_at: e.node.created_at.to_rfc3339(),
                            published_at: e.node.published_at.map(|dt| dt.to_rfc3339()),
                            tags: Some(tags_by_post.remove(&id).unwrap_or_default()),
                            submitted_by: None,
                            schedules: None,
                            contacts: None,
                            organization_id: None,
                            organization_name: None,
                            has_urgent_notes: None,
                            urgent_notes: None,
                            distance_miles: None,
                            relevance_score: None,
                            relevance_breakdown: None,
                        }
                    })
                    .collect(),
                total_count: connection.total_count,
                has_next_page: connection.page_info.has_next_page,
                has_previous_page: connection.page_info.has_previous_page,
            })
        }
    }

    async fn search_nearby(
        &self,
        _ctx: Context<'_>,
        req: NearbySearchRequest,
    ) -> Result<NearbySearchResults, HandlerError> {
        let radius = req.radius_miles.unwrap_or(25.0).min(100.0);
        let limit = req.limit.unwrap_or(50).min(200);

        // Validate zip exists
        let _center = ZipCode::find_by_code(&req.zip_code, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
            .ok_or_else(|| {
                TerminalError::new(format!("Zip code '{}' not found", req.zip_code))
            })?;

        let results = Post::find_near_zip(&req.zip_code, radius, limit, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(NearbySearchResults {
            results: results
                .into_iter()
                .map(|pwd| NearbyPostResult {
                    post: PostResult {
                        id: pwd.id.into_uuid(),
                        title: pwd.title,
                        description: pwd.description,
                        description_markdown: pwd.description_markdown,
                        summary: pwd.summary,
                        status: pwd.status,
                        post_type: pwd.post_type,
                        category: pwd.category,
                        urgency: pwd.urgency,
                        location: pwd.location,
                        source_url: pwd.source_url,
                        submission_type: pwd.submission_type,
                        created_at: pwd.created_at.to_rfc3339(),
                        updated_at: pwd.updated_at.to_rfc3339(),
                        published_at: pwd.published_at.map(|dt| dt.to_rfc3339()),
                        tags: None,
                        submitted_by: None,
                        schedules: None,
                        contacts: None,
                        organization_id: None,
                        organization_name: None,
                        has_urgent_notes: None,
                        urgent_notes: None,
                        distance_miles: None,
                            relevance_score: None,
                            relevance_breakdown: None,
                    },
                    distance_miles: pwd.distance_miles,
                })
                .collect(),
        })
    }

    async fn search_semantic(
        &self,
        _ctx: Context<'_>,
        req: SemanticSearchRequest,
    ) -> Result<SemanticSearchResults, HandlerError> {
        let threshold = req.threshold.unwrap_or(0.5);
        let limit = req.limit.unwrap_or(20);

        let results =
            activities::search::search_posts_semantic(&req.query, threshold, limit, &self.deps)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(SemanticSearchResults {
            results: results
                .into_iter()
                .map(|r| SearchResult {
                    post_id: r.post_id.into_uuid(),
                    title: r.title,
                    description: r.description,
                    category: r.category,
                    post_type: r.post_type,
                    similarity: r.similarity,
                })
                .collect(),
        })
    }

    async fn submit(
        &self,
        ctx: Context<'_>,
        req: SubmitPostRequest,
    ) -> Result<SubmitPostResult, HandlerError> {
        let user = optional_auth(ctx.headers(), &self.deps.jwt_service);

        use crate::domains::posts::data::types::ContactInfoInput;
        let contact_info = if req.contact_phone.is_some()
            || req.contact_email.is_some()
            || req.contact_website.is_some()
        {
            Some(ContactInfoInput {
                phone: req.contact_phone,
                email: req.contact_email,
                website: req.contact_website,
            })
        } else {
            None
        };

        let input = SubmitPostInput {
            title: req.title,
            description: req.description,
            contact_info,
            urgency: req.urgency,
            location: req.location,
        };

        let result = ctx
            .run(|| async {
                let post_id = activities::submit_post(
                    input.clone(),
                    user.as_ref().map(|u| u.member_id.into_uuid()),
                    &self.deps,
                )
                .await
                .map_err(Into::<restate_sdk::errors::HandlerError>::into)?;

                Ok(SubmitPostResult {
                    post_id: post_id.into_uuid(),
                })
            })
            .await?;

        Ok(result)
    }

    async fn submit_resource_link(
        &self,
        ctx: Context<'_>,
        req: SubmitResourceLinkRequest,
    ) -> Result<ResourceLinkSubmitResult, HandlerError> {
        let result = ctx
            .run(|| async {
                use crate::domains::posts::activities::scraping::ResourceLinkSubmission;
                let submission = activities::scraping::submit_resource_link(
                    req.url.clone(),
                    req.submitter_contact.clone(),
                    &self.deps,
                )
                .await
                .map_err(Into::<restate_sdk::errors::HandlerError>::into)?;

                match submission {
                    ResourceLinkSubmission::PendingApproval { .. } => Ok(ResourceLinkSubmitResult {
                        job_id: Uuid::nil(),
                        status: "pending_approval".to_string(),
                    }),
                    ResourceLinkSubmission::Processing { job_id, .. } => {
                        Ok(ResourceLinkSubmitResult {
                            job_id: job_id.into_uuid(),
                            status: "processing".to_string(),
                        })
                    }
                }
            })
            .await?;

        Ok(result)
    }

    async fn backfill_embeddings(
        &self,
        ctx: Context<'_>,
        req: BackfillRequest,
    ) -> Result<BackfillResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let limit = req.limit.unwrap_or(100);

        let result = ctx
            .run(|| async {
                let r = activities::backfill::backfill_post_embeddings(limit, &self.deps)
                    .await
                    .map_err(Into::<restate_sdk::errors::HandlerError>::into)?;

                Ok(BackfillResult {
                    processed: r.processed,
                    failed: r.failed,
                    remaining: r.remaining,
                })
            })
            .await?;

        Ok(result)
    }

    async fn deduplicate(
        &self,
        ctx: Context<'_>,
        _req: DeduplicateRequest,
    ) -> Result<DeduplicateResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let result = ctx
            .run(|| async {
                let r = activities::deduplicate_posts(
                    user.member_id.into_uuid(),
                    user.is_admin,
                    &self.deps,
                )
                .await
                .map_err(Into::<restate_sdk::errors::HandlerError>::into)?;

                Ok(DeduplicateResult {
                    duplicates_found: r.duplicates_found as i32,
                    posts_merged: r.posts_merged as i32,
                    posts_deleted: r.posts_deleted as i32,
                })
            })
            .await?;

        Ok(result)
    }

    async fn list_pending_revisions(
        &self,
        ctx: Context<'_>,
        req: PendingRevisionsRequest,
    ) -> Result<PendingRevisionsResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let source_filter = match (req.source_type.as_deref(), req.source_id) {
            (Some(st), Some(sid)) => Some((st, sid)),
            _ => None,
        };

        let revisions =
            activities::revision_actions::get_pending_revisions(source_filter, &self.deps.db_pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(PendingRevisionsResult {
            posts: revisions.into_iter().map(PostResult::from).collect(),
        })
    }

    async fn list_reports(
        &self,
        ctx: Context<'_>,
        req: ListReportsRequest,
    ) -> Result<ReportListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let limit = req.limit.unwrap_or(50) as i64;
        let offset = req.offset.unwrap_or(0) as i64;

        let reports: Vec<PostReportWithDetails> = match req.status.as_deref() {
            Some("pending") | None => {
                PostReportRecord::query_pending(limit, offset, &self.deps.db_pool).await
            }
            _ => PostReportRecord::query_all(limit, offset, &self.deps.db_pool).await,
        }
        .map_err(|e: anyhow::Error| TerminalError::new(e.to_string()))?;

        Ok(ReportListResult {
            reports: reports
                .into_iter()
                .map(|r| ReportDetailResult {
                    id: r.id.into_uuid(),
                    post_id: r.post_id.into_uuid(),
                    reason: r.reason,
                    category: r.category,
                    status: r.status,
                    reporter_email: None,
                    resolution_notes: r.resolution_notes,
                })
                .collect(),
        })
    }

    async fn upcoming_events(
        &self,
        _ctx: Context<'_>,
        req: UpcomingEventsRequest,
    ) -> Result<UpcomingEventsResult, HandlerError> {
        let limit = req.limit.unwrap_or(20).min(100) as usize;

        let events = activities::upcoming_events::get_upcoming_events(limit, &self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(UpcomingEventsResult {
            events: events
                .into_iter()
                .map(|e| EventPostResult {
                    id: e.id,
                    title: e.title,
                    description: e.description,
                    status: format!("{:?}", e.status),
                    location: e.location,
                    source_url: e.source_url,
                })
                .collect(),
        })
    }

    async fn schedules_for_entity(
        &self,
        _ctx: Context<'_>,
        req: SchedulesForEntityRequest,
    ) -> Result<ScheduleListResult, HandlerError> {
        let schedules = Schedule::find_for_entity(
            &req.schedulable_type,
            req.schedulable_id,
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(ScheduleListResult {
            schedules: schedules
                .into_iter()
                .map(|s| ScheduleResult {
                    id: s.id.into_uuid(),
                    schedulable_type: s.schedulable_type,
                    schedulable_id: s.schedulable_id,
                    day_of_week: s.day_of_week,
                    opens_at: s.opens_at.map(|t| t.to_string()),
                    closes_at: s.closes_at.map(|t| t.to_string()),
                    timezone: s.timezone,
                    valid_from: s.valid_from.map(|d| d.to_string()),
                    valid_to: s.valid_to.map(|d| d.to_string()),
                    notes: s.notes,
                    rrule: s.rrule,
                })
                .collect(),
        })
    }

    async fn backfill_locations(
        &self,
        ctx: Context<'_>,
        req: BackfillLocationsRequest,
    ) -> Result<BackfillLocationsResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let batch_size = req.batch_size.unwrap_or(100).min(500) as i64;

        let result = ctx
            .run(|| async {
                let r = activities::backfill::backfill_post_locations(batch_size, &self.deps)
                    .await
                    .map_err(Into::<restate_sdk::errors::HandlerError>::into)?;

                Ok(BackfillLocationsResult {
                    processed: r.processed,
                    failed: r.failed,
                    remaining: r.remaining,
                })
            })
            .await?;

        Ok(result)
    }

    async fn list_by_organization(
        &self,
        ctx: Context<'_>,
        req: ListPostsByOrganizationRequest,
    ) -> Result<PostListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let posts = Post::find_all_by_organization_id(req.organization_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let post_ids: Vec<uuid::Uuid> = posts.iter().map(|p| p.id.into_uuid()).collect();
        let tag_rows = Tag::find_for_post_ids(&post_ids, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let mut tags_by_post: std::collections::HashMap<uuid::Uuid, Vec<PostTagResult>> =
            std::collections::HashMap::new();
        for row in tag_rows {
            tags_by_post
                .entry(row.taggable_id)
                .or_default()
                .push(PostTagResult {
                    id: row.tag.id.into_uuid(),
                    kind: row.tag.kind,
                    value: row.tag.value,
                    display_name: row.tag.display_name,
                    color: row.tag.color,
                });
        }

        let total_count = posts.len() as i32;

        Ok(PostListResult {
            posts: posts
                .into_iter()
                .map(|p| {
                    let id = p.id.into_uuid();
                    PostResult {
                        id,
                        title: p.title,
                        description: p.description,
                        description_markdown: p.description_markdown,
                        summary: p.summary,
                        status: p.status,
                        post_type: p.post_type,
                        category: p.category,
                        urgency: p.urgency,
                        location: p.location,
                        source_url: p.source_url,
                        submission_type: p.submission_type,
                        created_at: p.created_at.to_rfc3339(),
                        updated_at: p.updated_at.to_rfc3339(),
                        published_at: p.published_at.map(|dt| dt.to_rfc3339()),
                        tags: Some(tags_by_post.remove(&id).unwrap_or_default()),
                        submitted_by: None,
                        schedules: None,
                        contacts: None,
                        organization_id: None,
                        organization_name: None,
                        has_urgent_notes: None,
                        urgent_notes: None,
                        distance_miles: None,
                        relevance_score: p.relevance_score,
                        relevance_breakdown: p.relevance_breakdown,
                    }
                })
                .collect(),
            total_count,
            has_next_page: false,
            has_previous_page: false,
        })
    }

    async fn public_list(
        &self,
        _ctx: Context<'_>,
        req: PublicListRequest,
    ) -> Result<PublicListResult, HandlerError> {
        let limit = req.limit.unwrap_or(50).min(200) as i64;
        let offset = req.offset.unwrap_or(0) as i64;
        let post_type = req.post_type.as_deref();
        let category = req.category.as_deref();

        let posts = Post::find_public_filtered(post_type, category, limit, offset, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let total_count = Post::count_public_filtered(post_type, category, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        // Batch-load public tags for returned posts
        let post_ids: Vec<uuid::Uuid> = posts.iter().map(|p| p.id.into_uuid()).collect();
        let tag_rows = Tag::find_public_for_post_ids(&post_ids, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        // Batch-load urgent note content
        use crate::domains::notes::models::note::Note;
        let urgent_rows = Note::find_urgent_note_content_for_posts(&post_ids, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;
        let mut urgent_notes_by_post: std::collections::HashMap<uuid::Uuid, Vec<UrgentNoteInfo>> =
            std::collections::HashMap::new();
        for (post_id, content, cta_text) in urgent_rows {
            urgent_notes_by_post.entry(post_id).or_default().push(UrgentNoteInfo { content, cta_text });
        }

        // Group tags by post id
        let mut tags_by_post: std::collections::HashMap<uuid::Uuid, Vec<PublicTagResult>> =
            std::collections::HashMap::new();
        for row in tag_rows {
            tags_by_post
                .entry(row.taggable_id)
                .or_default()
                .push(PublicTagResult {
                    kind: row.tag.kind,
                    value: row.tag.value,
                    display_name: row.tag.display_name,
                    color: row.tag.color,
                });
        }

        Ok(PublicListResult {
            posts: posts
                .into_iter()
                .map(|p| {
                    let id = p.id.into_uuid();
                    PublicPostResult {
                        id,
                        title: p.title,
                        summary: p.summary,
                        description: p.description,
                        location: p.location,
                        source_url: p.source_url,
                        post_type: p.post_type,
                        category: p.category,
                        created_at: p.created_at.to_rfc3339(),
                        published_at: p.published_at.map(|dt| dt.to_rfc3339()),
                        tags: tags_by_post.remove(&id).unwrap_or_default(),
                        urgent_notes: urgent_notes_by_post.remove(&id).unwrap_or_default(),
                    }
                })
                .collect(),
            total_count: total_count as i32,
        })
    }

    async fn public_filters(
        &self,
        _ctx: Context<'_>,
        _req: PublicFiltersRequest,
    ) -> Result<PublicFiltersResult, HandlerError> {
        let categories = Tag::find_active_categories(&self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let post_types = Tag::find_post_types(&self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(PublicFiltersResult {
            categories: categories
                .into_iter()
                .map(|c| FilterOption {
                    value: c.value,
                    display_name: c.display_name,
                    count: c.count,
                })
                .collect(),
            post_types: post_types
                .into_iter()
                .map(|t| PostTypeOption {
                    value: t.value,
                    display_name: t.display_name.unwrap_or_default(),
                    description: t.description,
                    color: t.color,
                    emoji: t.emoji,
                })
                .collect(),
        })
    }

    async fn deduplicate_cross_source(
        &self,
        ctx: Context<'_>,
        _req: DeduplicateCrossSourceRequest,
    ) -> Result<DeduplicateCrossSourceResult, HandlerError> {
        let user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let result = ctx
            .run(|| async {
                let r = activities::deduplicate_cross_source_all_orgs(
                    user.member_id.into_uuid(),
                    user.is_admin,
                    &self.deps,
                )
                .await
                .map_err(Into::<restate_sdk::errors::HandlerError>::into)?;

                Ok(DeduplicateCrossSourceResult {
                    batches_created: r.batches_created as i32,
                    total_proposals: r.total_proposals as i32,
                    orgs_processed: r.orgs_processed as i32,
                })
            })
            .await?;

        Ok(result)
    }

    async fn expire_stale_posts(
        &self,
        ctx: Context<'_>,
        _req: ExpireStalePostsRequest,
    ) -> Result<ExpireStalePostsResult, HandlerError> {
        let expired_count = ctx
            .run(|| async {
                activities::expire_scheduled_posts::expire_scheduled_posts(&self.deps)
                    .await
                    .map_err(Into::<restate_sdk::errors::HandlerError>::into)
            })
            .await?;

        Ok(ExpireStalePostsResult { expired_count })
    }

    async fn stats(
        &self,
        _ctx: Context<'_>,
        req: PostStatsRequest,
    ) -> Result<PostStatsResult, HandlerError> {
        let rows = Post::stats_by_status(req.status.as_deref(), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let mut total: i64 = 0;
        let mut services: i64 = 0;
        let mut opportunities: i64 = 0;
        let mut businesses: i64 = 0;
        let mut scraped: i64 = 0;
        let mut user_submitted: i64 = 0;

        for (post_type, submission_type, count) in &rows {
            total += count;

            match post_type.as_deref() {
                Some("service") => services += count,
                Some("opportunity") => opportunities += count,
                Some("business") => businesses += count,
                _ => {}
            }

            match submission_type.as_deref() {
                Some("scraped") => scraped += count,
                _ => user_submitted += count,
            }
        }

        Ok(PostStatsResult {
            total,
            services,
            opportunities,
            businesses,
            user_submitted,
            scraped,
        })
    }

    async fn batch_score_posts(
        &self,
        ctx: Context<'_>,
        req: BatchScorePostsRequest,
    ) -> Result<BatchScorePostsResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let limit = req.limit.unwrap_or(50).min(200);

        let result = ctx
            .run(|| async {
                let unscored = Post::find_unscored_active(&self.deps.db_pool)
                    .await
                    .map_err(Into::<restate_sdk::errors::HandlerError>::into)?;

                let total_remaining = unscored.len() as i32;
                let batch: Vec<_> = unscored.into_iter().take(limit as usize).collect();

                let mut scored = 0i32;
                let mut failed = 0i32;

                for post in batch {
                    match activities::score_post_by_id(
                        post.id,
                        &self.deps.ai,
                        &self.deps.db_pool,
                    )
                    .await
                    {
                        Some(_) => scored += 1,
                        None => failed += 1,
                    }
                }

                let remaining = (total_remaining - scored - failed).max(0);

                Ok(BatchScorePostsResult {
                    scored,
                    failed,
                    remaining,
                })
            })
            .await?;

        Ok(result)
    }
}
