//! Integration tests for refreshing page snapshots via GraphQL.
//!
//! Tests the refreshPageSnapshot mutation which re-scrapes a specific domain snapshot
//! to update listings when page content changes.

mod common;

use crate::common::{GraphQLClient, TestHarness};
use server_core::common::MemberId;
use server_core::domains::scraping::models::{Domain, DomainSnapshot};
use server_core::kernel::test_dependencies::{MockAI, MockWebScraper, TestDependencies};
use test_context::test_context;
use uuid::Uuid;

// Helper to create admin user for testing
async fn create_admin_user(ctx: &TestHarness) -> Uuid {
    let admin_id = Uuid::new_v4();
    let push_token = format!("admin-push-token-{}", admin_id);
    sqlx::query(
        "INSERT INTO members (id, expo_push_token, searchable_text) VALUES ($1, $2, $3)"
    )
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
    let mock_scraper = MockWebScraper::new()
        .with_response("# Food Bank\n\nInitial content")
        .with_response("# Food Bank\n\nUPDATED content - new information!");

    let mock_ai = MockAI::new()
        .with_response(r#"[]"#)  // First scrape
        .with_response(r#"[]"#)
        .with_response(r#"[]"#)
        .with_response(r#"[]"#)  // Second scrape (refresh)
        .with_response(r#"[]"#)
        .with_response(r#"[]"#);

    let test_deps = TestDependencies::new()
        .mock_scraper(mock_scraper)
        .mock_ai(mock_ai);

    let test_ctx = TestHarness::with_deps(test_deps)
        .await
        .expect("Failed to create test harness");

    // Create admin user
    let admin_id = create_admin_user(&test_ctx).await;

    // Create and approve a domain
    let domain = Domain::create(
        "https://test-refresh.org".to_string(),
        None,
        "public_user".to_string(),
        Some("test@example.com".to_string()),
        3,
        &test_ctx.db_pool,
    )
    .await
    .expect("Failed to create domain");

    Domain::approve(domain.id, MemberId::from_uuid(admin_id), &test_ctx.db_pool)
        .await
        .expect("Failed to approve domain");

    // Initial scrape
    let client = GraphQLClient::with_auth_user(test_ctx.kernel.clone(), admin_id, true);

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
        juniper::InputValue::scalar(domain.id.to_string()),
    );

    let scrape_result = client.query_with_vars(scrape_mutation, scrape_vars).await;
    assert_eq!(scrape_result["scrapeOrganization"]["status"].as_str().unwrap(), "completed");

    // Get the domain snapshot ID
    let snapshots = DomainSnapshot::find_by_domain(&test_ctx.db_pool, domain.id)
        .await
        .expect("Failed to fetch snapshots");
    assert_eq!(snapshots.len(), 1, "Expected one snapshot after initial scrape");
    let snapshot_id = snapshots[0].id;

    // Get initial page snapshot content hash
    let initial_content_hash = sqlx::query_scalar::<_, Vec<u8>>(
        r#"
        SELECT ps.content_hash
        FROM page_snapshots ps
        JOIN domain_snapshots ds ON ds.page_snapshot_id = ps.id
        WHERE ds.id = $1
        "#
    )
    .bind(snapshot_id)
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
        juniper::InputValue::scalar(snapshot_id.to_string()),
    );

    let refresh_result = client.query_with_vars(refresh_mutation, refresh_vars).await;

    // Assert: Refresh completed successfully
    assert_eq!(
        refresh_result["refreshPageSnapshot"]["status"].as_str().unwrap(),
        "completed"
    );

    // Verify new page snapshot was created with different content hash
    let updated_content_hash = sqlx::query_scalar::<_, Vec<u8>>(
        r#"
        SELECT ps.content_hash
        FROM page_snapshots ps
        JOIN domain_snapshots ds ON ds.page_snapshot_id = ps.id
        WHERE ds.id = $1
        "#
    )
    .bind(snapshot_id)
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
    // Create a domain and snapshot
    let admin_id = create_admin_user(ctx).await;
    let domain = Domain::create(
        "https://test-refresh-auth.org".to_string(),
        None,
        "public_user".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create domain");

    Domain::approve(domain.id, MemberId::from_uuid(admin_id), &ctx.db_pool)
        .await
        .expect("Failed to approve domain");

    // Create a domain snapshot manually
    let snapshot_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO domain_snapshots (domain_id, page_url, scrape_status)
        VALUES ($1, $2, 'pending')
        RETURNING id
        "#,
    )
    .bind(domain.id.into_uuid())
    .bind("https://test-refresh-auth.org")
    .fetch_one(&ctx.db_pool)
    .await
    .expect("Failed to create snapshot");

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
        juniper::InputValue::scalar(snapshot_id.to_string()),
    );

    let result = client.execute_with_vars(refresh_mutation, refresh_vars).await;

    // Assert: Should return an error for unauthenticated request
    assert!(!result.is_ok(), "Expected error for unauthenticated request, got success");
    assert!(!result.errors.is_empty(), "Expected errors in response");
}

/// Test that refreshing nonexistent snapshot returns error
#[test_context(TestHarness)]
#[tokio::test]
async fn refresh_nonexistent_snapshot_returns_error(ctx: &TestHarness) {
    let admin_id = create_admin_user(ctx).await;
    let client = GraphQLClient::with_auth_user(ctx.kernel.clone(), admin_id, true);

    let refresh_mutation = r#"
        mutation RefreshPageSnapshot($snapshotId: String!) {
            refreshPageSnapshot(snapshotId: $snapshotId) {
                status
            }
        }
    "#;

    let fake_id = Uuid::new_v4();
    let mut refresh_vars = juniper::Variables::new();
    refresh_vars.insert(
        "snapshotId".to_string(),
        juniper::InputValue::scalar(fake_id.to_string()),
    );

    let result = client.execute_with_vars(refresh_mutation, refresh_vars).await;

    // Assert: Should return an error for nonexistent snapshot
    assert!(!result.is_ok(), "Expected error for nonexistent snapshot, got success");
    assert!(!result.errors.is_empty(), "Expected errors in response");
}

/// Test that refresh with unchanged content doesn't create duplicate page snapshot
#[test_context(TestHarness)]
#[tokio::test]
async fn refresh_with_unchanged_content_reuses_page_snapshot(ctx: &TestHarness) {
    // Arrange: Set up scrape with same content twice
    let same_content = "# Food Bank\n\nSame content both times";

    let mock_scraper = MockWebScraper::new()
        .with_response(same_content)
        .with_response(same_content);  // Same content for refresh

    let mock_ai = MockAI::new()
        .with_response(r#"[]"#)
        .with_response(r#"[]"#)
        .with_response(r#"[]"#)
        .with_response(r#"[]"#)
        .with_response(r#"[]"#)
        .with_response(r#"[]"#);

    let test_deps = TestDependencies::new()
        .mock_scraper(mock_scraper)
        .mock_ai(mock_ai);

    let test_ctx = TestHarness::with_deps(test_deps)
        .await
        .expect("Failed to create test harness");

    let admin_id = create_admin_user(&test_ctx).await;

    // Create and approve domain
    let domain = Domain::create(
        "https://test-unchanged.org".to_string(),
        None,
        "public_user".to_string(),
        Some("test@example.com".to_string()),
        3,
        &test_ctx.db_pool,
    )
    .await
    .expect("Failed to create domain");

    Domain::approve(domain.id, MemberId::from_uuid(admin_id), &test_ctx.db_pool)
        .await
        .expect("Failed to approve domain");

    // Initial scrape
    let client = GraphQLClient::with_auth_user(test_ctx.kernel.clone(), admin_id, true);

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
        juniper::InputValue::scalar(domain.id.to_string()),
    );

    client.query_with_vars(scrape_mutation, scrape_vars).await;

    // Get snapshot ID
    let snapshots = DomainSnapshot::find_by_domain(&test_ctx.db_pool, domain.id)
        .await
        .expect("Failed to fetch snapshots");
    let snapshot_id = snapshots[0].id;

    // Get initial page snapshot
    let initial_page_snapshot_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT page_snapshot_id
        FROM domain_snapshots
        WHERE id = $1
        "#
    )
    .bind(snapshot_id)
    .fetch_one(&test_ctx.db_pool)
    .await
    .expect("Failed to fetch page snapshot id");

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
        juniper::InputValue::scalar(snapshot_id.to_string()),
    );

    client.query_with_vars(refresh_mutation, refresh_vars).await;

    // Assert: Should still point to same page snapshot (content_hash deduplication)
    let updated_page_snapshot_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT page_snapshot_id
        FROM domain_snapshots
        WHERE id = $1
        "#
    )
    .bind(snapshot_id)
    .fetch_one(&test_ctx.db_pool)
    .await
    .expect("Failed to fetch updated page snapshot id");

    assert_eq!(
        initial_page_snapshot_id, updated_page_snapshot_id,
        "Should reuse same page snapshot when content hasn't changed"
    );
}
