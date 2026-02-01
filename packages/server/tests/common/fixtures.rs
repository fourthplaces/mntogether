//! Test fixtures for creating test data.
//!
//! These fixtures use the model methods directly to create test data.

use anyhow::Result;
use server_core::common::{MemberId, PostId, WebsiteId};
use server_core::domains::posts::models::Post;
use server_core::domains::scraping::models::Website;
use sqlx::PgPool;

/// Create a test post with pending_approval status
pub async fn create_test_post_pending(
    pool: &PgPool,
    title: &str,
    description: &str,
) -> Result<PostId> {
    let post = Post::create(
        "Test Organization".to_string(),
        title.to_string(),
        description.to_string(),
        None,                                // tldr
        "service".to_string(),               // post_type
        "community_support".to_string(),     // category
        Some("accepting".to_string()),       // capacity_status
        Some("medium".to_string()),          // urgency
        Some("Minneapolis, MN".to_string()), // location
        "pending_approval".to_string(),      // status
        "en".to_string(),                    // source_language
        Some("user_submitted".to_string()),  // submission_type
        None,                                // submitted_by_admin_id
        None,                                // website_id
        None,                                // source_url
        None,                                // organization_id
        pool,
    )
    .await?;

    Ok(post.id)
}

/// Create a test post with active status
pub async fn create_test_post_active(
    pool: &PgPool,
    title: &str,
    description: &str,
) -> Result<PostId> {
    let post = Post::create(
        "Test Organization".to_string(),
        title.to_string(),
        description.to_string(),
        None,                                // tldr
        "service".to_string(),               // post_type
        "community_support".to_string(),     // category
        Some("accepting".to_string()),       // capacity_status
        Some("medium".to_string()),          // urgency
        Some("Minneapolis, MN".to_string()), // location
        "active".to_string(),                // status
        "en".to_string(),                    // source_language
        Some("admin".to_string()),           // submission_type
        None,                                // submitted_by_admin_id
        None,                                // website_id
        None,                                // source_url
        None,                                // organization_id
        pool,
    )
    .await?;

    Ok(post.id)
}

/// Create a test website (for scraping tests)
pub async fn create_test_website(pool: &PgPool, url: &str) -> Result<WebsiteId> {
    let website = Website::create(
        url.to_string(),
        None,                // submitted_by
        "admin".to_string(), // submitter_type
        None,                // submission_context
        2,                   // max_crawl_depth
        pool,
    )
    .await?;

    Ok(website.id)
}

/// Create an approved test website (ready for crawling)
pub async fn create_test_website_approved(
    pool: &PgPool,
    url: &str,
    admin_member_id: MemberId,
) -> Result<WebsiteId> {
    let website = Website::create(
        url.to_string(),
        None,                // submitted_by
        "admin".to_string(), // submitter_type
        None,                // submission_context
        2,                   // max_crawl_depth
        pool,
    )
    .await?;

    // Approve the website
    Website::approve(website.id, admin_member_id, pool).await?;

    Ok(website.id)
}
