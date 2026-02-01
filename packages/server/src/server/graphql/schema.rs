use super::context::GraphQLContext;
use crate::domains::auth::edges as auth_edges;
use crate::domains::domain_approval::data::WebsiteAssessmentData;
use crate::domains::domain_approval::data::WebsiteSearchResultData;
use crate::domains::domain_approval::edges::{
    generate_website_assessment, search_websites_semantic, website_assessment,
};
use crate::domains::posts::data::post_report::{
    PostReport as PostReportData, PostReportDetail as PostReportDetailData,
};
use crate::domains::posts::data::{
    EditPostInput, PostConnection, PostStatusData, PostType,
    ScrapeJobResult, SubmitPostInput, SubmitResourceLinkInput, SubmitResourceLinkResult,
};
use crate::domains::posts::edges::{
    approve_post, approve_website, archive_post, crawl_website,
    deduplicate_posts, delete_post, dismiss_report, edit_and_approve_post, expire_post,
    generate_post_embedding, query_listing, query_post, query_post_reports,
    query_posts, query_pending_websites, query_posts_for_post,
    query_published_posts, query_reports_for_post, query_website, query_websites,
    refresh_page_snapshot, reject_post, reject_website, report_post,
    repost_post, resolve_report, run_discovery_search, scrape_organization, submit_post,
    submit_resource_link, suspend_website, track_post_click, track_post_view,
    DeduplicationResult, DiscoverySearchResult,
};
use crate::domains::website::edges::{
    regenerate_page_posts, regenerate_page_summaries, regenerate_page_summary, regenerate_posts,
};
use crate::domains::website::edges::update_website_crawl_settings;
use crate::domains::chatrooms::data::{ContainerData, MessageData};
use crate::domains::chatrooms::edges as chatroom_edges;
use crate::domains::member::{data::MemberData, edges as member_edges};
use crate::domains::posts::data::types::RepostResult;
use crate::domains::posts::data::PostData;
use crate::domains::organization::data::{OrganizationData, WebsiteData};
use crate::domains::website::data::PageSnapshotData;
use crate::domains::website::edges::{query_page_snapshot, query_page_snapshot_by_url};
use crate::domains::providers::data::{ProviderData, SubmitProviderInput, UpdateProviderInput};
use crate::domains::providers::edges as provider_edges;
use crate::domains::resources::data::{EditResourceInput, ResourceConnection, ResourceData, ResourceStatusData};
use crate::domains::resources::edges::{self as resource_edges, GenerateEmbeddingsResult};
use juniper::{EmptySubscription, FieldResult, RootNode};
use uuid::Uuid;

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

pub struct Query;

