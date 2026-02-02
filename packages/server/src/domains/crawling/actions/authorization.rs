//! Authorization actions for crawling domain
//!
//! Reusable authorization checks for crawl operations.

use tracing::warn;

use crate::common::auth::{Actor, AdminCapability, HasAuthContext};
use crate::common::MemberId;
use crate::domains::crawling::events::CrawlEvent;

/// Check if actor can perform crawl operations.
///
/// Returns Ok(()) on success, or AuthorizationDenied event on failure.
pub async fn check_crawl_authorization<D: HasAuthContext>(
    requested_by: MemberId,
    is_admin: bool,
    action_name: &str,
    deps: &D,
) -> Result<(), CrawlEvent> {
    if let Err(auth_err) = Actor::new(requested_by, is_admin)
        .can(AdminCapability::TriggerScraping)
        .check(deps)
        .await
    {
        warn!(
            user_id = %requested_by,
            action = %action_name,
            error = %auth_err,
            "Authorization denied for crawl operation"
        );
        return Err(CrawlEvent::AuthorizationDenied {
            user_id: requested_by,
            action: action_name.to_string(),
            reason: auth_err.to_string(),
        });
    }
    Ok(())
}
