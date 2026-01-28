use super::context::GraphQLContext;
use crate::domains::organization::edges::{
    approve_need, edit_and_approve_need, query_need, query_needs, reject_need,
    scrape_organization, submit_need, EditNeedInput, Need, NeedConnection, NeedStatusGql,
    ScrapeResult, SubmitNeedInput,
};
use juniper::{EmptySubscription, FieldResult, RootNode};
use uuid::Uuid;

pub struct Query;

#[juniper::graphql_object(context = GraphQLContext)]
impl Query {
    /// Get a list of needs with filters
    async fn needs(
        ctx: &GraphQLContext,
        status: Option<NeedStatusGql>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> FieldResult<NeedConnection> {
        query_needs(&ctx.pool, status, limit, offset).await
    }

    /// Get a single need by ID
    async fn need(ctx: &GraphQLContext, id: Uuid) -> FieldResult<Option<Need>> {
        query_need(&ctx.pool, id).await
    }
}

pub struct Mutation;

#[juniper::graphql_object(context = GraphQLContext)]
impl Mutation {
    /// Scrape an organization source and extract needs (admin only)
    async fn scrape_organization(
        ctx: &GraphQLContext,
        source_id: Uuid,
    ) -> FieldResult<ScrapeResult> {
        scrape_organization(
            &ctx.pool,
            &ctx.firecrawl_client,
            &ctx.need_extractor,
            source_id,
        )
        .await
    }

    /// Submit a need from a volunteer (public, goes to pending_approval)
    async fn submit_need(
        ctx: &GraphQLContext,
        input: SubmitNeedInput,
        volunteer_id: Option<Uuid>,
    ) -> FieldResult<Need> {
        // TODO: Get IP address from request context
        submit_need(&ctx.pool, input, volunteer_id, None).await
    }

    /// Approve a need (make it visible to volunteers) (admin only)
    async fn approve_need(ctx: &GraphQLContext, need_id: Uuid) -> FieldResult<Need> {
        approve_need(&ctx.pool, need_id).await
    }

    /// Edit and approve a need (fix AI mistakes or improve user content) (admin only)
    async fn edit_and_approve_need(
        ctx: &GraphQLContext,
        need_id: Uuid,
        input: EditNeedInput,
    ) -> FieldResult<Need> {
        edit_and_approve_need(&ctx.pool, need_id, input).await
    }

    /// Reject a need (hide forever) (admin only)
    async fn reject_need(
        ctx: &GraphQLContext,
        need_id: Uuid,
        reason: String,
    ) -> FieldResult<bool> {
        reject_need(&ctx.pool, need_id, reason).await
    }
}

pub type Schema = RootNode<'static, Query, Mutation, EmptySubscription<GraphQLContext>>;

pub fn create_schema() -> Schema {
    Schema::new(Query, Mutation, EmptySubscription::new())
}
