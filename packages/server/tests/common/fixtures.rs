//! Test fixtures for creating test data.
//!
//! These fixtures use the model methods directly to create test data.

use anyhow::Result;
use server_core::common::{MemberId, PostId, WebsiteId};
use server_core::domains::posts::models::{CreatePost, Post};
use server_core::domains::website::models::{CreateWebsite, Website};
use sqlx::PgPool;

/// Create a test post with pending_approval status
pub async fn create_test_post_pending(
    pool: &PgPool,
    title: &str,
    description: &str,
) -> Result<PostId> {
    let post = Post::create(
        CreatePost::builder()
            .organization_name("Test Organization")
            .title(title)
            .description(description)
            .post_type("service")
            .category("community_support")
            .capacity_status(Some("accepting".to_string()))
            .urgency(Some("medium".to_string()))
            .location(Some("Minneapolis, MN".to_string()))
            .submission_type(Some("user_submitted".to_string()))
            .build(),
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
        CreatePost::builder()
            .organization_name("Test Organization")
            .title(title)
            .description(description)
            .post_type("service")
            .category("community_support")
            .status("active")
            .capacity_status(Some("accepting".to_string()))
            .urgency(Some("medium".to_string()))
            .location(Some("Minneapolis, MN".to_string()))
            .submission_type(Some("admin".to_string()))
            .build(),
        pool,
    )
    .await?;

    Ok(post.id)
}

/// Create a test website (for scraping tests)
pub async fn create_test_website(pool: &PgPool, url: &str) -> Result<WebsiteId> {
    let website =
        Website::create(CreateWebsite::builder().url_or_domain(url).build(), pool).await?;

    Ok(website.id)
}

/// Create an approved test website (ready for crawling)
pub async fn create_test_website_approved(
    pool: &PgPool,
    url: &str,
    admin_member_id: MemberId,
) -> Result<WebsiteId> {
    let website =
        Website::create(CreateWebsite::builder().url_or_domain(url).build(), pool).await?;

    // Approve the website
    Website::approve(website.id, admin_member_id, pool).await?;

    Ok(website.id)
}
