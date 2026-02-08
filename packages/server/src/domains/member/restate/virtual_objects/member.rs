//! Member virtual object
//!
//! Keyed by member_id. Per-member serialized writes, concurrent reads.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::common::auth::restate_auth::require_admin;
use crate::common::EmptyRequest;
use crate::domains::member::activities;
use crate::domains::member::models::member::Member;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatusRequest {
    pub active: bool,
}

impl_restate_serde!(UpdateStatusRequest);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberResult {
    pub id: Uuid,
    pub searchable_text: String,
    pub location_name: Option<String>,
    pub active: bool,
    pub created_at: String,
}

impl_restate_serde!(MemberResult);

impl From<Member> for MemberResult {
    fn from(m: Member) -> Self {
        Self {
            id: m.id,
            searchable_text: m.searchable_text,
            location_name: m.location_name,
            active: m.active,
            created_at: m.created_at.to_rfc3339(),
        }
    }
}

// =============================================================================
// Virtual object definition
// =============================================================================

#[restate_sdk::object]
#[name = "Member"]
pub trait MemberObject {
    async fn update_status(req: UpdateStatusRequest) -> Result<MemberResult, HandlerError>;

    #[shared]
    async fn get(req: EmptyRequest) -> Result<MemberResult, HandlerError>;
}

pub struct MemberObjectImpl {
    deps: Arc<ServerDeps>,
}

impl MemberObjectImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl MemberObject for MemberObjectImpl {
    async fn update_status(
        &self,
        ctx: ObjectContext<'_>,
        req: UpdateStatusRequest,
    ) -> Result<MemberResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let member_id = Uuid::parse_str(ctx.key())
            .map_err(|e| TerminalError::new(format!("Invalid member ID: {}", e)))?;

        ctx.run(|| async {
            activities::update_member_status(member_id, req.active, &self.deps)
                .await
                .map(|_| ())
                .map_err(Into::into)
        })
        .await?;

        let member = Member::find_by_id(member_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(MemberResult::from(member))
    }

    async fn get(
        &self,
        ctx: SharedObjectContext<'_>,
        _req: EmptyRequest,
    ) -> Result<MemberResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;
        let member_id = Uuid::parse_str(ctx.key())
            .map_err(|e| TerminalError::new(format!("Invalid member ID: {}", e)))?;

        let member = Member::find_by_id(member_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(MemberResult::from(member))
    }
}
