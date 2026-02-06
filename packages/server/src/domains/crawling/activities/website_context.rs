//! Website context helpers for crawling workflows
//!
//! Consolidates common website fetch + validation patterns.

use sqlx::PgPool;

use crate::common::WebsiteId;
use crate::domains::website::models::Website;

/// Fetch an approved website, returning None if not found or not approved.
pub async fn fetch_approved_website(website_id: WebsiteId, pool: &PgPool) -> Option<Website> {
    Website::find_by_id(website_id, pool)
        .await
        .ok()
        .filter(|w| w.status == "approved")
}
