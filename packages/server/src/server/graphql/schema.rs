//! GraphQL schema definition.
//!
//! # Deprecation Note
//!
//! Some queries use deprecated `PageSnapshot` model. See the crawling domain
//! models for migration paths to extraction library types.

#![allow(deprecated)] // Uses deprecated PageSnapshot during migration

use super::context::GraphQLContext;
use anyhow::Context as AnyhowContext;
use juniper::{EmptySubscription, FieldError, FieldResult, RootNode};
use sqlx::PgPool;
use tracing::{error, info};
use uuid::Uuid;

// Common types
use crate::common::{ContainerId, PaginationArgs, PostId, WebsiteId};

// Domain actions
use crate::domains::auth::actions as auth_actions;
use crate::domains::chatrooms::actions as chatroom_actions;
use crate::domains::crawling::actions as crawling_actions;
use crate::domains::extraction::actions as extraction_actions;
use crate::domains::member::actions as member_actions;
use crate::domains::organization::actions as organization_actions;
use crate::domains::posts::actions as post_actions;
use crate::domains::providers::actions as provider_actions;
use crate::domains::website::actions as website_actions;
use crate::domains::website_approval::actions as website_approval_actions;

// Domain data types (GraphQL types)
use crate::domains::chatrooms::data::{ContainerData, MessageData};
use crate::domains::extraction::data::{
    ExtractionData, SubmitUrlInput, SubmitUrlResult, TriggerExtractionInput,
    TriggerExtractionResult,
};
use crate::domains::member::data::{MemberConnection, MemberData};
use crate::domains::organization::data::{OrganizationConnection, OrganizationData};
use crate::domains::posts::data::post_report::{
    PostReport as PostReportData, PostReportDetail as PostReportDetailData,
};
use crate::domains::posts::data::types::RepostResult;
use crate::domains::posts::data::PostData;
use crate::domains::posts::data::{
    BusinessInfo, EditPostInput, PostConnection, PostStatusData, PostType, ScrapeJobResult,
    SubmitPostInput, SubmitResourceLinkInput, SubmitResourceLinkResult,
};
use crate::domains::providers::data::{
    ProviderConnection, ProviderData, SubmitProviderInput, UpdateProviderInput,
};
use crate::domains::website::data::{PageSnapshotData, WebsiteConnection, WebsiteData};
use crate::domains::website_approval::data::{WebsiteAssessmentData, WebsiteSearchResultData};

// Domain models (for queries)
use crate::domains::chatrooms::models::{Container, Message};
use crate::domains::contacts::ContactData;
use crate::domains::crawling::models::PageSnapshot;
use crate::domains::member::models::member::Member;
use crate::domains::organization::models::Organization;
use crate::domains::posts::models::post_report::PostReportRecord;
use crate::domains::posts::models::{BusinessPost, Post};
use crate::domains::tag::{Tag, TagData, Taggable};
use crate::domains::website::models::{Website, WebsiteAssessment};

#[derive(juniper::GraphQLInputObject)]
pub struct TagInput {
    pub kind: String,
    pub value: String,
}

#[derive(juniper::GraphQLObject)]
#[graphql(context = GraphQLContext)]
pub struct OrganizationMatchData {
    pub organization: OrganizationData,
    pub similarity_score: f64,
}

