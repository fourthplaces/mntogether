//! Authorization actions for crawling domain
//!
//! Reusable authorization checks for crawl operations.

use anyhow::Result;
use tracing::warn;

use crate::common::auth::{Actor, AdminCapability, HasAuthContext};
use crate::common::MemberId;

/// Check if actor can perform crawl operations.
///
/// Returns Ok(()) on success, or an error on failure.
pub async fn check_crawl_authorization<D: HasAuthContext>(
    requested_by: MemberId,
    is_admin: bool,
    action_name: &str,
    deps: &D,
) -> Result<()> {
    Actor::new(requested_by, is_admin)
        .can(AdminCapability::TriggerScraping)
        .check(deps)
        .await
        .map_err(|auth_err| {
            warn!(
                user_id = %requested_by,
                action = %action_name,
                error = %auth_err,
                "Authorization denied for crawl operation"
            );
            anyhow::anyhow!("Authorization denied for {}: {}", action_name, auth_err)
        })
}
