//! Integration tests for resource link submission via GraphQL.
//!
//! Tests the submit_resource_link mutation, which handles:
//! 1. Submitting new websites (should return pending_review status)
//! 2. Submitting pages from approved websites (should start scraping)

mod common;

use crate::common::TestHarness;
use indexmap::IndexMap;
use juniper::{InputValue, Variables};
use server_core::domains::scraping::models::Website;
use test_context::test_context;

// Helper function to create mutation variables
fn create_resource_link_input(url: &str, context: &str, submitter_contact: &str) -> Variables {
    let mut input_fields = IndexMap::new();
    input_fields.insert("url".to_string(), InputValue::scalar(url.to_string()));
    input_fields.insert(
        "context".to_string(),
        InputValue::scalar(context.to_string()),
    );
    input_fields.insert(
        "submitterContact".to_string(),
        InputValue::scalar(submitter_contact.to_string()),
    );

    let mut vars = Variables::new();
    vars.insert("input".to_string(), InputValue::object(input_fields));
    vars
}

// =============================================================================
// Resource Link Submission Tests
// =============================================================================

/// Submitting a new website should return pending_review status
#[test_context(TestHarness)]
#[tokio::test]
async fn submit_new_website_returns_pending_review(ctx: &TestHarness) {
    let client = ctx.graphql();

    let mutation = r#"
        mutation SubmitResourceLink($input: SubmitResourceLinkInput!) {
            submitResourceLink(input: $input) {
                jobId
                status
                message
            }
        }
    "#;

    let vars =
        create_resource_link_input("https://newwebsite.org/resources", "", "test@example.com");

    let result = client.query_with_vars(mutation, vars).await;

    // Verify GraphQL response
    assert_eq!(
        result["submitResourceLink"]["status"].as_str().unwrap(),
        "pending_review"
    );
    assert!(result["submitResourceLink"]["message"]
        .as_str()
        .unwrap()
        .contains("pending admin approval"));

    // Verify website was created in database with pending_review status
    let website = sqlx::query!(
        r#"SELECT id, url, status FROM websites WHERE url = $1"#,
        "https://newwebsite.org"
    )
    .fetch_one(&ctx.db_pool)
    .await
    .expect("Website should be created");

    assert_eq!(website.status, "pending_review");
}

