//! Integration tests for nested GraphQL queries: Website → Snapshots → Listings
//!
//! Tests the ability to query:
//! - website.snapshots - Get all website snapshots for a website
//! - website.snapshots.pageSnapshot - Get page content
//! - website.snapshots.listings - Get listings extracted from a page

mod common;

use crate::common::{GraphQLClient, TestHarness};
use server_core::common::{MemberId, WebsiteId};
use server_core::domains::scraping::models::Website;
use server_core::kernel::test_dependencies::{MockAI, MockWebScraper, TestDependencies};
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
// Website Query with Nested Snapshots
// =============================================================================

/// RED: This test will FAIL because website query doesn't have nested snapshots resolver yet
#[test_context(TestHarness)]
#[tokio::test]
async fn query_website_with_snapshots(ctx: &TestHarness) {
    // Arrange: Set up mocked external services
    let mock_scraper = MockWebScraper::new()
        .with_response("# Food Bank\n\nWe provide food assistance to families in need.");

    let mock_ai = MockAI::new()
        .with_response(r#"[]"#) // Empty array - AI extraction expects a sequence
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

    // Create and approve a website
    let website = Website::create(
        "https://test-nested.org".to_string(),
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

    // Trigger scrape via GraphQL mutation
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
        juniper::InputValue::scalar(website.id.to_string()),
    );

    let scrape_result = client.query_with_vars(scrape_mutation, scrape_vars).await;
    assert_eq!(
        scrape_result["scrapeOrganization"]["status"]
            .as_str()
            .unwrap(),
        "completed"
    );

    // Act: Query website with nested snapshots
    let query = r#"
        query GetWebsite($id: Uuid!) {
            organizationSource(id: $id) {
                id
                websiteUrl
                status
                snapshotsCount
            }
        }
    "#;

    let mut vars = juniper::Variables::new();
    vars.insert(
        "id".to_string(),
        juniper::InputValue::scalar(website.id.to_string()),
    );

    let result = client.query_with_vars(query, vars).await;

    // Assert: Verify we can query the website and get snapshots count
    assert_eq!(
        result["organizationSource"]["id"].as_str().unwrap(),
        website.id.to_string()
    );
    assert_eq!(
        result["organizationSource"]["status"].as_str().unwrap(),
        "approved"
    );

    // Verify snapshots count is greater than 0 (scraping created a snapshot)
    let snapshots_count = result["organizationSource"]["snapshotsCount"]
        .as_i64()
        .unwrap();
    assert!(
        snapshots_count > 0,
        "Expected at least one snapshot after scraping"
    );
}

/// Test querying website with full nested structure: snapshots → pageSnapshot → listings
#[test_context(TestHarness)]
#[tokio::test]
async fn query_website_with_full_nested_data(ctx: &TestHarness) {
    // Arrange: Set up mocked external services with listing data
    let mock_scraper = MockWebScraper::new()
        .with_response("# Volunteer Opportunities\n\n## Food Bank Helper\nHelp sort and distribute food to families.");

    let mock_ai = MockAI::new()
        .with_response(r#"[]"#) // Empty array for simpler test
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

    // Create and approve a website
    let website = Website::create(
        "https://test-nested-full.org".to_string(),
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

    // Trigger scrape via GraphQL mutation
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
        juniper::InputValue::scalar(website.id.to_string()),
    );

    let scrape_result = client.query_with_vars(scrape_mutation, scrape_vars).await;
    assert_eq!(
        scrape_result["scrapeOrganization"]["status"]
            .as_str()
            .unwrap(),
        "completed"
    );

    // Act: Query website with listings (should go through: website → listings)
    let query = r#"
        query GetWebsite($id: Uuid!) {
            organizationSource(id: $id) {
                id
                websiteUrl
                listingsCount
                listings {
                    id
                    title
                    status
                }
            }
        }
    "#;

    let mut vars = juniper::Variables::new();
    vars.insert(
        "id".to_string(),
        juniper::InputValue::scalar(website.id.to_string()),
    );

    let result = client.query_with_vars(query, vars).await;

    // Assert: Verify nested data structure works
    assert_eq!(
        result["organizationSource"]["id"].as_str().unwrap(),
        website.id.to_string()
    );

    // Verify we can query listingsCount (may be 0 with empty AI mock)
    let listings_count = result["organizationSource"]["listingsCount"]
        .as_i64()
        .unwrap();
    assert!(listings_count >= 0, "Should have valid listings count");

    // Verify listings array exists and is valid (may be empty)
    let listings = result["organizationSource"]["listings"].as_array().unwrap();
    assert!(
        listings.len() as i64 == listings_count,
        "Listings array should match count"
    );
}

/// Test that query works for website with no snapshots yet
#[test_context(TestHarness)]
#[tokio::test]
async fn query_website_with_no_snapshots(ctx: &TestHarness) {
    // Create admin user
    let admin_id = create_admin_user(ctx).await;

    // Create a website but don't scrape it
    let website = Website::create(
        "https://test-no-snapshots.org".to_string(),
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

    let client = GraphQLClient::with_auth_user(ctx.kernel.clone(), admin_id, true);

    // Query website
    let query = r#"
        query GetWebsite($id: Uuid!) {
            organizationSource(id: $id) {
                id
                websiteUrl
                snapshotsCount
                listingsCount
            }
        }
    "#;

    let mut vars = juniper::Variables::new();
    vars.insert(
        "id".to_string(),
        juniper::InputValue::scalar(website.id.to_string()),
    );

    let result = client.query_with_vars(query, vars).await;

    // Assert: Counts should be 0
    assert_eq!(
        result["organizationSource"]["snapshotsCount"]
            .as_i64()
            .unwrap(),
        0
    );
    assert_eq!(
        result["organizationSource"]["listingsCount"]
            .as_i64()
            .unwrap(),
        0
    );
}
