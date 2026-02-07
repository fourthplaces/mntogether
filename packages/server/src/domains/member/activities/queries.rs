//! Member query actions
//!
//! Query actions return data directly and are called without process().
//! Auth checks are done at the GraphQL layer.

use anyhow::Result;
use tracing::info;

use crate::common::{build_page_info, Cursor, ValidatedPaginationArgs};
use crate::domains::member::data::{MemberConnection, MemberData, MemberEdge};
use crate::domains::member::models::member::Member;
use crate::kernel::ServerDeps;

/// Get paginated members with cursor-based pagination (Relay spec)
/// Note: Admin auth is checked at the GraphQL layer
pub async fn get_members_paginated(
    args: &ValidatedPaginationArgs,
    deps: &ServerDeps,
) -> Result<MemberConnection> {
    info!("Getting paginated members");

    let pool = &deps.db_pool;

    let (members, has_more) = Member::find_paginated(args, pool).await?;
    let total_count = Member::count(pool).await? as i32;

    let edges: Vec<MemberEdge> = members
        .into_iter()
        .map(|member| {
            let cursor = Cursor::encode_uuid(member.id);
            MemberEdge {
                node: MemberData::from(member),
                cursor,
            }
        })
        .collect();

    let page_info = build_page_info(
        has_more,
        args,
        edges.first().map(|e| e.cursor.clone()),
        edges.last().map(|e| e.cursor.clone()),
    );

    Ok(MemberConnection {
        edges,
        page_info,
        total_count,
    })
}
