//! Integration tests for refreshing page snapshots via GraphQL.
//!
//! Tests the refreshPageSnapshot mutation which re-scrapes a specific website snapshot
//! to update listings when page content changes.

mod common;

use crate::common::{GraphQLClient, TestHarness};
use extraction::{MockIngestor, RawPage};
use server_core::common::MemberId;
use server_core::domains::crawling::models::WebsiteSnapshot;
use server_core::domains::website::models::Website;
use server_core::kernel::test_dependencies::{MockAI, TestDependencies};
use test_context::test_context;
use uuid::Uuid;

// Helper to create admin user for testing
async fn create_admin_user(ctx: &TestHarness) -> Uuid {
    let admin_id = Uuid::new_v4();
    let push_token = format!("admin-push-token-{}", admin_id);
    sqlx::query("INSERT INTO members (id, expo_push_token, searchable_text) VALUES ($1, $2, $3)")
        .bind(admin_id)
        .bind(push_token)
        .bind("admin user")
        .execute(&ctx.db_pool)
        .await
        .expect("Failed to create admin user");
    admin_id
}

// =============================================================================
// Refresh Page Snapshot Tests
// =============================================================================

/// RED: This test will FAIL because refreshPageSnapshot mutation doesn't exist yet
#[test_context(TestHarness)]
#[tokio::test]
async fn refresh_page_snapshot_updates_content(ctx: &TestHarness) {
    // Arrange: Set up initial scrape with first content
    // Note: MockIngestor returns pages by URL, so for refresh we need same URL with new content
    // For this test we'll use a single page that gets returned by fetch_one
    let mock_ingestor = MockIngestor::new().with_page(RawPage::new(
        "https://test-refresh.org",
        "# Food Bank\n\nInitial content",
    ));

    let mock_ai = MockAI::new()
        .with_response(r#"[]"#) // First scrape
        .with_response(r#"[]"#)
        .with_response(r#"[]"#)
        .with_response(r#"[]"#) // Second scrape (refresh)
        .with_response(r#"[]"#)
        .with_response(r#"[]"#);

    let test_deps = TestDependencies::new()
        .mock_ingestor(mock_ingestor)
        .mock_ai(mock_ai);

    let test_ctx = TestHarness::with_deps(test_deps)
        .await
        .expect("Failed to create test harness");

    // Create admin user
    let admin_id = create_admin_user(&test_ctx).await;

    // Create and approve a website
    let website = Website::create(
        "https://test-refresh.org".to_string(),
        None,
        "public_user".to_string(),
        Some("test@example.com".to_string()),
        3,
        &test_ctx.db_pool,
    )
    .await
    .expect("Failed to create website");

    Website::approve(website.id, MemberId::from_uuid(admin_id), &test_ctx.db_pool)
        .await
        .expect("Failed to approve website");

    // Initial scrape
    let client = test_ctx.graphql_with_auth(admin_id, true);

    let scrape_mutation = r#"
        mutation ScrapeOrganization($sourceId: Uuid!) {
            scrapeOrganization(sourceId: $sourceId) {
                status
            }
        }
    "#;

    let mut scrape_vars = juniper::Variables::new();
    scrape_vars.insert(
        "sourceId".to_string(),
        juniper::InputValue::scalar(website.id.to_string()),
    );

    let scrape_result = client.query_with_vars(scrape_mutation, scrape_vars).await;
    assert_eq!(
        scrape_result["scrapeOrganization"]["status"]
            .as_str()
            .unwrap(),
        "completed"
    );

    // Get the page snapshot ID (linked via website_snapshot)
    let snapshots = WebsiteSnapshot::find_by_website(&test_ctx.db_pool, website.id)
        .await
        .expect("Failed to fetch snapshots");
    assert_eq!(
        snapshots.len(),
        1,
        "Expected one snapshot after initial scrape"
    );

    // Get the page snapshot ID from the website snapshot
    let page_snapshot_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT page_snapshot_id
        FROM website_snapshots
        WHERE id = $1
        "#,
    )
    .bind(snapshots[0].id)
    .fetch_one(&test_ctx.db_pool)
    .await
    .expect("Failed to fetch page snapshot id");

    // Get initial page snapshot content hash
    let initial_content_hash = sqlx::query_scalar::<_, Vec<u8>>(
        r#"
        SELECT content_hash
        FROM page_snapshots
        WHERE id = $1
        "#,
    )
    .bind(page_snapshot_id)
    .fetch_one(&test_ctx.db_pool)
    .await
    .expect("Failed to fetch initial page snapshot");

    // Act: Refresh the page snapshot (will get updated content from mock)
    let refresh_mutation = r#"
        mutation RefreshPageSnapshot($snapshotId: String!) {
            refreshPageSnapshot(snapshotId: $snapshotId) {
                status
                message
            }
        }
    "#;

    let mut refresh_vars = juniper::Variables::new();
    refresh_vars.insert(
        "snapshotId".to_string(),
        juniper::InputValue::scalar(page_snapshot_id.to_string()),
    );

    let refresh_result = client.query_with_vars(refresh_mutation, refresh_vars).await;

    // Assert: Refresh completed successfully
    assert_eq!(
        refresh_result["refreshPageSnapshot"]["status"]
            .as_str()
            .unwrap(),
        "completed"
    );

    // Verify page snapshot content hash was updated
    let updated_content_hash = sqlx::query_scalar::<_, Vec<u8>>(
        r#"
        SELECT content_hash
        FROM page_snapshots
        WHERE id = $1
        "#,
    )
    .bind(page_snapshot_id)
    .fetch_one(&test_ctx.db_pool)
    .await
    .expect("Failed to fetch updated page snapshot");

    // Content hashes should be different (content changed)
    assert_ne!(
        initial_content_hash, updated_content_hash,
        "Content hash should change when page content changes"
    );
}

/// Test that refresh requires admin authentication
#[test_context(TestHarness)]
#[tokio::test]
async fn refresh_page_snapshot_requires_admin_auth(ctx: &TestHarness) {
    // Create a website and snapshot
    let admin_id = create_admin_user(ctx).await;
    let website = Website::create(
        "https://test-refresh-auth.org".to_string(),
        None,
        "public_user".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create website");

    Website::approve(website.id, MemberId::from_uuid(admin_id), &ctx.db_pool)
        .await
        .expect("Failed to approve website");

    // Create a page snapshot manually
    let page_snapshot_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO page_snapshots (id, url, content_hash, html, markdown, fetched_via, metadata, crawled_at, extraction_status)
        VALUES ($1, $2, $3, $4, $4, 'test', '{}', NOW(), 'pending')
        RETURNING id
        "#,
    )
    .bind(Uuid::new_v4())
    .bind("https://test-refresh-auth.org")
    .bind(vec![0u8; 32]) // dummy hash
    .bind("# Test content")
    .fetch_one(&ctx.db_pool)
    .await
    .expect("Failed to create page snapshot");

    // Try to refresh without authentication
    let client = ctx.graphql();

    let refresh_mutation = r#"
        mutation RefreshPageSnapshot($snapshotId: String!) {
            refreshPageSnapshot(snapshotId: $snapshotId) {
                status
            }
        }
    "#;

    let mut refresh_vars = juniper::Variables::new();
    refresh_vars.insert(
        "snapshotId".to_string(),
        juniper::InputValue::scalar(page_snapshot_id.to_string()),
    );

    let result = client
        .execute_with_vars(refresh_mutation, refresh_vars)
        .await;

    // Assert: Should return an error for unauthenticated request
    assert!(
        !result.is_ok(),
        "Expected error for unauthenticated request, got success"
    );
    assert!(!result.errors.is_empty(), "Expected errors in response");
}

/// Test that refreshing nonexistent snapshot returns failed status
#[test_context(TestHarness)]
#[tokio::test]
async fn refresh_nonexistent_snapshot_returns_error(ctx: &TestHarness) {
    let admin_id = create_admin_user(ctx).await;
    let client = ctx.graphql_with_auth(admin_id, true);

    let refresh_mutation = r#"
        mutation RefreshPageSnapshot($snapshotId: String!) {
            refreshPageSnapshot(snapshotId: $snapshotId) {
                status
                message
            }
        }
    "#;

    let fake_id = Uuid::new_v4();
    let mut refresh_vars = juniper::Variables::new();
    refresh_vars.insert(
        "snapshotId".to_string(),
        juniper::InputValue::scalar(fake_id.to_string()),
    );

    let result = client
        .execute_with_vars(refresh_mutation, refresh_vars)
        .await;

    // Assert: Should return "failed" status for nonexistent snapshot
    let data = result.data.expect("Expected data in response");
    let status = data["refreshPageSnapshot"]["status"]
        .as_str()
        .expect("Expected status field");
    assert_eq!(
        status, "failed",
        "Expected failed status for nonexistent snapshot"
    );

    let message = data["refreshPageSnapshot"]["message"]
        .as_str()
        .unwrap_or("");
    assert!(
        message.contains("not found"),
        "Expected message to contain 'not found', got: {:?}",
        message
    );
}

/// Test that refresh with unchanged content doesn't create duplicate page snapshot
#[test_context(TestHarness)]
#[tokio::test]
async fn refresh_with_unchanged_content_reuses_page_snapshot(ctx: &TestHarness) {
    // Arrange: Set up scrape with same content twice
    let same_content = "# Food Bank\n\nSame content both times";

    let mock_ingestor =
        MockIngestor::new().with_page(RawPage::new("https://test-unchanged.org", same_content));

    let mock_ai = MockAI::new()
        .with_response(r#"[]"#)
        .with_response(r#"[]"#)
        .with_response(r#"[]"#)
        .with_response(r#"[]"#)
        .with_response(r#"[]"#)
        .with_response(r#"[]"#);

    let test_deps = TestDependencies::new()
        .mock_ingestor(mock_ingestor)
        .mock_ai(mock_ai);

    let test_ctx = TestHarness::with_deps(test_deps)
        .await
        .expect("Failed to create test harness");

    let admin_id = create_admin_user(&test_ctx).await;

    // Create and approve website
    let website = Website::create(
        "https://test-unchanged.org".to_string(),
        None,
        "public_user".to_string(),
        Some("test@example.com".to_string()),
        3,
        &test_ctx.db_pool,
    )
    .await
    .expect("Failed to create website");

    Website::approve(website.id, MemberId::from_uuid(admin_id), &test_ctx.db_pool)
        .await
        .expect("Failed to approve website");

    // Initial scrape
    let client = test_ctx.graphql_with_auth(admin_id, true);

    let scrape_mutation = r#"
        mutation ScrapeOrganization($sourceId: Uuid!) {
            scrapeOrganization(sourceId: $sourceId) {
                status
            }
        }
    "#;

    let mut scrape_vars = juniper::Variables::new();
    scrape_vars.insert(
        "sourceId".to_string(),
        juniper::InputValue::scalar(website.id.to_string()),
    );

    client.query_with_vars(scrape_mutation, scrape_vars).await;

    // Get page snapshot ID via website snapshot
    let snapshots = WebsiteSnapshot::find_by_website(&test_ctx.db_pool, website.id)
        .await
        .expect("Failed to fetch snapshots");

    let page_snapshot_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT page_snapshot_id
        FROM website_snapshots
        WHERE id = $1
        "#,
    )
    .bind(snapshots[0].id)
    .fetch_one(&test_ctx.db_pool)
    .await
    .expect("Failed to fetch page snapshot id");

    // Get initial content hash
    let initial_content_hash = sqlx::query_scalar::<_, Vec<u8>>(
        r#"
        SELECT content_hash
        FROM page_snapshots
        WHERE id = $1
        "#,
    )
    .bind(page_snapshot_id)
    .fetch_one(&test_ctx.db_pool)
    .await
    .expect("Failed to fetch initial content hash");

    // Act: Refresh with same content
    let refresh_mutation = r#"
        mutation RefreshPageSnapshot($snapshotId: String!) {
            refreshPageSnapshot(snapshotId: $snapshotId) {
                status
            }
        }
    "#;

    let mut refresh_vars = juniper::Variables::new();
    refresh_vars.insert(
        "snapshotId".to_string(),
        juniper::InputValue::scalar(page_snapshot_id.to_string()),
    );

    client.query_with_vars(refresh_mutation, refresh_vars).await;

    // Assert: Content hash should be the same (same content)
    let updated_content_hash = sqlx::query_scalar::<_, Vec<u8>>(
        r#"
        SELECT content_hash
        FROM page_snapshots
        WHERE id = $1
        "#,
    )
    .bind(page_snapshot_id)
    .fetch_one(&test_ctx.db_pool)
    .await
    .expect("Failed to fetch updated content hash");

    assert_eq!(
        initial_content_hash, updated_content_hash,
        "Content hash should be the same when content hasn't changed"
    );
}
