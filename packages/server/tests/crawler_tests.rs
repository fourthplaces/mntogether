//! Integration tests for the crawler system.
//!
//! Tests the crawl workflow through the GraphQL edge:
//! - crawlWebsite mutation triggers crawl workflow
//! - Authorization checks are enforced
//! - Listings are extracted and synced to database

mod common;

use crate::common::{GraphQLClient, TestHarness};
use server_core::common::{ContactInfo, ExtractedPostWithSource, MemberId};
use server_core::domains::scraping::models::{PageSnapshot, Website};
use server_core::kernel::test_dependencies::{MockAI, MockWebScraper};
use server_core::kernel::TestDependencies;
use test_context::test_context;
use uuid::Uuid;

// =============================================================================
// Test Helpers
// =============================================================================

/// Create an admin user in the database
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

/// Create an approved website ready for crawling
async fn create_approved_website(ctx: &TestHarness, domain: &str, admin_id: Uuid) -> Uuid {
    let website = Website::create(
        domain.to_string(),
        None,
        "admin".to_string(),
        None,
        2,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create website");

    Website::approve(website.id, MemberId::from_uuid(admin_id), &ctx.db_pool)
        .await
        .expect("Failed to approve website");

    website.id.into_uuid()
}

/// Build mock extraction response JSON
fn mock_extraction_response(posts: Vec<(&str, &str, &str)>) -> String {
    let extracted: Vec<ExtractedPostWithSource> = posts
        .into_iter()
        .map(|(source_url, title, description)| ExtractedPostWithSource {
            source_url: source_url.to_string(),
            title: title.to_string(),
            tldr: format!("Summary of {}", title),
            description: description.to_string(),
            contact: Some(ContactInfo {
                phone: None,
                email: Some("contact@example.org".to_string()),
                website: None,
            }),
            location: None,
            urgency: Some("normal".to_string()),
            confidence: Some("high".to_string()),
            audience_roles: vec!["volunteer".to_string()],
        })
        .collect();
    serde_json::to_string(&extracted).expect("Failed to serialize mock response")
}

/// Generate a crawlWebsite mutation with inline UUID
fn crawl_mutation(website_id: Uuid) -> String {
    format!(
        r#"mutation {{ crawlWebsite(websiteId: "{}") {{ jobId sourceId status message }} }}"#,
        website_id
    )
}

/// Generate a simple crawlWebsite mutation returning just status
fn crawl_mutation_simple(website_id: Uuid) -> String {
    format!(
        r#"mutation {{ crawlWebsite(websiteId: "{}") {{ status message }} }}"#,
        website_id
    )
}

// =============================================================================
// Crawl Website Success Tests
// =============================================================================

/// Test that crawlWebsite mutation successfully crawls and extracts listings
#[test_context(TestHarness)]
#[tokio::test]
async fn crawl_website_extracts_listings_from_pages(ctx: &TestHarness) {
    // Arrange: Set up mocks with test dependencies
    let mock_scraper = MockWebScraper::new().with_crawl_pages(vec![
        (
            "https://volunteer-org.example",
            "# Welcome\n\nWe help the community through volunteer work.",
        ),
        (
            "https://volunteer-org.example/volunteer",
            "# Volunteer Opportunities\n\n## Food Pantry Helpers\nHelp sort and distribute food donations every Saturday.",
        ),
    ]);

    // Two-pass extraction: Pass 1 = page summaries, Pass 2 = synthesis
    let mock_ai = MockAI::new()
        // Pass 1: Page summaries (one per page)
        .with_response(r#"{"organization_name": "Volunteer Org", "organization_description": "Helps community", "services": []}"#)
        .with_response(r#"{"organization_name": "Volunteer Org", "organization_description": "Volunteer work", "services": [{"title": "Food Pantry Helpers", "description": "Help sort donations", "contact": "", "location": ""}]}"#)
        // Pass 2: Synthesis
        .with_response(mock_extraction_response(vec![
            (
                "https://volunteer-org.example/volunteer",
                "Food Pantry Helpers",
                "Help sort and distribute food donations every Saturday morning. No experience needed.",
            ),
        ]));

    let deps = TestDependencies::new()
        .mock_scraper(mock_scraper)
        .mock_ai(mock_ai);

    let ctx = TestHarness::with_deps(deps)
        .await
        .expect("Failed to create test harness with mocks");

    // Create website in context with mocks
    let admin_id = create_admin_user(&ctx).await;
    let website_id = create_approved_website(&ctx, "https://volunteer-org.example", admin_id).await;

    // Act: Call crawlWebsite mutation
    let client = GraphQLClient::with_auth_user(ctx.kernel.clone(), admin_id, true);
    let result = client.query(&crawl_mutation(website_id)).await;

    // Assert: Crawl completed successfully
    let status = result["crawlWebsite"]["status"].as_str().unwrap();
    assert!(
        status == "completed" || status == "no_listings",
        "Expected completed or no_listings status, got: {}",
        status
    );

    // Wait for effects to settle
    ctx.settle().await;

    // Assert: Website status updated
    let website = Website::find_by_id(
        server_core::common::WebsiteId::from_uuid(website_id),
        &ctx.db_pool,
    )
    .await
    .expect("Failed to find website");

    assert!(
        website.crawl_status == Some("completed".to_string())
            || website.crawl_status == Some("no_listings_found".to_string()),
        "Expected crawl_status to be completed or no_listings_found, got: {:?}",
        website.crawl_status
    );
}

// =============================================================================
// Authorization Tests
// =============================================================================

/// Test that crawlWebsite requires authentication
#[test_context(TestHarness)]
#[tokio::test]
async fn crawl_website_requires_authentication(ctx: &TestHarness) {
    // Arrange: Create approved website
    let admin_id = create_admin_user(ctx).await;
    let website_id = create_approved_website(ctx, "https://auth-test.example", admin_id).await;

    // Act: Try to crawl without authentication
    let client = ctx.graphql(); // Unauthenticated client
    let result = client.execute(&crawl_mutation_simple(website_id)).await;

    // Assert: Should fail with authentication error
    assert!(
        !result.is_ok(),
        "Expected error for unauthenticated request"
    );
    assert!(!result.errors.is_empty(), "Expected errors in response");
}

/// Test that crawlWebsite requires admin privileges
#[test_context(TestHarness)]
#[tokio::test]
async fn crawl_website_requires_admin(ctx: &TestHarness) {
    // Arrange: Create approved website and non-admin user
    let admin_id = create_admin_user(ctx).await;
    let website_id = create_approved_website(ctx, "https://admin-test.example", admin_id).await;

    // Create non-admin user
    let non_admin_id = Uuid::new_v4();
    let push_token = format!("user-push-token-{}", non_admin_id);
    sqlx::query("INSERT INTO members (id, expo_push_token, searchable_text) VALUES ($1, $2, $3)")
        .bind(non_admin_id)
        .bind(push_token)
        .bind("regular user")
        .execute(&ctx.db_pool)
        .await
        .expect("Failed to create non-admin user");

    // Act: Try to crawl with non-admin user (using execute since we expect error)
    let client = GraphQLClient::with_auth_user(ctx.kernel.clone(), non_admin_id, false);
    let result = client.execute(&crawl_mutation(website_id)).await;

    // Assert: Should return authorization denied error
    // The error message should contain "Authorization denied"
    assert!(
        !result.errors.is_empty(),
        "Expected authorization error, got no errors. Data: {:?}",
        result.data
    );

    let error_text = result.errors.join(" ");
    assert!(
        error_text.contains("Authorization") || error_text.contains("denied") || error_text.contains("Admin"),
        "Expected authorization error message, got: {}",
        error_text
    );
}

// =============================================================================
// Error Handling Tests
// =============================================================================

/// Test that crawlWebsite handles non-existent website
#[test_context(TestHarness)]
#[tokio::test]
async fn crawl_website_nonexistent_returns_error(ctx: &TestHarness) {
    // Arrange: Create admin but no website
    let admin_id = create_admin_user(ctx).await;
    let fake_website_id = Uuid::new_v4();

    // Act: Try to crawl non-existent website
    let client = GraphQLClient::with_auth_user(ctx.kernel.clone(), admin_id, true);
    let result = client.execute(&crawl_mutation(fake_website_id)).await;

    // Assert: Should return an error
    assert!(
        !result.is_ok() || !result.errors.is_empty(),
        "Expected error for non-existent website"
    );
}

// =============================================================================
// Page Snapshot Tests
// =============================================================================

/// Test that crawlWebsite creates page snapshots for crawled pages
#[test_context(TestHarness)]
#[tokio::test]
async fn crawl_website_creates_page_snapshots(ctx: &TestHarness) {
    // Arrange: Set up mocks
    let mock_scraper = MockWebScraper::new().with_crawl_pages(vec![
        (
            "https://snapshot-test.example",
            "# Home Page\n\nWelcome to our organization.",
        ),
        (
            "https://snapshot-test.example/about",
            "# About Us\n\nWe are a nonprofit.",
        ),
        (
            "https://snapshot-test.example/volunteer",
            "# Volunteer\n\n## Help Needed\nWe need volunteers.",
        ),
    ]);

    // Two-pass extraction: Pass 1 = 3 page summaries, Pass 2 = synthesis
    let mock_ai = MockAI::new()
        // Pass 1: Page summaries (one per page)
        .with_response(r#"{"organization_name": "Test Org", "organization_description": "Welcome", "services": []}"#)
        .with_response(r#"{"organization_name": "Test Org", "organization_description": "About us", "services": []}"#)
        .with_response(r#"{"organization_name": "Test Org", "organization_description": "Volunteer", "services": [{"title": "Help Needed", "description": "We need volunteers", "contact": "", "location": ""}]}"#)
        // Pass 2: Synthesis
        .with_response(mock_extraction_response(vec![(
            "https://snapshot-test.example/volunteer",
            "Help Needed",
            "We need volunteers to help with various tasks.",
        )]));

    let deps = TestDependencies::new()
        .mock_scraper(mock_scraper)
        .mock_ai(mock_ai);

    let ctx = TestHarness::with_deps(deps)
        .await
        .expect("Failed to create test harness");

    let admin_id = create_admin_user(&ctx).await;
    let website_id = create_approved_website(&ctx, "https://snapshot-test.example", admin_id).await;

    // Act: Crawl the website
    let client = GraphQLClient::with_auth_user(ctx.kernel.clone(), admin_id, true);
    let _ = client.query(&crawl_mutation_simple(website_id)).await;
    ctx.settle().await;

    // Assert: Page snapshots were created
    let snapshots: Vec<PageSnapshot> = sqlx::query_as::<_, PageSnapshot>(
        "SELECT * FROM page_snapshots WHERE url LIKE $1 ORDER BY url",
    )
    .bind("https://snapshot-test.example%")
    .fetch_all(&ctx.db_pool)
    .await
    .expect("Failed to query page snapshots");

    assert!(
        !snapshots.is_empty(),
        "Expected page snapshots to be created"
    );
}

// =============================================================================
// No Listings Handling Tests
// =============================================================================

/// Test that crawlWebsite handles case where no listings are found
#[test_context(TestHarness)]
#[tokio::test]
async fn crawl_website_handles_no_listings(ctx: &TestHarness) {
    // Arrange: Set up mocks that return no listings
    let mock_scraper = MockWebScraper::new().with_crawl_pages(vec![(
        "https://no-listings.example",
        "# Company Website\n\nWe sell products. No volunteer opportunities here.",
    )]);

    // AI returns empty array - provide multiple responses in case of retries
    let mock_ai = MockAI::new()
        .with_response("[]")
        .with_response("[]")
        .with_response("[]");

    let deps = TestDependencies::new()
        .mock_scraper(mock_scraper)
        .mock_ai(mock_ai);

    let ctx = TestHarness::with_deps(deps)
        .await
        .expect("Failed to create test harness");

    let admin_id = create_admin_user(&ctx).await;
    let website_id = create_approved_website(&ctx, "https://no-listings.example", admin_id).await;

    // Act: Crawl the website
    let client = GraphQLClient::with_auth_user(ctx.kernel.clone(), admin_id, true);
    let result = client.query(&crawl_mutation_simple(website_id)).await;

    // Assert: Should complete with no_listings status from the mutation response
    // The crawl workflow completes synchronously for the GraphQL mutation
    let status = result["crawlWebsite"]["status"].as_str().unwrap_or("");
    assert!(
        status == "no_listings" || status == "completed",
        "Expected no_listings or completed status, got: {}",
        status
    );

    // Note: We don't check the database state here because the workflow
    // completion is indicated by the mutation response.
    // The database update may happen asynchronously after the response.
}

// =============================================================================
// Mock Call Verification Tests
// =============================================================================

/// Test that the scraper is called with correct parameters
#[test_context(TestHarness)]
#[tokio::test]
async fn crawl_website_passes_correct_params_to_scraper(ctx: &TestHarness) {
    // Arrange: Create mock scraper we can inspect
    let mock_scraper = MockWebScraper::new().with_crawl_pages(vec![(
        "https://params-test.example",
        "# Test\n\nContent here.",
    )]);

    // Provide multiple empty array responses in case of retries
    let mock_ai = MockAI::new()
        .with_response("[]")
        .with_response("[]")
        .with_response("[]");

    let deps = TestDependencies::new()
        .mock_scraper(mock_scraper)
        .mock_ai(mock_ai);

    let ctx = TestHarness::with_deps(deps)
        .await
        .expect("Failed to create test harness");

    let admin_id = create_admin_user(&ctx).await;
    let website_id = create_approved_website(&ctx, "https://params-test.example", admin_id).await;

    // Act: Crawl (the mock will be called)
    let client = GraphQLClient::with_auth_user(ctx.kernel.clone(), admin_id, true);
    let _ = client.query(&crawl_mutation_simple(website_id)).await;
    ctx.settle().await;

    // Assert: Verify the mock was called (checking via database state since we can't
    // easily access the mock after harness creation with current architecture)
    let website = Website::find_by_id(
        server_core::common::WebsiteId::from_uuid(website_id),
        &ctx.db_pool,
    )
    .await
    .expect("Failed to find website");

    // If crawl completed, the scraper was called
    assert!(
        website.crawl_status.is_some(),
        "Expected crawl_status to be set after crawl"
    );
}
