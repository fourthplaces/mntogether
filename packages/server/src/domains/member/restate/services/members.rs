//! Members service (stateless)
//!
//! Cross-member operations: list.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use crate::common::auth::restate_auth::require_admin;
use crate::common::{EmptyRequest, PaginationArgs};
use crate::domains::member::activities;
use crate::domains::member::models::member::Member;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

use crate::domains::member::restate::virtual_objects::member::MemberResult;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListMembersRequest {
    pub first: Option<i32>,
    pub after: Option<String>,
    pub last: Option<i32>,
    pub before: Option<String>,
}

impl_restate_serde!(ListMembersRequest);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberListResult {
    pub members: Vec<MemberResult>,
    pub total_count: i32,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

impl_restate_serde!(MemberListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyResetResult {
    pub members_reset: i64,
}

impl_restate_serde!(WeeklyResetResult);

// =============================================================================
// Service definition
// =============================================================================

#[restate_sdk::service]
#[name = "Members"]
pub trait MembersService {
    async fn list(req: ListMembersRequest) -> Result<MemberListResult, HandlerError>;
    async fn run_weekly_reset(req: EmptyRequest) -> Result<WeeklyResetResult, HandlerError>;
}

pub struct MembersServiceImpl {
    deps: Arc<ServerDeps>,
}

impl MembersServiceImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl MembersService for MembersServiceImpl {
    async fn list(
        &self,
        ctx: Context<'_>,
        req: ListMembersRequest,
    ) -> Result<MemberListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let pagination_args = PaginationArgs {
            first: req.first,
            after: req.after,
            last: req.last,
            before: req.before,
        };
        let validated = pagination_args
            .validate()
            .map_err(|e| TerminalError::new(e))?;

        let connection = activities::get_members_paginated(&validated, &self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(MemberListResult {
            members: connection
                .edges
                .into_iter()
                .filter_map(|e| {
                    uuid::Uuid::parse_str(&e.node.id).ok().map(|id| MemberResult {
                        id,
                        searchable_text: e.node.searchable_text,
                        location_name: e.node.location_name,
                        active: e.node.active,
                        created_at: e.node.created_at.to_rfc3339(),
                    })
                })
                .collect(),
            total_count: connection.total_count,
            has_next_page: connection.page_info.has_next_page,
            has_previous_page: connection.page_info.has_previous_page,
        })
    }

    async fn run_weekly_reset(
        &self,
        ctx: Context<'_>,
        _req: EmptyRequest,
    ) -> Result<WeeklyResetResult, HandlerError> {
        tracing::info!("Running weekly notification reset");

        let pool = &self.deps.db_pool;

        let rows_affected = ctx
            .run(|| async {
                Member::reset_weekly_counts(pool)
                    .await
                    .map_err(Into::into)
            })
            .await?;

        tracing::info!(
            members_reset = rows_affected,
            "Weekly reset complete"
        );

        // Schedule next run (1 week)
        ctx.service_client::<MembersServiceClient>()
            .run_weekly_reset(EmptyRequest {})
            .send_after(Duration::from_secs(7 * 24 * 3600));

        Ok(WeeklyResetResult {
            members_reset: rows_affected as i64,
        })
    }
}
