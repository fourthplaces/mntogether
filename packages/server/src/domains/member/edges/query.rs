use juniper::FieldResult;
use tracing::info;
use uuid::Uuid;

use crate::domains::member::{data::MemberData, models::member::Member};
use crate::server::graphql::context::GraphQLContext;

/// Get member by ID
pub async fn get_member(id: String, ctx: &GraphQLContext) -> FieldResult<Option<MemberData>> {
    info!("get_member query called: {}", id);

    let member_id = Uuid::parse_str(&id)?;
    let member = Member::find_by_id(member_id, &ctx.db_pool).await?;

    Ok(Some(MemberData::from(member)))
}

/// Get all active members
pub async fn get_members(ctx: &GraphQLContext) -> FieldResult<Vec<MemberData>> {
    info!("get_members query called");

    let members = Member::find_active(&ctx.db_pool).await?;

    Ok(members.into_iter().map(MemberData::from).collect())
}
