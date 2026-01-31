use super::context::GraphQLContext;
use crate::domains::auth::edges as auth_edges;
use crate::domains::domain_approval::data::WebsiteAssessmentData;
use crate::domains::domain_approval::data::WebsiteSearchResultData;
use crate::domains::domain_approval::edges::{
    generate_website_assessment, search_websites_semantic, website_assessment,
};
use crate::domains::listings::data::agent::AgentData;
use crate::domains::listings::data::listing_report::{
    ListingReport as ListingReportData, ListingReportDetail as ListingReportDetailData,
};
use crate::domains::listings::data::{
    ContactInfoInput, EditListingInput, ListingConnection, ListingStatusData, ListingType,
    ScrapeJobResult, SubmitListingInput, SubmitResourceLinkInput, SubmitResourceLinkResult,
};
use crate::domains::listings::edges::{
    approve_listing, approve_website, archive_post, crawl_website, create_agent, delete_listing,
    dismiss_report, edit_and_approve_listing, expire_post, generate_agent_config_from_description,
    get_all_agents, query_listing, query_listing_reports, query_listings, query_pending_websites,
    query_post, query_posts_for_listing, query_published_posts, query_reports_for_listing,
    query_website, query_websites, refresh_page_snapshot, reject_listing, reject_website,
    report_listing, repost_listing, resolve_report, scrape_organization, submit_listing,
    submit_resource_link, suspend_website, track_post_click, track_post_view, trigger_agent_search,
    update_agent, CreateAgentInput, GenerateAgentConfigResult, TriggerSearchResult, UpdateAgentInput,
};
use crate::domains::chatrooms::data::{ContainerData, MessageData};
use crate::domains::chatrooms::edges as chatroom_edges;
use crate::domains::member::{data::MemberData, edges as member_edges};
use crate::domains::organization::data::post_types::RepostResult;
use crate::domains::organization::data::{OrganizationData, PostData, WebsiteData};
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
        status: Option<ListingStatusData>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> FieldResult<ListingConnection> {
        query_listings(&ctx.db_pool, status, limit, offset).await
    }

    /// Get a single listing by ID
    async fn listing(ctx: &GraphQLContext, id: Uuid) -> FieldResult<Option<ListingType>> {
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
    async fn posts_for_listing(
        ctx: &GraphQLContext,
        listing_id: Uuid,
    ) -> FieldResult<Vec<PostData>> {
        query_posts_for_listing(ctx, listing_id).await
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

    /// Get all websites with optional status and agent filters
    /// Status can be: "pending_review", "approved", or null for all
    /// agent_id filters to websites discovered by a specific agent
    async fn websites(
        ctx: &GraphQLContext,
        status: Option<String>,
        agent_id: Option<String>,
    ) -> FieldResult<Vec<WebsiteData>> {
        query_websites(&ctx.db_pool, status, agent_id).await
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
    async fn listing_reports(
        ctx: &GraphQLContext,
        status: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> FieldResult<Vec<ListingReportDetailData>> {
        query_listing_reports(ctx, status, limit, offset).await
    }

    /// Get reports for a specific listing (admin only)
    async fn reports_for_listing(
        ctx: &GraphQLContext,
        listing_id: Uuid,
    ) -> FieldResult<Vec<ListingReportData>> {
        query_reports_for_listing(ctx, listing_id).await
    }

    /// Get all agents (admin only)
    async fn agents(ctx: &GraphQLContext) -> FieldResult<Vec<AgentData>> {
        get_all_agents(ctx).await
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
    async fn submit_listing(
        ctx: &GraphQLContext,
        input: SubmitListingInput,
        member_id: Option<Uuid>,
    ) -> FieldResult<ListingType> {
        // TODO: Get IP address from request context
        submit_listing(ctx, input, member_id, None).await
    }

    /// Submit a resource link (URL) for scraping (public)
    async fn submit_resource_link(
        ctx: &GraphQLContext,
        input: SubmitResourceLinkInput,
    ) -> FieldResult<SubmitResourceLinkResult> {
        submit_resource_link(ctx, input).await
    }

    /// Approve a listing (make it visible to volunteers) (admin only)
    async fn approve_listing(ctx: &GraphQLContext, listing_id: Uuid) -> FieldResult<ListingType> {
        approve_listing(ctx, listing_id).await
    }

    /// Edit and approve a listing (fix AI mistakes or improve user content) (admin only)
    async fn edit_and_approve_listing(
        ctx: &GraphQLContext,
        listing_id: Uuid,
        input: EditListingInput,
    ) -> FieldResult<ListingType> {
        edit_and_approve_listing(ctx, listing_id, input).await
    }

    /// Reject a listing (hide forever) (admin only)
    async fn reject_listing(
        ctx: &GraphQLContext,
        listing_id: Uuid,
        reason: String,
    ) -> FieldResult<bool> {
        reject_listing(ctx, listing_id, reason).await
    }

    /// Delete a listing (admin only)
    async fn delete_listing(ctx: &GraphQLContext, listing_id: Uuid) -> FieldResult<bool> {
        delete_listing(ctx, listing_id).await
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
    async fn repost_listing(ctx: &GraphQLContext, listing_id: Uuid) -> FieldResult<RepostResult> {
        repost_listing(ctx, listing_id).await
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

    /// Refresh a page snapshot by re-scraping (admin only)
    async fn refresh_page_snapshot(
        ctx: &GraphQLContext,
        snapshot_id: String,
    ) -> FieldResult<crate::domains::listings::data::ScrapeJobResult> {
        refresh_page_snapshot(ctx, snapshot_id).await
    }

    /// Generate agent configuration from natural language description (admin only)
    /// Uses AI to convert user intent into search query and extraction instructions
    async fn generate_agent_config(
        ctx: &GraphQLContext,
        description: String,
        location_context: String,
    ) -> FieldResult<GenerateAgentConfigResult> {
        generate_agent_config_from_description(ctx, description, location_context).await
    }

    /// Trigger an agent search manually (admin only)
    /// Immediately dispatches a Tavily search for the specified agent
    async fn trigger_agent_search(
        ctx: &GraphQLContext,
        agent_id: String,
    ) -> FieldResult<TriggerSearchResult> {
        trigger_agent_search(ctx, agent_id).await
    }

    /// Create a new agent (admin only)
    async fn create_agent(ctx: &GraphQLContext, input: CreateAgentInput) -> FieldResult<AgentData> {
        create_agent(ctx, input).await
    }

    /// Update an agent (admin only)
    async fn update_agent(
        ctx: &GraphQLContext,
        agent_id: String,
        input: UpdateAgentInput,
    ) -> FieldResult<AgentData> {
        update_agent(ctx, agent_id, input).await
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
    async fn report_listing(
        ctx: &GraphQLContext,
        listing_id: Uuid,
        reason: String,
        category: String,
        reporter_email: Option<String>,
    ) -> FieldResult<ListingReportData> {
        report_listing(ctx, listing_id, reason, category, reporter_email).await
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
    async fn create_chat(
        ctx: &GraphQLContext,
        language: Option<String>,
    ) -> FieldResult<ContainerData> {
        chatroom_edges::create_chat(ctx, language).await
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