/// Submitting a page from an approved website should start scraping
#[test_context(TestHarness)]
#[tokio::test]
async fn submit_approved_website_returns_pending_status(ctx: &TestHarness) {
    let client = ctx.graphql();

    // Setup: Create an approved website
    let website = Website::create(
        "https://approvedwebsite.org".to_string(),
        None,
        "test".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create website");

    // Approve the website
    sqlx::query!(
        r#"UPDATE websites SET status = 'approved' WHERE id = $1"#,
        website.id.into_uuid()
    )
    .execute(&ctx.db_pool)
    .await
    .expect("Failed to approve website");

    let mutation = r#"
        mutation SubmitResourceLink($input: SubmitResourceLinkInput!) {
            submitResourceLink(input: $input) {
                jobId
                status
                message
            }
        }
    "#;

    let vars = create_resource_link_input(
        "https://approvedwebsite.org/volunteer-page",
        "",
        "test@example.com",
    );

    let result = client.query_with_vars(mutation, vars).await;

    // Verify GraphQL response - should indicate scraping will happen
    assert_eq!(
        result["submitResourceLink"]["status"].as_str().unwrap(),
        "pending"
    );
    assert!(result["submitResourceLink"]["message"]
        .as_str()
        .unwrap()
        .contains("process it shortly"));
}

/// Submitting same website twice should not cause duplicate key error
#[test_context(TestHarness)]
#[tokio::test]
async fn submit_existing_pending_website_returns_pending_review(ctx: &TestHarness) {
    let client = ctx.graphql();

    let mutation = r#"
        mutation SubmitResourceLink($input: SubmitResourceLinkInput!) {
            submitResourceLink(input: $input) {
                jobId
                status
                message
            }
        }
    "#;

    // First submission
    let vars1 = create_resource_link_input("https://samewebsite.org/page1", "", "test@example.com");

    let result1 = client.query_with_vars(mutation, vars1).await;

    assert_eq!(
        result1["submitResourceLink"]["status"].as_str().unwrap(),
        "pending_review"
    );

    // Second submission of same website - should not error
    let vars2 = create_resource_link_input("https://samewebsite.org/page1", "", "test@example.com");

    let result2 = client.query_with_vars(mutation, vars2).await;

    assert_eq!(
        result2["submitResourceLink"]["status"].as_str().unwrap(),
        "pending_review"
    );

    // Verify only one website was created
    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM websites WHERE url = $1")
        .bind("https://samewebsite.org")
        .fetch_one(&ctx.db_pool)
        .await
        .expect("Failed to count websites");

    assert_eq!(count, 1, "Should only have one website record");
}

/// Concurrent submissions of same website should not cause race condition
#[test_context(TestHarness)]
#[tokio::test]
async fn concurrent_submissions_handled_atomically(ctx: &TestHarness) {
    let client = ctx.graphql();

    let mutation = r#"
        mutation SubmitResourceLink($input: SubmitResourceLinkInput!) {
            submitResourceLink(input: $input) {
                jobId
                status
                message
            }
        }
    "#;

    let vars1 = create_resource_link_input("https://concurrent.org/page", "", "test@example.com");

    let vars2 = create_resource_link_input("https://concurrent.org/page", "", "test@example.com");

    // Send two concurrent requests
    let (result1, result2) = tokio::join!(
        client.query_with_vars(mutation, vars1),
        client.query_with_vars(mutation, vars2)
    );

    // Both should succeed without errors
    assert_eq!(
        result1["submitResourceLink"]["status"].as_str().unwrap(),
        "pending_review"
    );
    assert_eq!(
        result2["submitResourceLink"]["status"].as_str().unwrap(),
        "pending_review"
    );

    // Verify only one website was created (no duplicate key error)
    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM websites WHERE url = $1")
        .bind("https://concurrent.org")
        .fetch_one(&ctx.db_pool)
        .await
        .expect("Failed to count websites");

    assert_eq!(
        count, 1,
        "Should only have one website despite concurrent requests"
    );
}

/// Test: Manual scrape via GraphQL should create website_snapshot entries
/// This is what happens when admin clicks "Run Scraper" button in UI
#[tokio::test]
async fn test_manual_scrape_creates_website_snapshot() {
    use server_core::common::MemberId;
    use server_core::domains::scraping::models::{Website, WebsiteSnapshot};
    use server_core::kernel::test_dependencies::{MockAI, MockWebScraper, TestDependencies};

    // Setup: Create test harness with mocked AI and web scraper
    let mock_scraper =
        MockWebScraper::new().with_response("# Test Page\n\nTest volunteer opportunity content");

    let mock_ai = MockAI::new()
        .with_response(r#"[]"#) // Empty array - AI extraction expects a sequence
        .with_response(r#"[]"#)
        .with_response(r#"[]"#);

    let test_deps = TestDependencies::new()
        .mock_scraper(mock_scraper)
        .mock_ai(mock_ai);

    let ctx = TestHarness::with_deps(test_deps)
        .await
        .expect("Failed to create test harness");

    // Setup: Create a test member (admin reviewer)
    let reviewer_id = MemberId::new();
    sqlx::query("INSERT INTO members (id, expo_push_token, searchable_text) VALUES ($1, $2, $3)")
        .bind(reviewer_id.into_uuid())
        .bind("test-push-token")
        .bind("") // Empty searchable text for test member
        .execute(&ctx.db_pool)
        .await
        .expect("Failed to create test member");

    // Setup: Create an approved website (no website_snapshots yet)
    let website = Website::create(
        "https://scrapetest.org".to_string(),
        None,
        "system".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create website");

    // Approve the website using model method
    Website::approve(website.id, reviewer_id, &ctx.db_pool)
        .await
        .expect("Failed to approve website");

    // Verify no website_snapshots exist yet using model method
    let initial_snapshots = WebsiteSnapshot::find_by_website(&ctx.db_pool, website.id)
        .await
        .unwrap_or_default();

    assert_eq!(
        initial_snapshots.len(),
        0,
        "Expected no website_snapshots before scraping"
    );

    // Use authenticated client (admin) for scraping mutation
    use crate::common::GraphQLClient;
    let client = GraphQLClient::with_auth_user(ctx.kernel.clone(), reviewer_id.into_uuid(), true);

    // Execute: Trigger manual scrape via GraphQL (admin clicks "Run Scraper" button)
    let mutation = r#"
        mutation ScrapeOrganization($sourceId: Uuid!) {
            scrapeOrganization(sourceId: $sourceId) {
                jobId
                status
                message
            }
        }
    "#;

    let result = client
        .query_with_vars(mutation, vars!("sourceId" => website.id.to_string()))
        .await;

    // Verify GraphQL response
    assert_eq!(
        result["scrapeOrganization"]["status"].as_str().unwrap(),
        "completed"
    );

    // Verify: website_snapshot should be created using model method
    let snapshots = WebsiteSnapshot::find_by_website(&ctx.db_pool, website.id)
        .await
        .expect("Failed to fetch website_snapshots");

    assert_eq!(
        snapshots.len(),
        1,
        "Expected exactly one website_snapshot to be created when manually scraping"
    );

    let website_snapshot = &snapshots[0];

    // Verify: website_snapshot should be linked to page_snapshot
    assert!(
        website_snapshot.page_snapshot_id.is_some(),
        "Expected website_snapshot to be linked to page_snapshot"
    );
    assert_eq!(
        website_snapshot.scrape_status, "scraped",
        "Expected scrape_status to be 'scraped'"
    );
    assert_eq!(
        website_snapshot.page_url, "https://scrapetest.org",
        "Expected page_url to match website URL"
    );
}