#[juniper::graphql_object(context = GraphQLContext)]
impl Query {
    /// Get a list of listings with filters
    async fn listings(
        ctx: &GraphQLContext,
        status: Option<PostStatusData>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> FieldResult<PostConnection> {
        query_posts(&ctx.db_pool, status, limit, offset).await
    }

    /// Get a single listing by ID
    async fn listing(ctx: &GraphQLContext, id: Uuid) -> FieldResult<Option<PostType>> {
        query_listing(&ctx.db_pool, id).await
    }

    /// Get published posts (for volunteers)
    async fn published_posts(
        ctx: &GraphQLContext,
        limit: Option<i32>,
    ) -> FieldResult<Vec<PostData>> {
        query_published_posts(ctx, limit).await
    }

    /// Get posts for a specific listing
    async fn posts_for_post(
        ctx: &GraphQLContext,
        post_id: Uuid,
    ) -> FieldResult<Vec<PostData>> {
        query_posts_for_post(ctx, post_id).await
    }

    /// Get a single post by ID
    async fn post(ctx: &GraphQLContext, id: Uuid) -> FieldResult<Option<PostData>> {
        query_post(ctx, id).await
    }

    /// Get a member by ID
    async fn member(ctx: &GraphQLContext, id: String) -> FieldResult<Option<MemberData>> {
        member_edges::get_member(id, ctx).await
    }

    /// Get all active members
    async fn members(ctx: &GraphQLContext) -> FieldResult<Vec<MemberData>> {
        member_edges::get_members(ctx).await
    }

    /// Get an organization by ID
    async fn organization(
        ctx: &GraphQLContext,
        id: String,
    ) -> FieldResult<Option<OrganizationData>> {
        use crate::common::OrganizationId;
        use crate::domains::organization::data::OrganizationData;
        use crate::domains::organization::models::Organization;

        let org_id = OrganizationId::parse(&id)?;
        match Organization::find_by_id(org_id, &ctx.db_pool).await {
            Ok(org) => Ok(Some(OrganizationData::from(org))),
            Err(_) => Ok(None),
        }
    }

    /// Search organizations by name
    async fn search_organizations(
        ctx: &GraphQLContext,
        query: String,
    ) -> FieldResult<Vec<OrganizationData>> {
        use crate::domains::organization::data::OrganizationData;
        use crate::domains::organization::models::Organization;

        let orgs = Organization::search_by_name(&query, &ctx.db_pool).await?;
        Ok(orgs.into_iter().map(OrganizationData::from).collect())
    }

    /// Search organizations using AI semantic search
    /// Example: "I need immigration legal help in Spanish"
    async fn search_organizations_semantic(
        ctx: &GraphQLContext,
        query: String,
        limit: Option<i32>,
    ) -> FieldResult<Vec<OrganizationMatchData>> {
        use crate::kernel::ai_matching::AIMatchingService;

        // Create AI matching service using shared OpenAI client
        let ai_matching = AIMatchingService::new((*ctx.openai_client).clone());

        // Search with custom limit if provided
        let results = if let Some(lim) = limit {
            ai_matching
                .find_relevant_organizations_with_config(query, 0.7, lim, &ctx.db_pool)
                .await?
        } else {
            ai_matching
                .find_relevant_organizations(query, &ctx.db_pool)
                .await?
        };

        // Convert to GraphQL data types
        Ok(results
            .into_iter()
            .map(|(org, similarity)| OrganizationMatchData {
                organization: OrganizationData::from(org),
                similarity_score: similarity as f64,
            })
            .collect())
    }

    /// Get all websites with optional status filter
    /// Status can be: "pending_review", "approved", or null for all
    async fn websites(
        ctx: &GraphQLContext,
        status: Option<String>,
    ) -> FieldResult<Vec<WebsiteData>> {
        query_websites(&ctx.db_pool, status).await
    }

    /// Get a single website by ID
    async fn website(ctx: &GraphQLContext, id: Uuid) -> FieldResult<Option<WebsiteData>> {
        query_website(&ctx.db_pool, id).await
    }

    /// Get websites pending review (for admin approval queue)
    async fn pending_websites(ctx: &GraphQLContext) -> FieldResult<Vec<WebsiteData>> {
        query_pending_websites(&ctx.db_pool).await
    }

    /// Get all verified organizations
    async fn organizations(ctx: &GraphQLContext) -> FieldResult<Vec<OrganizationData>> {
        use crate::domains::organization::data::OrganizationData;
        use crate::domains::organization::models::Organization;

        let orgs = Organization::find_verified(&ctx.db_pool).await?;
        Ok(orgs.into_iter().map(OrganizationData::from).collect())
    }

    // =========================================================================
    // Listing Queries
    // =========================================================================

    /// Get all listing reports (admin only)
    async fn post_reports(
        ctx: &GraphQLContext,
        status: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> FieldResult<Vec<PostReportDetailData>> {
        query_post_reports(ctx, status, limit, offset).await
    }

    /// Get reports for a specific listing (admin only)
    async fn reports_for_post(
        ctx: &GraphQLContext,
        post_id: Uuid,
    ) -> FieldResult<Vec<PostReportData>> {
        query_reports_for_post(ctx, post_id).await
    }

    /// Get the latest assessment for a website (admin only)
    async fn website_assessment(
        ctx: &GraphQLContext,
        website_id: String,
    ) -> FieldResult<Option<WebsiteAssessmentData>> {
        website_assessment(ctx, website_id).await
    }

    /// Search websites semantically using natural language queries
    ///
    /// Example queries:
    /// - "find me a law firm helping immigrants"
    /// - "food shelves in Minneapolis"
    /// - "mental health services for teenagers"
    async fn search_websites(
        ctx: &GraphQLContext,
        query: String,
        limit: Option<i32>,
        threshold: Option<f64>,
    ) -> FieldResult<Vec<WebsiteSearchResultData>> {
        search_websites_semantic(ctx, query, limit, threshold).await
    }

    // =========================================================================
    // Chatrooms
    // =========================================================================

    /// Get a chat container by ID
    async fn container(ctx: &GraphQLContext, id: String) -> FieldResult<Option<ContainerData>> {
        chatroom_edges::get_container(ctx, id).await
    }

    /// Get messages for a chat container
    async fn messages(ctx: &GraphQLContext, container_id: String) -> FieldResult<Vec<MessageData>> {
        chatroom_edges::get_messages(ctx, container_id).await
    }

    /// Get recent AI chat containers
    async fn recent_chats(
        ctx: &GraphQLContext,
        limit: Option<i32>,
    ) -> FieldResult<Vec<ContainerData>> {
        chatroom_edges::get_recent_chats(ctx, limit).await
    }

    // =========================================================================
    // Providers
    // =========================================================================

    /// Get a provider by ID
    async fn provider(ctx: &GraphQLContext, id: String) -> FieldResult<Option<ProviderData>> {
        provider_edges::get_provider(ctx, id).await
    }

    /// Get all providers with optional filters
    async fn providers(
        ctx: &GraphQLContext,
        status: Option<String>,
        accepting_clients: Option<bool>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> FieldResult<Vec<ProviderData>> {
        provider_edges::get_providers(ctx, status, accepting_clients, limit, offset).await
    }

    /// Get all pending providers (for admin approval queue)
    async fn pending_providers(ctx: &GraphQLContext) -> FieldResult<Vec<ProviderData>> {
        provider_edges::get_pending_providers(ctx).await
    }

    // =========================================================================
    // Resources (new simplified content model)
    // =========================================================================

    /// Get a single resource by ID
    async fn resource(ctx: &GraphQLContext, id: String) -> FieldResult<Option<ResourceData>> {
        resource_edges::get_resource(ctx, id).await
    }

    /// Get resources with pagination and optional status filter
    async fn resources(
        ctx: &GraphQLContext,
        status: Option<ResourceStatusData>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> FieldResult<ResourceConnection> {
        resource_edges::get_resources(ctx, status, limit, offset).await
    }

    /// Get pending resources (for admin approval queue)
    async fn pending_resources(ctx: &GraphQLContext) -> FieldResult<Vec<ResourceData>> {
        resource_edges::get_pending_resources(ctx).await
    }

    /// Get active resources
    async fn active_resources(
        ctx: &GraphQLContext,
        limit: Option<i32>,
    ) -> FieldResult<Vec<ResourceData>> {
        resource_edges::get_active_resources(ctx, limit).await
    }

    // =========================================================================
    // Page Snapshots (scraped page content)
    // =========================================================================

    /// Get a page snapshot by ID
    async fn page_snapshot(ctx: &GraphQLContext, id: Uuid) -> FieldResult<Option<PageSnapshotData>> {
        query_page_snapshot(&ctx.db_pool, id).await
    }

    /// Get a page snapshot by URL
    async fn page_snapshot_by_url(ctx: &GraphQLContext, url: String) -> FieldResult<Option<PageSnapshotData>> {
        query_page_snapshot_by_url(&ctx.db_pool, &url).await
    }
}

pub struct Mutation;

#[juniper::graphql_object(context = GraphQLContext)]
impl Mutation {
    /// Scrape an organization source and extract listings (admin only)
    async fn scrape_organization(
        ctx: &GraphQLContext,
        source_id: Uuid,
    ) -> FieldResult<ScrapeJobResult> {
        scrape_organization(ctx, source_id).await
    }

    /// Crawl a website (multi-page) to discover and extract listings (admin only)
    /// This performs a full crawl using Firecrawl, discovering multiple pages and extracting listings from each.
    async fn crawl_website(ctx: &GraphQLContext, website_id: Uuid) -> FieldResult<ScrapeJobResult> {
        crawl_website(ctx, website_id).await
    }

    /// Submit a listing from a member (public, goes to pending_approval)
    async fn submit_post(
        ctx: &GraphQLContext,
        input: SubmitPostInput,
        member_id: Option<Uuid>,
    ) -> FieldResult<PostType> {
        // TODO: Get IP address from request context
        submit_post(ctx, input, member_id, None).await
    }

    /// Submit a resource link (URL) for scraping (public)
    async fn submit_resource_link(
        ctx: &GraphQLContext,
        input: SubmitResourceLinkInput,
    ) -> FieldResult<SubmitResourceLinkResult> {
        submit_resource_link(ctx, input).await
    }

    /// Approve a listing (make it visible to volunteers) (admin only)
    async fn approve_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<PostType> {
        approve_post(ctx, post_id).await
    }

    /// Edit and approve a listing (fix AI mistakes or improve user content) (admin only)
    async fn edit_and_approve_post(
        ctx: &GraphQLContext,
        post_id: Uuid,
        input: EditPostInput,
    ) -> FieldResult<PostType> {
        edit_and_approve_post(ctx, post_id, input).await
    }

    /// Reject a listing (hide forever) (admin only)
    async fn reject_post(
        ctx: &GraphQLContext,
        post_id: Uuid,
        reason: String,
    ) -> FieldResult<bool> {
        reject_post(ctx, post_id, reason).await
    }

    /// Delete a listing (admin only)
    async fn delete_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
        delete_post(ctx, post_id).await
    }

    /// Send OTP verification code via SMS
    async fn send_verification_code(
        ctx: &GraphQLContext,
        phone_number: String,
    ) -> FieldResult<bool> {
        auth_edges::send_verification_code(phone_number, ctx).await
    }

    /// Verify OTP code and create session
    async fn verify_code(
        ctx: &GraphQLContext,
        phone_number: String,
        code: String,
    ) -> FieldResult<String> {
        auth_edges::verify_code(phone_number, code, ctx).await
    }

    /// Logout (delete session)
    async fn logout(ctx: &GraphQLContext, session_token: String) -> FieldResult<bool> {
        auth_edges::logout(session_token, ctx).await
    }

    /// Repost a listing (create new post for existing active listing) (admin only)
    async fn repost_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<RepostResult> {
        repost_post(ctx, post_id).await
    }

    /// Expire a post (admin only)
    async fn expire_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<PostData> {
        expire_post(ctx, post_id).await
    }

    /// Archive a post (admin only)
    async fn archive_post(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<PostData> {
        archive_post(ctx, post_id).await
    }

    /// Track post view (analytics - public)
    async fn post_viewed(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
        track_post_view(ctx, post_id).await
    }

    /// Track post click (analytics - public)
    async fn post_clicked(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
        track_post_click(ctx, post_id).await
    }

    /// Register a new member (public)
    async fn register_member(
        ctx: &GraphQLContext,
        expo_push_token: String,
        searchable_text: String,
        city: String,
        state: String,
    ) -> FieldResult<MemberData> {
        member_edges::register_member(expo_push_token, searchable_text, city, state, ctx).await
    }

    /// Update member status (activate/deactivate) (admin only)
    async fn update_member_status(
        ctx: &GraphQLContext,
        member_id: String,
        active: bool,
    ) -> FieldResult<MemberData> {
        member_edges::update_member_status(member_id, active, ctx).await
    }

    /// Create a new organization (admin only)
    async fn create_organization(
        ctx: &GraphQLContext,
        name: String,
        description: Option<String>,
        website: Option<String>,
        phone: Option<String>,
        city: Option<String>,
    ) -> FieldResult<OrganizationData> {
        use crate::domains::organization::data::OrganizationData;
        use crate::domains::organization::models::{CreateOrganization, Organization};

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
        use crate::common::OrganizationId;
        use crate::domains::organization::data::OrganizationData;
        use crate::domains::organization::models::Organization;
        use crate::kernel::tag::{Tag, Taggable};

        let org_id = OrganizationId::parse(&organization_id)?;

        for tag_input in tags {
            let tag =
                Tag::find_or_create(&tag_input.kind, &tag_input.value, None, &ctx.db_pool).await?;
            let _ = Taggable::create_organization_tag(org_id, tag.id, &ctx.db_pool).await;
        }

        let org = Organization::find_by_id(org_id, &ctx.db_pool).await?;
        Ok(OrganizationData::from(org))
    }

    /// Approve a website for crawling (admin only)
    async fn approve_website(
        ctx: &GraphQLContext,
        website_id: String,
    ) -> FieldResult<WebsiteData> {
        approve_website(ctx, website_id).await
    }

    /// Reject a website submission (admin only)
    async fn reject_website(
        ctx: &GraphQLContext,
        website_id: String,
        reason: String,
    ) -> FieldResult<WebsiteData> {
        reject_website(ctx, website_id, reason).await
    }

    /// Suspend a website (admin only)
    async fn suspend_website(
        ctx: &GraphQLContext,
        website_id: String,
        reason: String,
    ) -> FieldResult<WebsiteData> {
        suspend_website(ctx, website_id, reason).await
    }

    /// Update website crawl settings (admin only)
    async fn update_website_crawl_settings(
        ctx: &GraphQLContext,
        website_id: String,
        max_pages_per_crawl: i32,
    ) -> FieldResult<WebsiteData> {
        update_website_crawl_settings(ctx, website_id, max_pages_per_crawl).await
    }

    /// Refresh a page snapshot by re-scraping (admin only)
    async fn refresh_page_snapshot(
        ctx: &GraphQLContext,
        snapshot_id: String,
    ) -> FieldResult<crate::domains::posts::data::ScrapeJobResult> {
        refresh_page_snapshot(ctx, snapshot_id).await
    }

    /// Regenerate posts from existing page snapshots (admin only)
    /// Re-runs the AI extraction and sync workflow without re-crawling the website
    async fn regenerate_posts(ctx: &GraphQLContext, website_id: Uuid) -> FieldResult<ScrapeJobResult> {
        regenerate_posts(ctx, website_id).await
    }

    /// Regenerate page summaries for existing snapshots (admin only)
    /// Clears cached summaries and re-runs AI summarization
    async fn regenerate_page_summaries(
        ctx: &GraphQLContext,
        website_id: Uuid,
    ) -> FieldResult<ScrapeJobResult> {
        regenerate_page_summaries(ctx, website_id).await
    }

    /// Regenerate AI summary for a single page snapshot (admin only)
    async fn regenerate_page_summary(
        ctx: &GraphQLContext,
        page_snapshot_id: Uuid,
    ) -> FieldResult<ScrapeJobResult> {
        regenerate_page_summary(ctx, page_snapshot_id).await
    }

    /// Regenerate posts for a single page snapshot (admin only)
    async fn regenerate_page_posts(
        ctx: &GraphQLContext,
        page_snapshot_id: Uuid,
    ) -> FieldResult<ScrapeJobResult> {
        regenerate_page_posts(ctx, page_snapshot_id).await
    }

    /// Run discovery search manually (admin only)
    /// Executes all static discovery queries via Tavily and creates pending websites
    async fn run_discovery_search(ctx: &GraphQLContext) -> FieldResult<DiscoverySearchResult> {
        run_discovery_search(ctx).await
    }

    /// Generate embedding for a single post (admin only)
    async fn generate_post_embedding(ctx: &GraphQLContext, post_id: Uuid) -> FieldResult<bool> {
        generate_post_embedding(ctx, post_id).await
    }

    /// Deduplicate posts using embedding similarity (admin only)
    /// Finds posts with similar embeddings (default threshold: 0.95) and merges them
    /// Keeps the oldest post and deletes newer duplicates
    async fn deduplicate_posts(
        ctx: &GraphQLContext,
        similarity_threshold: Option<f64>,
    ) -> FieldResult<DeduplicationResult> {
        deduplicate_posts(ctx, similarity_threshold).await
    }

    /// Generate a comprehensive assessment report for a website (admin only)
    /// Creates a "background check" style markdown report to help with approval decisions
    async fn generate_website_assessment(
        ctx: &GraphQLContext,
        website_id: String,
    ) -> FieldResult<String> {
        generate_website_assessment(ctx, website_id).await
    }

    /// Report a listing (public or authenticated)
    async fn report_post(
        ctx: &GraphQLContext,
        post_id: Uuid,
        reason: String,
        category: String,
        reporter_email: Option<String>,
    ) -> FieldResult<PostReportData> {
        report_post(ctx, post_id, reason, category, reporter_email).await
    }

    /// Resolve a report (admin only)
    async fn resolve_report(
        ctx: &GraphQLContext,
        report_id: Uuid,
        resolution_notes: Option<String>,
        action_taken: String,
    ) -> FieldResult<bool> {
        resolve_report(ctx, report_id, resolution_notes, action_taken).await
    }

    /// Dismiss a report (admin only)
    async fn dismiss_report(
        ctx: &GraphQLContext,
        report_id: Uuid,
        resolution_notes: Option<String>,
    ) -> FieldResult<bool> {
        dismiss_report(ctx, report_id, resolution_notes).await
    }

    // =========================================================================
    // Chatrooms
    // =========================================================================

    /// Create a new AI chat container
    /// If withAgent is provided, the container will be tagged with the agent config
    /// and the agent will generate a greeting message
    async fn create_chat(
        ctx: &GraphQLContext,
        language: Option<String>,
        with_agent: Option<String>,
    ) -> FieldResult<ContainerData> {
        chatroom_edges::create_chat(ctx, language, with_agent).await
    }

    /// Send a message to a chat container
    /// Triggers agent reply flow for AI chat containers
    async fn send_message(
        ctx: &GraphQLContext,
        container_id: String,
        content: String,
    ) -> FieldResult<MessageData> {
        chatroom_edges::send_message(ctx, container_id, content).await
    }

    /// Signal that the user is typing (for real-time indicators)
    async fn signal_typing(ctx: &GraphQLContext, container_id: String) -> FieldResult<bool> {
        chatroom_edges::signal_typing(ctx, container_id).await
    }

    // =========================================================================
    // Providers
    // =========================================================================

    /// Submit a new provider (public, goes to pending_review)
    async fn submit_provider(
        ctx: &GraphQLContext,
        input: SubmitProviderInput,
        member_id: Option<Uuid>,
    ) -> FieldResult<ProviderData> {
        provider_edges::submit_provider(ctx, input, member_id).await
    }

    /// Update a provider (admin only)
    async fn update_provider(
        ctx: &GraphQLContext,
        provider_id: String,
        input: UpdateProviderInput,
    ) -> FieldResult<ProviderData> {
        provider_edges::update_provider(ctx, provider_id, input).await
    }

    /// Approve a provider (admin only)
    async fn approve_provider(
        ctx: &GraphQLContext,
        provider_id: String,
        reviewed_by_id: Uuid,
    ) -> FieldResult<ProviderData> {
        provider_edges::approve_provider(ctx, provider_id, reviewed_by_id).await
    }

    /// Reject a provider (admin only)
    async fn reject_provider(
        ctx: &GraphQLContext,
        provider_id: String,
        reason: String,
        reviewed_by_id: Uuid,
    ) -> FieldResult<ProviderData> {
        provider_edges::reject_provider(ctx, provider_id, reason, reviewed_by_id).await
    }

    /// Add a tag to a provider (admin only)
    async fn add_provider_tag(
        ctx: &GraphQLContext,
        provider_id: String,
        tag_kind: String,
        tag_value: String,
        display_name: Option<String>,
    ) -> FieldResult<crate::domains::tag::TagData> {
        provider_edges::add_provider_tag(ctx, provider_id, tag_kind, tag_value, display_name).await
    }

    /// Remove a tag from a provider (admin only)
    async fn remove_provider_tag(
        ctx: &GraphQLContext,
        provider_id: String,
        tag_id: String,
    ) -> FieldResult<bool> {
        provider_edges::remove_provider_tag(ctx, provider_id, tag_id).await
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
    ) -> FieldResult<crate::domains::contacts::ContactData> {
        provider_edges::add_provider_contact(
            ctx,
            provider_id,
            contact_type,
            contact_value,
            contact_label,
            is_public,
            display_order,
        )
        .await
    }

    /// Remove a contact (admin only)
    async fn remove_provider_contact(
        ctx: &GraphQLContext,
        contact_id: String,
    ) -> FieldResult<bool> {
        provider_edges::remove_provider_contact(ctx, contact_id).await
    }

    /// Delete a provider (admin only)
    async fn delete_provider(ctx: &GraphQLContext, provider_id: String) -> FieldResult<bool> {
        provider_edges::delete_provider(ctx, provider_id).await
    }

    // =========================================================================
    // Resources (new simplified content model)
    // =========================================================================

    /// Approve a resource (admin only)
    async fn approve_resource(
        ctx: &GraphQLContext,
        resource_id: String,
    ) -> FieldResult<ResourceData> {
        resource_edges::approve_resource(ctx, resource_id).await
    }

    /// Reject a resource (admin only)
    async fn reject_resource(
        ctx: &GraphQLContext,
        resource_id: String,
        reason: String,
    ) -> FieldResult<ResourceData> {
        resource_edges::reject_resource(ctx, resource_id, reason).await
    }

    /// Edit a resource (admin only)
    async fn edit_resource(
        ctx: &GraphQLContext,
        resource_id: String,
        input: EditResourceInput,
    ) -> FieldResult<ResourceData> {
        resource_edges::edit_resource(ctx, resource_id, input).await
    }

    /// Edit and approve a resource in one operation (admin only)
    async fn edit_and_approve_resource(
        ctx: &GraphQLContext,
        resource_id: String,
        input: EditResourceInput,
    ) -> FieldResult<ResourceData> {
        resource_edges::edit_and_approve_resource(ctx, resource_id, input).await
    }

    /// Delete a resource (admin only)
    async fn delete_resource(ctx: &GraphQLContext, resource_id: String) -> FieldResult<bool> {
        resource_edges::delete_resource(ctx, resource_id).await
    }

    /// Generate missing embeddings for resources (admin only)
    /// Processes up to batch_size resources at a time (default 50)
    async fn generate_missing_embeddings(
        ctx: &GraphQLContext,
        batch_size: Option<i32>,
    ) -> FieldResult<GenerateEmbeddingsResult> {
        resource_edges::generate_missing_embeddings(ctx, batch_size).await
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
        use crate::common::PostId;
        use crate::domains::posts::models::Post;
        use crate::kernel::tag::{Tag, Taggable};

        // Check admin auth
        let _user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| juniper::FieldError::new("Authentication required", juniper::Value::null()))?;

        let post_id = PostId::from_uuid(post_id);

        // Clear existing tags
        Taggable::delete_all_for_post(post_id, &ctx.db_pool).await?;

        // Add new tags
        for tag_input in tags {
            let tag = Tag::find_or_create(&tag_input.kind, &tag_input.value, None, &ctx.db_pool).await?;
            Taggable::create_post_tag(post_id, tag.id, &ctx.db_pool).await?;
        }

        // Return updated listing
        let post = Post::find_by_id(post_id, &ctx.db_pool).await?
            .ok_or_else(|| juniper::FieldError::new("Listing not found", juniper::Value::null()))?;
        Ok(PostType::from(post))
    }

    /// Add a single tag to a listing (admin only)
    async fn add_post_tag(
        ctx: &GraphQLContext,
        post_id: Uuid,
        tag_kind: String,
        tag_value: String,
        display_name: Option<String>,
    ) -> FieldResult<crate::domains::tag::TagData> {
        use crate::common::PostId;
        use crate::domains::tag::TagData;
        use crate::kernel::tag::{Tag, Taggable};

        // Check admin auth
        let _user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| juniper::FieldError::new("Authentication required", juniper::Value::null()))?;

        let post_id = PostId::from_uuid(post_id);

        let tag = Tag::find_or_create(&tag_kind, &tag_value, display_name, &ctx.db_pool).await?;
        Taggable::create_post_tag(post_id, tag.id, &ctx.db_pool).await?;

        Ok(TagData::from(tag))
    }

    /// Remove a tag from a listing (admin only)
    async fn remove_post_tag(
        ctx: &GraphQLContext,
        post_id: Uuid,
        tag_id: String,
    ) -> FieldResult<bool> {
        use crate::common::{PostId, TagId};
        use crate::kernel::tag::Taggable;

        // Check admin auth
        let _user = ctx
            .auth_user
            .as_ref()
            .ok_or_else(|| juniper::FieldError::new("Authentication required", juniper::Value::null()))?;

        let post_id = PostId::from_uuid(post_id);
        let tag_id = TagId::parse(&tag_id)?;

        Taggable::delete_post_tag(post_id, tag_id, &ctx.db_pool).await?;
        Ok(true)
    }
}

pub type Schema = RootNode<'static, Query, Mutation, EmptySubscription<GraphQLContext>>;

pub fn create_schema() -> Schema {
    // Note: Juniper 0.16 doesn't support runtime introspection disabling.
    // To disable introspection in production, use a reverse proxy or API gateway
    // to block __schema and __type queries.
    //
    // For development, introspection is useful for GraphQL playground and tooling.
    Schema::new(Query, Mutation, EmptySubscription::new())
}
