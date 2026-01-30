use super::context::GraphQLContext;
use crate::domains::auth::edges as auth_edges;
use crate::domains::member::{data::MemberData, edges as member_edges};
use crate::domains::organization::data::{OrganizationData, PostData, SourceData as DomainData};
use crate::domains::organization::data::post_types::RepostResult;
use crate::domains::listings::edges::{
    approve_listing, delete_listing,
    edit_and_approve_listing, query_listing, query_listings,
    reject_listing,
    scrape_organization, submit_listing, submit_resource_link,
    archive_post, expire_post, query_post, query_posts_for_listing, query_published_posts,
    repost_listing, track_post_click, track_post_view,
    query_organization_source, query_organization_sources,
    query_domains, query_pending_domains,
    approve_domain, reject_domain, suspend_domain, refresh_page_snapshot,
    generate_agent_config_from_description, GenerateAgentConfigResult,
    trigger_agent_search, TriggerSearchResult,
};
use crate::domains::listings::data::{
    ContactInfoInput, EditListingInput, ListingConnection, ListingStatusData,
    ListingType, ScrapeJobResult, SubmitListingInput,
    SubmitResourceLinkInput, SubmitResourceLinkResult,
};
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
    async fn posts_for_listing(ctx: &GraphQLContext, listing_id: Uuid) -> FieldResult<Vec<PostData>> {
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

    /// Get all organization sources (websites to scrape)
    async fn organization_sources(ctx: &GraphQLContext) -> FieldResult<Vec<DomainData>> {
        query_organization_sources(&ctx.db_pool).await
    }

    /// Get a single organization source by ID
    async fn organization_source(
        ctx: &GraphQLContext,
        id: Uuid,
    ) -> FieldResult<Option<DomainData>> {
        query_organization_source(&ctx.db_pool, id).await
    }

    /// Get all domains with optional status filter
    /// Status can be: "pending_review", "approved", or null for all
    async fn domains(
        ctx: &GraphQLContext,
        status: Option<String>,
    ) -> FieldResult<Vec<DomainData>> {
        query_domains(&ctx.db_pool, status).await
    }

    /// Get domains pending review (for admin approval queue)
    async fn pending_domains(ctx: &GraphQLContext) -> FieldResult<Vec<DomainData>> {
        query_pending_domains(&ctx.db_pool).await
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
    async fn reject_listing(ctx: &GraphQLContext, listing_id: Uuid, reason: String) -> FieldResult<bool> {
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
            let tag = Tag::find_or_create(&tag_input.kind, &tag_input.value, None, &ctx.db_pool).await?;
            let _ = Taggable::create_organization_tag(org_id, tag.id, &ctx.db_pool).await;
        }

        let org = Organization::find_by_id(org_id, &ctx.db_pool).await?;
        Ok(OrganizationData::from(org))
    }

    /// Approve a domain for crawling (admin only)
    async fn approve_domain(
        ctx: &GraphQLContext,
        domain_id: String,
    ) -> FieldResult<DomainData> {
        approve_domain(ctx, domain_id).await
    }

    /// Reject a domain submission (admin only)
    async fn reject_domain(
        ctx: &GraphQLContext,
        domain_id: String,
        reason: String,
    ) -> FieldResult<DomainData> {
        reject_domain(ctx, domain_id, reason).await
    }

    /// Suspend a domain (admin only)
    async fn suspend_domain(
        ctx: &GraphQLContext,
        domain_id: String,
        reason: String,
    ) -> FieldResult<DomainData> {
        suspend_domain(ctx, domain_id, reason).await
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
