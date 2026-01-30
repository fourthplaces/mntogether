//! Test fixtures for creating test data.
//!
//! NOTE: These fixtures are for legacy organization_needs tests.
//! New tests should use model methods directly.

use anyhow::Result;
use chrono::Utc;
// TODO: Update these imports after organization domain refactor
// use server_core::domains::organization::models::{
//     NeedStatus, OrganizationNeed, OrganizationSource,
// };
use sqlx::PgPool;
use uuid::Uuid;

// Temporary NeedStatus enum for fixtures
#[allow(dead_code)]
enum NeedStatus {
    PendingApproval,
    Active,
}

impl NeedStatus {
    fn to_string(&self) -> String {
        match self {
            Self::PendingApproval => "pending_approval".to_string(),
            Self::Active => "active".to_string(),
        }
    }
}

/// Create a test organization source
#[allow(dead_code)]
pub async fn create_test_source(
    _pool: &PgPool,
    _organization_name: &str,
    _source_url: &str,
) -> Result<Uuid> {
    // Disabled - table no longer exists after domain refactor
    unimplemented!("create_test_source is deprecated - use Domain::create instead")
}

/// Create a test need with pending_approval status
#[allow(dead_code)]
pub async fn create_test_need_pending(
    _pool: &PgPool,
    _source_id: Option<Uuid>,
    _title: &str,
    _description: &str,
) -> Result<Uuid> {
    unimplemented!("create_test_need_pending is deprecated - use model methods instead")
}

/// Create a test need with active status
#[allow(dead_code)]
pub async fn create_test_need_active(
    _pool: &PgPool,
    _title: &str,
    _description: &str,
) -> Result<Uuid> {
    unimplemented!("create_test_need_active is deprecated - use model methods instead")
}

/// Create a full test need with all fields
#[allow(dead_code)]
pub async fn create_test_need_full(
    _pool: &PgPool,
    _organization_name: &str,
    _title: &str,
    _description: &str,
    _tldr: &str,
    _contact_json: Option<serde_json::Value>,
    _urgency: Option<&str>,
    _status: NeedStatus,
) -> Result<Uuid> {
    unimplemented!("create_test_need_full is deprecated - use model methods instead")
}

/// Clean all needs from database (for test isolation)
#[allow(dead_code)]
pub async fn clean_needs(_pool: &PgPool) -> Result<()> {
    unimplemented!("clean_needs is deprecated")
}

/// Clean all sources from database (for test isolation)
#[allow(dead_code)]
pub async fn clean_sources(_pool: &PgPool) -> Result<()> {
    unimplemented!("clean_sources is deprecated - use Domain::delete or database cleanup")
}
