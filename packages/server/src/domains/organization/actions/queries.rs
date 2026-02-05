//! Organization query actions
//!
//! Query actions return data directly and are called without process().
//! Auth checks are done at the GraphQL layer.

use anyhow::Result;
use tracing::info;

use crate::common::{build_page_info, Cursor, ValidatedPaginationArgs};
use crate::domains::organization::data::{
    OrganizationConnection, OrganizationData, OrganizationEdge,
};
use crate::domains::organization::models::Organization;
use crate::kernel::ServerDeps;

/// Get paginated organizations with cursor-based pagination (Relay spec)
/// Note: Admin auth is checked at the GraphQL layer
pub async fn get_organizations_paginated(
    args: &ValidatedPaginationArgs,
    deps: &ServerDeps,
) -> Result<OrganizationConnection> {
    info!("Getting paginated organizations");

    let pool = &deps.db_pool;

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
