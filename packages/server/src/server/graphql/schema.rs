use super::context::GraphQLContext;
use crate::domains::auth::edges as auth_edges;
use crate::domains::member::{data::MemberData, edges as member_edges};
use crate::domains::organization::data::OrganizationData;
use crate::domains::organization::edges::{
    add_organization_scrape_url, approve_need, archive_post, create_custom_post, delete_need,
    edit_and_approve_need, expire_post, query_need, query_needs, query_organization_source,
    query_organization_sources, query_post, query_posts_for_need, query_published_posts,
    reject_need, remove_organization_scrape_url, repost_need, scrape_organization, submit_need,
    submit_resource_link, track_post_click, track_post_view, CreatePostInput, EditNeedInput, Need,
    NeedConnection, NeedStatusData, OrganizationSourceData, PostData, RepostResult, ScrapeJobResult,
    SubmitNeedInput, SubmitResourceLinkInput, SubmitResourceLinkResult,
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
    /// Get a list of needs with filters
    async fn needs(
        ctx: &GraphQLContext,
        status: Option<NeedStatusData>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> FieldResult<NeedConnection> {
        query_needs(&ctx.db_pool, status, limit, offset).await
    }

    /// Get a single need by ID
    async fn need(ctx: &GraphQLContext, id: Uuid) -> FieldResult<Option<Need>> {
        query_need(&ctx.db_pool, id).await
    }

    /// Get published posts (for volunteers)
    async fn published_posts(
        ctx: &GraphQLContext,
        limit: Option<i32>,
    ) -> FieldResult<Vec<PostData>> {
        query_published_posts(ctx, limit).await
    }

    /// Get posts for a specific need
    async fn posts_for_need(ctx: &GraphQLContext, need_id: Uuid) -> FieldResult<Vec<PostData>> {
        query_posts_for_need(ctx, need_id).await
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
    async fn organization_sources(ctx: &GraphQLContext) -> FieldResult<Vec<OrganizationSourceData>> {
        query_organization_sources(&ctx.db_pool).await
    }

    /// Get a single organization source by ID
    async fn organization_source(
        ctx: &GraphQLContext,
        id: Uuid,
    ) -> FieldResult<Option<OrganizationSourceData>> {
        query_organization_source(&ctx.db_pool, id).await
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

    /// Get a listing by ID
    async fn listing(ctx: &GraphQLContext, id: String) -> FieldResult<Option<crate::domains::listings::ListingData>> {
        use crate::domains::listings::edges::query_listing;
        query_listing(&ctx.db_pool, id).await
    }

    /// Get listings by type (service, opportunity, business)
    async fn listings_by_type(
        ctx: &GraphQLContext,
        listing_type: String,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> FieldResult<Vec<crate::domains::listings::ListingData>> {
        use crate::domains::listings::edges::query_listings_by_type;
        query_listings_by_type(&ctx.db_pool, listing_type, limit, offset).await
    }

    /// Get listings by category (legal, healthcare, housing, etc.)
    async fn listings_by_category(
        ctx: &GraphQLContext,
        category: String,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> FieldResult<Vec<crate::domains::listings::ListingData>> {
        use crate::domains::listings::edges::query_listings_by_category;
        query_listings_by_category(&ctx.db_pool, category, limit, offset).await
    }

    /// Search listings with multiple filters
    async fn search_listings(
        ctx: &GraphQLContext,
        listing_type: Option<String>,
        category: Option<String>,
        capacity_status: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> FieldResult<Vec<crate::domains::listings::ListingData>> {
        use crate::domains::listings::edges::search_listings;
        search_listings(&ctx.db_pool, listing_type, category, capacity_status, limit, offset).await
    }
}

pub struct Mutation;

#[juniper::graphql_object(context = GraphQLContext)]
impl Mutation {
    /// Scrape an organization source and extract needs (admin only)
    async fn scrape_organization(
        ctx: &GraphQLContext,
        source_id: Uuid,
    ) -> FieldResult<ScrapeJobResult> {
        scrape_organization(ctx, source_id).await
    }

    /// Submit a need from a member (public, goes to pending_approval)
    async fn submit_need(
        ctx: &GraphQLContext,
        input: SubmitNeedInput,
        member_id: Option<Uuid>,
    ) -> FieldResult<Need> {
        // TODO: Get IP address from request context
        submit_need(ctx, input, member_id, None).await
    }

    /// Submit a resource link (URL) for scraping (public)
    async fn submit_resource_link(
        ctx: &GraphQLContext,
        input: SubmitResourceLinkInput,
    ) -> FieldResult<SubmitResourceLinkResult> {
        submit_resource_link(ctx, input).await
    }

    /// Approve a need (make it visible to volunteers) (admin only)
    async fn approve_need(ctx: &GraphQLContext, need_id: Uuid) -> FieldResult<Need> {
        approve_need(ctx, need_id).await
    }

    /// Edit and approve a need (fix AI mistakes or improve user content) (admin only)
    async fn edit_and_approve_need(
        ctx: &GraphQLContext,
        need_id: Uuid,
        input: EditNeedInput,
    ) -> FieldResult<Need> {
        edit_and_approve_need(ctx, need_id, input).await
    }

    /// Reject a need (hide forever) (admin only)
    async fn reject_need(ctx: &GraphQLContext, need_id: Uuid, reason: String) -> FieldResult<bool> {
        reject_need(ctx, need_id, reason).await
    }

    /// Delete a need (admin only)
    async fn delete_need(ctx: &GraphQLContext, need_id: Uuid) -> FieldResult<bool> {
        delete_need(ctx, need_id).await
    }

    /// Add a scrape URL to an organization source (admin only)
    async fn add_organization_scrape_url(
        ctx: &GraphQLContext,
        source_id: Uuid,
        url: String,
    ) -> FieldResult<bool> {
        add_organization_scrape_url(ctx, source_id, url).await
    }

    /// Remove a scrape URL from an organization source (admin only)
    async fn remove_organization_scrape_url(
        ctx: &GraphQLContext,
        source_id: Uuid,
        url: String,
    ) -> FieldResult<bool> {
        remove_organization_scrape_url(ctx, source_id, url).await
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

    /// Create a custom post for a need (admin only)
    async fn create_custom_post(
        ctx: &GraphQLContext,
        input: CreatePostInput,
    ) -> FieldResult<PostData> {
        create_custom_post(ctx, input).await
    }

    /// Repost a need (create new post for existing active need) (admin only)
    async fn repost_need(ctx: &GraphQLContext, need_id: Uuid) -> FieldResult<RepostResult> {
        repost_need(ctx, need_id).await
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

    // =========================================================================
    // Listing Mutations
    // =========================================================================

    /// Create a new listing
    async fn create_listing(
        ctx: &GraphQLContext,
        input: crate::domains::listings::CreateListingInput,
    ) -> FieldResult<crate::domains::listings::ListingData> {
        use crate::domains::listings::edges::create_listing;
        create_listing(&ctx.db_pool, input).await
    }

    /// Update listing status
    async fn update_listing_status(
        ctx: &GraphQLContext,
        listing_id: String,
        status: String,
    ) -> FieldResult<crate::domains::listings::ListingData> {
        use crate::domains::listings::edges::update_listing_status;
        update_listing_status(&ctx.db_pool, listing_id, status).await
    }

    /// Update listing capacity status
    async fn update_listing_capacity(
        ctx: &GraphQLContext,
        listing_id: String,
        capacity_status: String,
    ) -> FieldResult<crate::domains::listings::ListingData> {
        use crate::domains::listings::edges::update_listing_capacity;
        update_listing_capacity(&ctx.db_pool, listing_id, capacity_status).await
    }

    /// Mark listing as verified
    async fn verify_listing(
        ctx: &GraphQLContext,
        listing_id: String,
    ) -> FieldResult<crate::domains::listings::ListingData> {
        use crate::domains::listings::edges::verify_listing;
        verify_listing(&ctx.db_pool, listing_id).await
    }

    /// Add tags to a listing
    async fn add_listing_tags(
        ctx: &GraphQLContext,
        input: crate::domains::listings::AddListingTagsInput,
    ) -> FieldResult<crate::domains::listings::ListingData> {
        use crate::domains::listings::edges::add_listing_tags;
        add_listing_tags(&ctx.db_pool, input).await
    }

    /// Delete a listing
    async fn delete_listing(
        ctx: &GraphQLContext,
        listing_id: String,
    ) -> FieldResult<bool> {
        use crate::domains::listings::edges::delete_listing;
        delete_listing(&ctx.db_pool, listing_id).await
    }
}

pub type Schema = RootNode<'static, Query, Mutation, EmptySubscription<GraphQLContext>>;

pub fn create_schema() -> Schema {
    Schema::new(Query, Mutation, EmptySubscription::new())
}