/// Result of running discovery search
#[derive(Debug, Clone, juniper::GraphQLObject)]
pub struct DiscoverySearchResult {
    pub queries_run: i32,
    pub total_results: i32,
    pub websites_created: i32,
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
                proceeds_beneficiary_id: business.proceeds_beneficiary_id.map(|id| id.into_uuid()),
                proceeds_description: business.proceeds_description,
                impact_statement: business.impact_statement,
            });
        }
    }

    post_type
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

        let connection = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| post_actions::get_posts_paginated(status_filter, &validated, ectx))
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
        let pagination_args = PaginationArgs {
            first,
            after,
            last,
            before,
        };
        let validated = pagination_args
            .validate()
            .map_err(|e| FieldError::new(e, juniper::Value::null()))?;

        let connection = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| member_actions::get_members_paginated(&validated, ectx))
            .await
            .map_err(|e| {
                error!("Failed to get paginated members: {}", e);
                FieldError::new(e.to_string(), juniper::Value::null())
            })?;

        Ok(connection)
    }

    // =========================================================================
    // Organization Queries
    // =========================================================================

    /// Get an organization by ID (admin only)
    async fn organization(
        ctx: &GraphQLContext,
        id: String,
    ) -> FieldResult<Option<OrganizationData>> {
        ctx.require_admin()?;

        use crate::common::OrganizationId;

        let org_id = OrganizationId::parse(&id)?;
        match Organization::find_by_id(org_id, &ctx.db_pool).await {
            Ok(org) => Ok(Some(OrganizationData::from(org))),
            Err(_) => Ok(None),
        }
    }

    /// Search organizations by name (admin only)
    async fn search_organizations(
        ctx: &GraphQLContext,
        query: String,
    ) -> FieldResult<Vec<OrganizationData>> {
        ctx.require_admin()?;

        let orgs = Organization::search_by_name(&query, &ctx.db_pool).await?;
        Ok(orgs.into_iter().map(OrganizationData::from).collect())
    }

    /// Search organizations using AI semantic search (admin only)
    async fn search_organizations_semantic(
        ctx: &GraphQLContext,
        query: String,
        limit: Option<i32>,
    ) -> FieldResult<Vec<OrganizationMatchData>> {
        ctx.require_admin()?;

        use crate::kernel::ai_matching::AIMatchingService;

        let ai_matching = AIMatchingService::new((*ctx.openai_client).clone());

        let results = if let Some(lim) = limit {
            ai_matching
                .find_relevant_organizations_with_config(query, 0.7, lim, &ctx.db_pool)
                .await?
        } else {
            ai_matching
                .find_relevant_organizations(query, &ctx.db_pool)
                .await?
        };

        Ok(results
            .into_iter()
            .map(|(org, similarity)| OrganizationMatchData {
                organization: OrganizationData::from(org),
                similarity_score: similarity as f64,
            })
            .collect())
    }

    /// Get paginated organizations with cursor-based pagination (Relay spec)
    ///
    /// Arguments:
    /// - first: Return first N items (forward pagination)
    /// - after: Return items after this cursor (forward pagination)
    /// - last: Return last N items (backward pagination)
    /// - before: Return items before this cursor (backward pagination)
    async fn organizations(
        ctx: &GraphQLContext,
        first: Option<i32>,
        after: Option<String>,
        last: Option<i32>,
        before: Option<String>,
    ) -> FieldResult<OrganizationConnection> {
        let pagination_args = PaginationArgs {
            first,
            after,
            last,
            before,
        };
        let validated = pagination_args
            .validate()
            .map_err(|e| FieldError::new(e, juniper::Value::null()))?;

        let connection = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| organization_actions::get_organizations_paginated(&validated, ectx))
            .await
            .map_err(|e| {
                error!("Failed to get paginated organizations: {}", e);
                FieldError::new(e.to_string(), juniper::Value::null())
            })?;

        Ok(connection)
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

        let connection = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| website_actions::get_websites_paginated(status_ref, &validated, ectx))
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
        let websites = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| website_actions::get_pending_websites(ectx))
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
            .create_embedding(&query)
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
    // Page Snapshot Queries
    // =========================================================================

    /// Get a page snapshot by ID
    async fn page_snapshot(
        ctx: &GraphQLContext,
        id: Uuid,
    ) -> FieldResult<Option<PageSnapshotData>> {
        match PageSnapshot::find_by_id(&ctx.db_pool, id).await {
            Ok(snapshot) => Ok(Some(PageSnapshotData::from(snapshot))),
            Err(_) => Ok(None),
        }
    }

    /// Get a page snapshot by URL
    async fn page_snapshot_by_url(
        ctx: &GraphQLContext,
        url: String,
    ) -> FieldResult<Option<PageSnapshotData>> {
        let snapshot: Option<PageSnapshot> = sqlx::query_as::<_, PageSnapshot>(
            "SELECT * FROM page_snapshots WHERE url = $1 ORDER BY crawled_at DESC LIMIT 1",
        )
        .bind(&url)
        .fetch_optional(&ctx.db_pool)
        .await
        .context("Failed to query page snapshot by URL")?;

        Ok(snapshot.map(PageSnapshotData::from))
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
        let containers = Container::find_recent_by_type("ai_chat", limit, &ctx.db_pool).await?;
        Ok(containers.into_iter().map(ContainerData::from).collect())
    }

    // =========================================================================
    // Provider Queries
    // =========================================================================

    /// Get a provider by ID
    async fn provider(ctx: &GraphQLContext, id: String) -> FieldResult<Option<ProviderData>> {
        info!("get_provider query called: {}", id);

        let provider = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| provider_actions::get_provider(id, ectx))
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

        let connection = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| provider_actions::get_providers_paginated(status_ref, &validated, ectx))
            .await
            .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))?;

        Ok(connection)
    }

    /// Get all pending providers (for admin approval queue)
    async fn pending_providers(ctx: &GraphQLContext) -> FieldResult<Vec<ProviderData>> {
        let providers = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| provider_actions::get_pending_providers(ectx))
            .await
            .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))?;

        Ok(providers.into_iter().map(ProviderData::from).collect())
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
    /// Creates a job record, runs the crawl immediately, and returns the result.
    /// Job is tracked in the database - query via the website's `crawlJob` field.
    async fn crawl_website(ctx: &GraphQLContext, website_id: Uuid) -> FieldResult<ScrapeJobResult> {
        use crate::domains::crawling::{execute_crawl_website_job, CrawlWebsiteJob};

        info!(website_id = %website_id, "Crawling website (with job tracking)");

        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let job = CrawlWebsiteJob::new(website_id, user.member_id.into_uuid(), true);

        let result = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| execute_crawl_website_job(job.clone(), ectx))
            .await
            .map_err(to_field_error)?;

        Ok(ScrapeJobResult {
            job_id: result.job_id,
            source_id: website_id,
            status: result.status,
            message: result.message,
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

        let result = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| {
                crawling_actions::discover_website(
                    website_id,
                    user.member_id.into_uuid(),
                    user.is_admin,
                    ectx,
                )
            })
            .await
            .map_err(to_field_error)?;

        Ok(ScrapeJobResult {
            job_id: result.job_id,
            source_id: result.website_id,
            status: result.status,
            message: result.message,
        })
    }

    /// Submit a listing from a member (public, goes to pending_approval)
    async fn submit_post(
        ctx: &GraphQLContext,
        input: SubmitPostInput,
        member_id: Option<Uuid>,
    ) -> FieldResult<PostType> {
        let post = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| post_actions::submit_post(input, member_id, ectx))
            .await
            .map_err(to_field_error)?;

        Ok(PostType::from(post))
    }

    /// Submit a resource link (URL) for scraping (public)
    async fn submit_resource_link(
        ctx: &GraphQLContext,
        input: SubmitResourceLinkInput,
    ) -> FieldResult<SubmitResourceLinkResult> {
        let result = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| {
                post_actions::submit_resource_link(input.url, input.submitter_contact, ectx)
            })
            .await
            .map_err(to_field_error)?;

        Ok(SubmitResourceLinkResult {
            job_id: result.job_id,
            status: result.status,
            message: result.message,
        })
    }

    /// Approve a listing (make it visible to volunteers) (admin only)
    async fn approve_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<PostType> {
        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let post = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| {
                post_actions::approve_post(post_id, user.member_id.into_uuid(), user.is_admin, ectx)
            })
            .await
            .map_err(to_field_error)?;

        Ok(PostType::from(post))
    }

    /// Edit and approve a listing (fix AI mistakes or improve user content) (admin only)
    async fn edit_and_approve_post(
        ctx: &GraphQLContext,
        post_id: Uuid,
        input: EditPostInput,
    ) -> FieldResult<PostType> {
        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let post = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| {
                post_actions::edit_and_approve_post(
                    post_id,
                    input,
                    user.member_id.into_uuid(),
                    user.is_admin,
                    ectx,
                )
            })
            .await
            .map_err(to_field_error)?;

        Ok(PostType::from(post))
    }

    /// Reject a listing (hide forever) (admin only)
    async fn reject_post(ctx: &GraphQLContext, post_id: Uuid, reason: String) -> FieldResult<bool> {
        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        ctx.engine
            .activate(ctx.app_state())
            .process(|ectx| {
                post_actions::reject_post(
                    post_id,
                    reason,
                    user.member_id.into_uuid(),
                    user.is_admin,
                    ectx,
                )
            })
            .await
            .map_err(to_field_error)
    }

    /// Delete a listing (admin only)
    async fn delete_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        ctx.engine
            .activate(ctx.app_state())
            .process(|ectx| {
                post_actions::delete_post(post_id, user.member_id.into_uuid(), user.is_admin, ectx)
            })
            .await
            .map_err(to_field_error)
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
        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let post = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| {
                post_actions::expire_post(post_id, user.member_id.into_uuid(), user.is_admin, ectx)
            })
            .await
            .map_err(to_field_error)?;

        Ok(PostData::from(post))
    }

    /// Archive a post (admin only)
    async fn archive_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<PostData> {
        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let post = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| {
                post_actions::archive_post(post_id, user.member_id.into_uuid(), user.is_admin, ectx)
            })
            .await
            .map_err(to_field_error)?;

        Ok(PostData::from(post))
    }

    /// Track post view (analytics - public)
    async fn post_viewed(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
        ctx.engine
            .activate(ctx.app_state())
            .process(|ectx| post_actions::track_post_view(post_id, ectx))
            .await
            .map_err(to_field_error)
    }

    /// Track post click (analytics - public)
    async fn post_clicked(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
        ctx.engine
            .activate(ctx.app_state())
            .process(|ectx| post_actions::track_post_click(post_id, ectx))
            .await
            .map_err(to_field_error)
    }

    /// Report a listing (public or authenticated)
    async fn report_post(
        ctx: &GraphQLContext,
        post_id: Uuid,
        reason: String,
        category: String,
        reporter_email: Option<String>,
    ) -> FieldResult<PostReportData> {
        let reported_by = ctx.auth_user.as_ref().map(|u| u.member_id.into_uuid());

        let report = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| {
                post_actions::report_post(
                    post_id,
                    reported_by,
                    reporter_email,
                    reason,
                    category,
                    ectx,
                )
            })
            .await
            .map_err(to_field_error)?;

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

        ctx.engine
            .activate(ctx.app_state())
            .process(|ectx| {
                post_actions::resolve_report(
                    report_id,
                    user.member_id.into_uuid(),
                    user.is_admin,
                    resolution_notes,
                    action_taken,
                    ectx,
                )
            })
            .await
            .map_err(to_field_error)
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

        ctx.engine
            .activate(ctx.app_state())
            .process(|ectx| {
                post_actions::dismiss_report(
                    report_id,
                    user.member_id.into_uuid(),
                    user.is_admin,
                    resolution_notes,
                    ectx,
                )
            })
            .await
            .map_err(to_field_error)
    }

    /// Run discovery search manually (admin only)
    async fn run_discovery_search(ctx: &GraphQLContext) -> FieldResult<DiscoverySearchResult> {
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

        info!(requested_by = %user.member_id, "Admin triggering manual discovery search");

        let config = crate::config::Config::from_env().map_err(|e| {
            FieldError::new(
                format!("Failed to load config: {}", e),
                juniper::Value::null(),
            )
        })?;

        let web_searcher = extraction::TavilyWebSearcher::new(config.tavily_api_key);

        let result =
            crate::domains::posts::effects::run_discovery_searches(&web_searcher, &ctx.db_pool)
                .await
                .map_err(|e| {
                    FieldError::new(
                        format!("Discovery search failed: {}", e),
                        juniper::Value::null(),
                    )
                })?;

        info!(
            queries_run = result.queries_run,
            total_results = result.total_results,
            websites_created = result.websites_created,
            "Discovery search completed"
        );

        Ok(DiscoverySearchResult {
            queries_run: result.queries_run as i32,
            total_results: result.total_results as i32,
            websites_created: result.websites_created as i32,
        })
    }

    /// Generate embedding for a single post (admin only)
    async fn generate_post_embedding(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
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

        info!(post_id = %post_id, "Embedding generation is deprecated - no-op");

        // Embeddings are deprecated - return success without doing anything
        Ok(true)
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

        let result = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| {
                post_actions::deduplicate_posts(user.member_id.into_uuid(), user.is_admin, ectx)
            })
            .await
            .map_err(to_field_error)?;

        Ok(DeduplicationResult {
            job_id: result.job_id,
            duplicates_found: result.duplicates_found,
            posts_merged: result.posts_merged,
            posts_deleted: result.posts_deleted,
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
        let is_phone = phone_number.starts_with('+');
        let is_email = phone_number.contains('@');

        if !is_phone && !is_email {
            return Err(FieldError::new(
                "Must provide either phone number with country code (e.g., +1234567890) or email address",
                juniper::Value::null(),
            ));
        }

        let phone = phone_number.clone();

        let result = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| auth_actions::send_otp(phone, ectx))
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to send OTP");
                FieldError::new(e.to_string(), juniper::Value::null())
            })?;

        match result {
            auth_actions::SendOtpResult::Sent => Ok(true),
            auth_actions::SendOtpResult::NotRegistered => Err(FieldError::new(
                "Phone number not registered",
                juniper::Value::null(),
            )),
        }
    }

    /// Verify OTP code and create session
    async fn verify_code(
        ctx: &GraphQLContext,
        phone_number: String,
        code: String,
    ) -> FieldResult<String> {
        let phone = phone_number.clone();

        let result = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| auth_actions::verify_otp(phone, code, ectx))
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to verify OTP");
                FieldError::new(e.to_string(), juniper::Value::null())
            })?;

        match result {
            auth_actions::VerifyOtpResult::Verified {
                member_id,
                is_admin,
            } => {
                let token = ctx
                    .jwt_service
                    .create_token(member_id.into(), phone_number, is_admin)
                    .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))?;

                Ok(token)
            }
            auth_actions::VerifyOtpResult::Failed { reason } => Err(FieldError::new(
                format!("Verification failed: {}", reason),
                juniper::Value::null(),
            )),
        }
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

        let member = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| {
                member_actions::register_member(expo_push_token, searchable_text, city, state, ectx)
            })
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to register member");
                FieldError::new(e.to_string(), juniper::Value::null())
            })?
            .read()
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to read member after registration");
                FieldError::new(e.to_string(), juniper::Value::null())
            })?;

        Ok(MemberData::from(member))
    }

    /// Update member status (activate/deactivate) (admin only)
    async fn update_member_status(
        ctx: &GraphQLContext,
        member_id: String,
        active: bool,
    ) -> FieldResult<MemberData> {
        let member_id = Uuid::parse_str(&member_id)?;

        info!(
            "update_member_status mutation called: {} -> {}",
            member_id, active
        );

        let member = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| member_actions::update_member_status(member_id, active, ectx))
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to update member status");
                FieldError::new(e.to_string(), juniper::Value::null())
            })?
            .read()
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to read member after status update");
                FieldError::new(e.to_string(), juniper::Value::null())
            })?;

        Ok(MemberData::from(member))
    }

    // =========================================================================
    // Organization Mutations
    // =========================================================================

    /// Create a new organization (admin only)
    async fn create_organization(
        ctx: &GraphQLContext,
        name: String,
        description: Option<String>,
        website: Option<String>,
        phone: Option<String>,
        city: Option<String>,
    ) -> FieldResult<OrganizationData> {
        ctx.require_admin()?;

        use crate::domains::organization::models::CreateOrganization;

        let primary_address = city.map(|c| format!("{}, MN", c));

        let builder = CreateOrganization::builder()
            .name(name)
            .description(description)
            .website(website)
            .phone(phone)
            .primary_address(primary_address)
            .organization_type(Some("nonprofit".to_string()))
            .build();

        let created = Organization::create(builder, &ctx.db_pool).await?;

        Ok(OrganizationData::from(created))
    }

    /// Add tags to an organization (admin only)
    async fn add_organization_tags(
        ctx: &GraphQLContext,
        organization_id: String,
        tags: Vec<TagInput>,
    ) -> FieldResult<OrganizationData> {
        ctx.require_admin()?;

        use crate::common::OrganizationId;

        let org_id = OrganizationId::parse(&organization_id)?;

        for tag_input in tags {
            let tag =
                Tag::find_or_create(&tag_input.kind, &tag_input.value, None, &ctx.db_pool).await?;
            let _ = Taggable::create_organization_tag(org_id, tag.id, &ctx.db_pool).await;
        }

        let org = Organization::find_by_id(org_id, &ctx.db_pool).await?;
        Ok(OrganizationData::from(org))
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

        let website = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| website_actions::approve_website(id, requested_by, ectx))
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to approve website: {}", e),
                    juniper::Value::null(),
                )
            })?
            .read()
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

        let website = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| website_actions::reject_website(id, reason, requested_by, ectx))
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to reject website: {}", e),
                    juniper::Value::null(),
                )
            })?
            .read()
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

        let website = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| website_actions::suspend_website(id, reason, requested_by, ectx))
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to suspend website: {}", e),
                    juniper::Value::null(),
                )
            })?
            .read()
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
        let requested_by = user.member_id;

        let website = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| {
                website_actions::update_crawl_settings(id, max_pages_per_crawl, requested_by, ectx)
            })
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to update settings: {}", e),
                    juniper::Value::null(),
                )
            })?
            .read()
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to fetch website: {}", e),
                    juniper::Value::null(),
                )
            })?;

        Ok(WebsiteData::from(website))
    }

    /// Refresh a page snapshot by re-scraping the specific page URL (admin only)
    async fn refresh_page_snapshot(
        ctx: &GraphQLContext,
        snapshot_id: String,
    ) -> FieldResult<ScrapeJobResult> {
        info!(snapshot_id = %snapshot_id, "Refreshing page snapshot");

        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let snapshot_uuid = Uuid::parse_str(&snapshot_id)
            .map_err(|_| FieldError::new("Invalid snapshot ID", juniper::Value::null()))?;

        let result = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| {
                post_actions::refresh_page_snapshot(
                    snapshot_uuid,
                    user.member_id.into_uuid(),
                    user.is_admin,
                    ectx,
                )
            })
            .await
            .map_err(to_field_error)?;

        Ok(ScrapeJobResult {
            job_id: result.job_id,
            source_id: result.page_snapshot_id, // Return page snapshot ID as source_id
            status: result.status,
            message: result.message,
        })
    }

    /// Regenerate posts from existing page snapshots (admin only)
    ///
    /// Creates a job record, runs the regeneration immediately, and returns the result.
    /// Job is tracked in the database - query via the website's `regeneratePostsJob` field.
    async fn regenerate_posts(
        ctx: &GraphQLContext,
        website_id: Uuid,
    ) -> FieldResult<ScrapeJobResult> {
        use crate::domains::crawling::{execute_regenerate_posts_job, RegeneratePostsJob};

        info!(website_id = %website_id, "Regenerating posts (with job tracking)");

        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let job = RegeneratePostsJob::new(website_id, user.member_id.into_uuid());

        let result = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| execute_regenerate_posts_job(job.clone(), ectx))
            .await
            .map_err(to_field_error)?;

        Ok(ScrapeJobResult {
            job_id: result.job_id,
            source_id: website_id,
            status: result.status,
            message: result.message,
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

        let result = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| {
                website_approval_actions::assess_website(
                    website_uuid,
                    user.member_id.into_uuid(),
                    ectx,
                )
            })
            .await
            .map_err(to_field_error)?;

        match result.assessment_id {
            Some(id) => Ok(id.to_string()),
            None => {
                // Assessment is still processing, return job_id
                Ok(result.job_id.to_string())
            }
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

        let container = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| {
                chatroom_actions::create_container(
                    "ai_chat".to_string(),
                    None,
                    language,
                    member_id,
                    with_agent,
                    ectx,
                )
            })
            .await
            .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))?
            .read()
            .await
            .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))?;

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

        let message = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| {
                chatroom_actions::send_message(container_id, content, member_id, None, ectx)
            })
            .await
            .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))?
            .read()
            .await
            .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))?;

        Ok(MessageData::from(message))
    }

    /// Signal that the user is typing (for real-time indicators)
    async fn signal_typing(ctx: &GraphQLContext, container_id: String) -> FieldResult<bool> {
        let member_id = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?
            .member_id;

        let container_id = ContainerId::parse(&container_id)?;

        let handle = ctx.engine.activate(ctx.app_state());
        handle
            .context
            .emit(crate::domains::chatrooms::events::TypingEvent::Started {
                container_id,
                member_id,
            });

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

        let provider = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| provider_actions::submit_provider(input, member_id, ectx))
            .await
            .map_err(to_field_error)?;

        Ok(ProviderData::from(provider))
    }

    /// Update a provider (admin only)
    async fn update_provider(
        ctx: &GraphQLContext,
        provider_id: String,
        input: UpdateProviderInput,
    ) -> FieldResult<ProviderData> {
        info!("update_provider mutation called: {}", provider_id);

        let provider = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| provider_actions::update_provider(provider_id, input, ectx))
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
        info!("approve_provider mutation called: {}", provider_id);

        let provider = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| provider_actions::approve_provider(provider_id, reviewed_by_id, ectx))
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
        info!("reject_provider mutation called: {}", provider_id);

        let provider = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| {
                provider_actions::reject_provider(provider_id, reason, reviewed_by_id, ectx)
            })
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
        info!(
            "add_provider_tag mutation called: {} - {}:{}",
            provider_id, tag_kind, tag_value
        );

        let tag = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| {
                provider_actions::add_provider_tag(
                    provider_id,
                    tag_kind,
                    tag_value,
                    display_name,
                    ectx,
                )
            })
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
        info!(
            "remove_provider_tag mutation called: {} - {}",
            provider_id, tag_id
        );

        ctx.engine
            .activate(ctx.app_state())
            .process(|ectx| provider_actions::remove_provider_tag(provider_id, tag_id, ectx))
            .await
            .map_err(to_field_error)?;

        Ok(true)
    }

    /// Add a contact to a provider (admin only)
    async fn add_provider_contact(
        ctx: &GraphQLContext,
        provider_id: String,
        contact_type: String,
        contact_value: String,
        contact_label: Option<String>,
        is_public: Option<bool>,
        display_order: Option<i32>,
    ) -> FieldResult<ContactData> {
        info!(
            "add_provider_contact mutation called: {} - {}:{}",
            provider_id, contact_type, contact_value
        );

        let contact = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| {
                provider_actions::add_provider_contact(
                    provider_id,
                    contact_type,
                    contact_value,
                    contact_label,
                    is_public,
                    display_order,
                    ectx,
                )
            })
            .await
            .map_err(to_field_error)?;

        Ok(ContactData::from(contact))
    }

    /// Remove a contact (admin only)
    async fn remove_provider_contact(
        ctx: &GraphQLContext,
        contact_id: String,
    ) -> FieldResult<bool> {
        info!("remove_provider_contact mutation called: {}", contact_id);

        ctx.engine
            .activate(ctx.app_state())
            .process(|ectx| provider_actions::remove_provider_contact(contact_id, ectx))
            .await
            .map_err(to_field_error)?;

        Ok(true)
    }

    /// Delete a provider (admin only)
    async fn delete_provider(ctx: &GraphQLContext, provider_id: String) -> FieldResult<bool> {
        info!("delete_provider mutation called: {}", provider_id);

        ctx.engine
            .activate(ctx.app_state())
            .process(|ectx| provider_actions::delete_provider(provider_id, ectx))
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
        let action_tags: Vec<post_actions::tags::TagInput> = tags
            .into_iter()
            .map(|t| post_actions::tags::TagInput {
                kind: t.kind,
                value: t.value,
            })
            .collect();

        let post = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| {
                post_actions::tags::update_post_tags(post_id, action_tags.clone(), ectx)
            })
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
        let tag = ctx
            .engine
            .activate(ctx.app_state())
            .process(|ectx| {
                post_actions::tags::add_post_tag(
                    post_id,
                    tag_kind.clone(),
                    tag_value.clone(),
                    display_name.clone(),
                    ectx,
                )
            })
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
        ctx.engine
            .activate(ctx.app_state())
            .process(|ectx| post_actions::tags::remove_post_tag(post_id, tag_id.clone(), ectx))
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

        let deps = ctx.deps();

        match extraction_actions::submit_url(&input.url, input.query.as_deref(), deps).await {
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

        let deps = ctx.deps();

        match extraction_actions::trigger_extraction(&input.query, input.site.as_deref(), deps)
            .await
        {
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

        let deps = ctx.deps();

        let result = extraction_actions::ingest_site(&site_url, max_pages, deps)
            .await
            .map_err(to_field_error)?;

        Ok(IngestSiteResult {
            site_url: result.site_url,
            pages_crawled: result.pages_crawled,
            pages_summarized: result.pages_summarized,
            pages_skipped: result.pages_skipped,
        })
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

pub type Schema = RootNode<'static, Query, Mutation, EmptySubscription<GraphQLContext>>;

pub fn create_schema() -> Schema {
    Schema::new(Query, Mutation, EmptySubscription::new())
}
