//! Organization query actions
//!
//! All organization read operations go through these actions via `engine.activate().process()`.
//! Actions are self-contained: they handle ID parsing and return final models.

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::info;

use crate::common::{build_page_info, AppState, Cursor, ValidatedPaginationArgs};
use crate::domains::organization::data::{
    OrganizationConnection, OrganizationData, OrganizationEdge,
};
use crate::domains::organization::models::Organization;
use crate::kernel::ServerDeps;

/// Get paginated organizations with cursor-based pagination (Relay spec)
pub async fn get_organizations_paginated(
    args: &ValidatedPaginationArgs,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<OrganizationConnection> {
    ctx.next_state().require_admin()?;

    info!("Getting paginated organizations");

    let pool = &ctx.deps().db_pool;

    let (organizations, has_more) = Organization::find_paginated(args, pool).await?;
    let total_count = Organization::count(pool).await? as i32;

    let edges: Vec<OrganizationEdge> = organizations
        .into_iter()
        .map(|org| {
            let cursor = Cursor::encode_uuid(org.id.into_uuid());
            OrganizationEdge {
                node: OrganizationData::from(org),
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

    Ok(OrganizationConnection {
        edges,
        page_info,
        total_count,
    })
}
