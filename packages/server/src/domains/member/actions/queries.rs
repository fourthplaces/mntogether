//! Member query actions
//!
//! All member read operations go through these actions via `engine.activate().process()`.
//! Actions are self-contained: they handle ID parsing and return final models.

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::info;

use crate::common::{build_page_info, AppState, Cursor, ValidatedPaginationArgs};
use crate::domains::member::data::{MemberConnection, MemberData, MemberEdge};
use crate::domains::member::models::member::Member;
use crate::kernel::ServerDeps;

/// Get paginated members with cursor-based pagination (Relay spec)
pub async fn get_members_paginated(
    args: &ValidatedPaginationArgs,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<MemberConnection> {
    ctx.next_state().require_admin()?;

    info!("Getting paginated members");

    let pool = &ctx.deps().db_pool;

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
