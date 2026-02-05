//! GraphQL schema definition.

use super::context::GraphQLContext;
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
use crate::domains::discovery::actions as discovery_actions;
use crate::domains::discovery::models::{
    DiscoveryFilterRule, DiscoveryQuery, DiscoveryRun, DiscoveryRunResult,
};
use crate::domains::website::actions as website_actions;
use crate::domains::website_approval::actions as website_approval_actions;

// Domain data types (GraphQL types)
use crate::domains::chatrooms::data::{ContainerData, MessageData};
use crate::domains::extraction::data::{
    ExtractionData, ExtractionPageData, SubmitUrlInput, SubmitUrlResult, TriggerExtractionInput,
    TriggerExtractionResult,
};
use crate::domains::member::data::{MemberConnection, MemberData};
use crate::domains::chatrooms::events::ChatEvent;
use crate::domains::member::events::MemberEvent;
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
use crate::domains::providers::events::ProviderEvent;
use crate::domains::providers::models::Provider;
use crate::domains::website::data::{WebsiteConnection, WebsiteData};
use crate::domains::website::events::WebsiteEvent;
use crate::domains::website_approval::data::{WebsiteAssessmentData, WebsiteSearchResultData};

// Domain models (for queries)
use crate::domains::chatrooms::models::{Container, Message};
use crate::domains::contacts::ContactData;
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
    pub organization_name: String,
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

        let connection = post_actions::get_posts_paginated(status_filter, &validated, ctx.deps())
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

        let connection = member_actions::get_members_paginated(&validated, ctx.deps())
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
            .map_err(|e| FieldError::new(format!("Search failed: {}", e), juniper::Value::null()))?;

        Ok(results
            .into_iter()
            .map(|r| PostSearchResultData {
                post_id: r.post_id.into_uuid(),
                title: r.title,
                description: r.description,
                organization_name: r.organization_name,
                category: r.category,
                post_type: r.post_type,
                similarity: r.similarity,
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
    /// Get organizations (admin only)
    async fn organizations(
        ctx: &GraphQLContext,
        first: Option<i32>,
        after: Option<String>,
        last: Option<i32>,
        before: Option<String>,
    ) -> FieldResult<OrganizationConnection> {
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

        let connection = organization_actions::get_organizations_paginated(&validated, ctx.deps())
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

        let connection = website_actions::get_websites_paginated(status_ref, &validated, ctx.deps())
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

        let websites = website_actions::get_pending_websites(ctx.deps())
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
        let revisions = post_actions::get_pending_revisions(website_id, &ctx.db_pool)
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
        let revision = post_actions::get_revision_for_post(post_id, &ctx.db_pool)
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
    async fn extraction_pages_count(
        ctx: &GraphQLContext,
        domain: String,
    ) -> FieldResult<i32> {
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
        let containers = Container::find_recent_by_type("ai_chat", limit, &ctx.db_pool).await?;
        Ok(containers.into_iter().map(ContainerData::from).collect())
    }

    // =========================================================================
    // Provider Queries
    // =========================================================================

    /// Get a provider by ID (admin only)
    async fn provider(ctx: &GraphQLContext, id: String) -> FieldResult<Option<ProviderData>> {
        ctx.require_admin()?;

        info!("get_provider query called: {}", id);

        let provider = provider_actions::get_provider(id, ctx.deps())
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

        let connection = provider_actions::get_providers_paginated(status_ref, &validated, ctx.deps())
            .await
            .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))?;

        Ok(connection)
    }

    /// Get all pending providers (for admin approval queue)
    async fn pending_providers(ctx: &GraphQLContext) -> FieldResult<Vec<ProviderData>> {
        ctx.require_admin()?;

        let providers = provider_actions::get_pending_providers(ctx.deps())
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
            DiscoveryQuery::find_all(&ctx.db_pool).await.map_err(to_field_error)?
        } else {
            DiscoveryQuery::find_active(&ctx.db_pool).await.map_err(to_field_error)?
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
    /// Emits a CrawlWebsiteEnqueued event and returns immediately.
    /// The queued crawl_website_effect picks it up in the background.
    async fn crawl_website(ctx: &GraphQLContext, website_id: Uuid) -> FieldResult<ScrapeJobResult> {
        use crate::domains::crawling::events::CrawlEvent;

        info!(website_id = %website_id, "Emitting crawl website event");

        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let visitor_id = user.member_id.into_uuid();

        let handle = ctx.queue_engine
            .process(CrawlEvent::CrawlWebsiteEnqueued {
                website_id,
                visitor_id,
                use_firecrawl: true,
            })
            .await
            .map_err(to_field_error)?;

        Ok(ScrapeJobResult {
            job_id: handle.correlation_id,
            source_id: website_id,
            status: "enqueued".to_string(),
            message: Some("Crawl enqueued for background processing".to_string()),
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

        let event = crawling_actions::discover_website(
            website_id,
            user.member_id.into_uuid(),
            user.is_admin,
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        ctx.queue_engine.process(event).await.map_err(to_field_error)?;

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

        let event = post_actions::submit_post(input.clone(), member_id, ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine.process(event.clone()).await.map_err(to_field_error)?;

        // Extract post_id from event and find the post
        let post_id = match event {
            PostEvent::PostEntryCreated { post_id, .. } => post_id,
            _ => return Err(FieldError::new("Unexpected event type", juniper::Value::null())),
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

        let event = post_actions::submit_resource_link(input.url, input.submitter_contact, ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine.process(event.clone()).await.map_err(to_field_error)?;

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
                message: format!("Your submission has been received. We'll process it shortly: {}", url),
            }),
            _ => Err(FieldError::new("Unexpected event type", juniper::Value::null())),
        }
    }

    /// Approve a listing (make it visible to volunteers) (admin only)
    async fn approve_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<PostType> {
        use crate::domains::posts::events::PostEvent;

        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let event = post_actions::approve_post(post_id, user.member_id.into_uuid(), user.is_admin, ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine.process(event.clone()).await.map_err(to_field_error)?;

        let PostEvent::PostApproved { post_id } = event else {
            return Err(FieldError::new("Unexpected event type", juniper::Value::null()));
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

        let event = post_actions::edit_and_approve_post(
            post_id,
            input,
            user.member_id.into_uuid(),
            user.is_admin,
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        ctx.queue_engine.process(event.clone()).await.map_err(to_field_error)?;

        let PostEvent::PostApproved { post_id } = event else {
            return Err(FieldError::new("Unexpected event type", juniper::Value::null()));
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

        let event = post_actions::reject_post(
            post_id,
            reason,
            user.member_id.into_uuid(),
            user.is_admin,
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        ctx.queue_engine.process(event).await.map_err(to_field_error)?;

        Ok(true)
    }

    /// Delete a listing (admin only)
    async fn delete_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
        let user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let event = post_actions::delete_post(post_id, user.member_id.into_uuid(), user.is_admin, ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine.process(event).await.map_err(to_field_error)?;

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

        let event = post_actions::expire_post(post_id, user.member_id.into_uuid(), user.is_admin, ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine.process(event.clone()).await.map_err(to_field_error)?;

        let PostEvent::PostExpired { post_id } = event else {
            return Err(FieldError::new("Unexpected event type", juniper::Value::null()));
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

        let event = post_actions::archive_post(post_id, user.member_id.into_uuid(), user.is_admin, ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine.process(event.clone()).await.map_err(to_field_error)?;

        let PostEvent::PostArchived { post_id } = event else {
            return Err(FieldError::new("Unexpected event type", juniper::Value::null()));
        };

        let post = Post::find_by_id(post_id, &ctx.db_pool)
            .await
            .map_err(to_field_error)?
            .ok_or_else(|| FieldError::new("Post not found", juniper::Value::null()))?;

        Ok(PostData::from(post))
    }

    /// Track post view (analytics - public)
    async fn post_viewed(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
        let event = post_actions::track_post_view(post_id, ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine.process(event).await.map_err(to_field_error)?;

        Ok(true)
    }

    /// Track post click (analytics - public)
    async fn post_clicked(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
        let event = post_actions::track_post_click(post_id, ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine.process(event).await.map_err(to_field_error)?;

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

        let event = post_actions::report_post(
            post_id,
            reported_by,
            reporter_email,
            reason,
            category,
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        ctx.queue_engine.process(event.clone()).await.map_err(to_field_error)?;

        let PostEvent::PostReported { report_id, post_id: returned_post_id } = event else {
            return Err(FieldError::new("Unexpected event type", juniper::Value::null()));
        };

        // Fetch the report from DB
        let reports = PostReportRecord::query_for_post(returned_post_id, &ctx.db_pool)
            .await
            .map_err(to_field_error)?;

        let report = reports.into_iter().find(|r| r.id == report_id)
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

        let event = post_actions::resolve_report(
            report_id,
            user.member_id.into_uuid(),
            user.is_admin,
            resolution_notes,
            action_taken,
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        ctx.queue_engine.process(event).await.map_err(to_field_error)?;

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

        let event = post_actions::dismiss_report(
            report_id,
            user.member_id.into_uuid(),
            user.is_admin,
            resolution_notes,
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        ctx.queue_engine.process(event).await.map_err(to_field_error)?;

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
        let updated_post = post_actions::approve_revision(revision_id, &ctx.db_pool)
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
        post_actions::reject_revision(revision_id, &ctx.db_pool)
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

        let event = discovery_actions::run_discovery("manual", ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine.process(event.clone()).await.map_err(to_field_error)?;

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
        DiscoveryQuery::delete(id, &ctx.db_pool).await.map_err(to_field_error)?;
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
        DiscoveryFilterRule::delete(id, &ctx.db_pool).await.map_err(to_field_error)?;
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
        post_operations::generate_post_embedding(post_id, ctx.server_deps.embedding_service.as_ref(), &ctx.db_pool)
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

        info!(processed = processed, failed = failed, remaining = remaining, "Backfill completed");

        Ok(BackfillPostEmbeddingsResult {
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

        let event = post_actions::deduplicate_posts(
            user.member_id.into_uuid(),
            user.is_admin,
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        ctx.queue_engine.process(event).await.map_err(to_field_error)?;

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
        let is_phone = phone_number.starts_with('+');
        let is_email = phone_number.contains('@');

        if !is_phone && !is_email {
            return Err(FieldError::new(
                "Must provide either phone number with country code (e.g., +1234567890) or email address",
                juniper::Value::null(),
            ));
        }

        let phone = phone_number.clone();

        let event = auth_actions::send_otp(phone, ctx.deps())
            .await
            .map_err(|e| {
                // Check for NotAuthorizedError specifically
                if e.downcast_ref::<auth_actions::NotAuthorizedError>().is_some() {
                    return FieldError::new("Not authorized", juniper::Value::null());
                }
                error!(error = %e, "Failed to send OTP");
                FieldError::new(e.to_string(), juniper::Value::null())
            })?;

        ctx.queue_engine.process(event).await.map_err(to_field_error)?;

        Ok(true)
    }

    /// Verify OTP code and create session
    async fn verify_code(
        ctx: &GraphQLContext,
        phone_number: String,
        code: String,
    ) -> FieldResult<String> {
        use crate::domains::auth::events::AuthEvent;

        let event = auth_actions::verify_otp(phone_number, code, ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine.process(event.clone()).await.map_err(to_field_error)?;

        match event {
            AuthEvent::OTPVerified { token, .. } => Ok(token),
            _ => Err(FieldError::new("Unexpected event type", juniper::Value::null())),
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

        let token_for_lookup = expo_push_token.clone();

        let event = member_actions::register_member(
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

        ctx.queue_engine.process(event).await.map_err(to_field_error)?;

        // After process(), find member by expo_push_token
        let member = Member::find_by_token(&token_for_lookup, &ctx.db_pool)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to read member after registration");
                FieldError::new(e.to_string(), juniper::Value::null())
            })?
            .ok_or_else(|| FieldError::new("Member not found after registration", juniper::Value::null()))?;

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

        let event = member_actions::update_member_status(member_id_uuid, active, ctx.deps())
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to update member status");
                FieldError::new(e.to_string(), juniper::Value::null())
            })?;

        ctx.queue_engine.process(event.clone()).await.map_err(to_field_error)?;

        // Extract member_id from event and fetch member data
        let MemberEvent::MemberStatusUpdated { member_id: returned_member_id, .. } = event else {
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

        let event = website_actions::approve_website(id, requested_by, ctx.deps())
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to approve website: {}", e),
                    juniper::Value::null(),
                )
            })?;

        ctx.queue_engine.process(event.clone()).await.map_err(to_field_error)?;

        // Extract website_id from event and fetch updated website
        let WebsiteEvent::WebsiteApproved { website_id: returned_website_id, .. } = event else {
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

        let event = website_actions::reject_website(id, reason, requested_by, ctx.deps())
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to reject website: {}", e),
                    juniper::Value::null(),
                )
            })?;

        ctx.queue_engine.process(event.clone()).await.map_err(to_field_error)?;

        // Extract website_id from event and fetch updated website
        let WebsiteEvent::WebsiteRejected { website_id: returned_website_id, .. } = event else {
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

        let event = website_actions::suspend_website(id, reason, requested_by, ctx.deps())
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to suspend website: {}", e),
                    juniper::Value::null(),
                )
            })?;

        ctx.queue_engine.process(event.clone()).await.map_err(to_field_error)?;

        // Extract website_id from event and fetch updated website
        let WebsiteEvent::WebsiteSuspended { website_id: returned_website_id, .. } = event else {
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

        let event = website_actions::update_crawl_settings(id, max_pages_per_crawl, ctx.deps())
            .await
            .map_err(|e| {
                FieldError::new(
                    format!("Failed to update settings: {}", e),
                    juniper::Value::null(),
                )
            })?;

        ctx.queue_engine.process(event.clone()).await.map_err(to_field_error)?;

        // Extract website_id from event and fetch updated website
        let WebsiteEvent::CrawlSettingsUpdated { website_id: returned_website_id, .. } = event else {
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

        let handle = ctx.queue_engine
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
    async fn regenerate_post(
        ctx: &GraphQLContext,
        post_id: Uuid,
    ) -> FieldResult<ScrapeJobResult> {
        use crate::domains::crawling::events::CrawlEvent;

        info!(post_id = %post_id, "Emitting regenerate single post event");

        let _user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| FieldError::new("Authentication required", juniper::Value::null()))?;

        let handle = ctx.queue_engine
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

        use crate::domains::website_approval::events::WebsiteApprovalEvent;

        let event = website_approval_actions::assess_website(
            website_uuid,
            user.member_id.into_uuid(),
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        ctx.queue_engine.process(event.clone()).await.map_err(to_field_error)?;

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

        let event = chatroom_actions::create_container(
            "ai_chat".to_string(),
            None,
            language,
            member_id,
            with_agent,
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        ctx.queue_engine.process(event.clone()).await.map_err(to_field_error)?;

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

        let event = chatroom_actions::send_message(container_id, content, member_id, None, ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine.process(event.clone()).await.map_err(to_field_error)?;

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

        let event = provider_actions::submit_provider(input, member_id, ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine.process(event.clone()).await.map_err(to_field_error)?;

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

        let provider = provider_actions::update_provider(provider_id, input, ctx.deps())
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

        let event = provider_actions::approve_provider(provider_id, reviewed_by_id, ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine.process(event.clone()).await.map_err(to_field_error)?;

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

        let event = provider_actions::reject_provider(provider_id, reason, reviewed_by_id, ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine.process(event.clone()).await.map_err(to_field_error)?;

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

        let tag = provider_actions::add_provider_tag(
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

        provider_actions::remove_provider_tag(provider_id, tag_id, ctx.deps())
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
        ctx.require_admin()?;
        info!(
            "add_provider_contact mutation called: {} - {}:{}",
            provider_id, contact_type, contact_value
        );

        let contact = provider_actions::add_provider_contact(
            provider_id,
            contact_type,
            contact_value,
            contact_label,
            is_public,
            display_order,
            ctx.deps(),
        )
        .await
        .map_err(to_field_error)?;

        Ok(ContactData::from(contact))
    }

    /// Remove a contact (admin only)
    async fn remove_provider_contact(
        ctx: &GraphQLContext,
        contact_id: String,
    ) -> FieldResult<bool> {
        ctx.require_admin()?;
        info!("remove_provider_contact mutation called: {}", contact_id);

        provider_actions::remove_provider_contact(contact_id, ctx.deps())
            .await
            .map_err(to_field_error)?;

        Ok(true)
    }

    /// Delete a provider (admin only)
    async fn delete_provider(ctx: &GraphQLContext, provider_id: String) -> FieldResult<bool> {
        ctx.require_admin()?;
        info!("delete_provider mutation called: {}", provider_id);

        let event = provider_actions::delete_provider(provider_id, ctx.deps())
            .await
            .map_err(to_field_error)?;

        ctx.queue_engine.process(event).await.map_err(to_field_error)?;

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

        ctx.require_admin()?;

        let post = post_actions::tags::update_post_tags(post_id, action_tags.clone(), &ctx.db_pool)
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

        let tag = post_actions::tags::add_post_tag(
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

        post_actions::tags::remove_post_tag(post_id, tag_id.clone(), &ctx.db_pool)
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

        match extraction_actions::submit_url(&url, query.as_deref(), ctx.deps())
            .await
        {
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

        match extraction_actions::trigger_extraction(&query, site.as_deref(), ctx.deps())
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

        let result = extraction_actions::ingest_site(&site_url, max_pages, ctx.deps())
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
