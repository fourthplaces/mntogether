use super::context::GraphQLContext;
use crate::domains::auth::edges as auth_edges;
use crate::domains::member::{data::MemberData, edges as member_edges};
use crate::domains::organization::data::OrganizationData;
use crate::domains::organization::edges::{
    approve_need, archive_post, create_custom_post, edit_and_approve_need, expire_post, query_need,
    query_needs, query_organization_source, query_organization_sources, query_post,
    query_posts_for_need, query_published_posts, reject_need, repost_need, scrape_organization,
    submit_need, submit_resource_link, track_post_click, track_post_view, CreatePostInput, EditNeedInput, Need,
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
        use crate::domains::organization::data::OrganizationData;
        use crate::domains::organization::models::Organization;

        let org_id = Uuid::parse_str(&id)?;
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

    /// Get all active organizations
    async fn organizations(ctx: &GraphQLContext) -> FieldResult<Vec<OrganizationData>> {
        use crate::domains::organization::data::OrganizationData;
        use crate::domains::organization::models::Organization;

        let orgs = Organization::find_active(&ctx.db_pool).await?;
        Ok(orgs.into_iter().map(OrganizationData::from).collect())
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
        use crate::domains::organization::models::Organization;

        let contact_info = if website.is_some() || phone.is_some() {
            let mut map = serde_json::Map::new();
            if let Some(w) = website {
                map.insert("website".to_string(), serde_json::Value::String(w));
            }
            if let Some(p) = phone {
                map.insert("phone".to_string(), serde_json::Value::String(p));
            }
            Some(serde_json::Value::Object(map))
        } else {
            None
        };

        let org = Organization {
            id: Uuid::new_v4(),
            name,
            description,
            contact_info,
            location: None,
            city,
            state: Some("MN".to_string()),
            status: "active".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let created = org.insert(&ctx.db_pool).await?;
        Ok(OrganizationData::from(created))
    }

    /// Add tags to an organization (admin only)
    async fn add_organization_tags(
        ctx: &GraphQLContext,
        organization_id: String,
        tags: Vec<TagInput>,
    ) -> FieldResult<OrganizationData> {
        use crate::domains::organization::data::OrganizationData;
        use crate::domains::organization::models::{Organization, Tag, TagOnOrganization};

        let org_id = Uuid::parse_str(&organization_id)?;

        for tag_input in tags {
            let tag = Tag::find_or_create(&tag_input.kind, &tag_input.value, &ctx.db_pool).await?;
            let _ = TagOnOrganization::create(org_id, tag.id, &ctx.db_pool).await;
        }

        let org = Organization::find_by_id(org_id, &ctx.db_pool).await?;
        Ok(OrganizationData::from(org))
    }
}

pub type Schema = RootNode<'static, Query, Mutation, EmptySubscription<GraphQLContext>>;

pub fn create_schema() -> Schema {
    Schema::new(Query, Mutation, EmptySubscription::new())
}
