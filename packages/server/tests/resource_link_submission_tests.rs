//! Integration tests for resource link submission via GraphQL.
//!
//! Tests the submit_resource_link mutation, which handles:
//! 1. Submitting new domains (should return pending_review status)
//! 2. Submitting pages from approved domains (should start scraping)

mod common;

use crate::common::TestHarness;
use indexmap::IndexMap;
use juniper::{InputValue, Variables};
use server_core::domains::scraping::models::Domain;
use test_context::test_context;

// Helper function to create mutation variables
fn create_resource_link_input(url: &str, context: &str, submitter_contact: &str) -> Variables {
    let mut input_fields = IndexMap::new();
    input_fields.insert("url".to_string(), InputValue::scalar(url.to_string()));
    input_fields.insert("context".to_string(), InputValue::scalar(context.to_string()));
    input_fields.insert("submitterContact".to_string(), InputValue::scalar(submitter_contact.to_string()));

    let mut vars = Variables::new();
    vars.insert("input".to_string(), InputValue::object(input_fields));
    vars
}

// =============================================================================
// Resource Link Submission Tests
// =============================================================================

/// Submitting a new domain should return pending_review status
#[test_context(TestHarness)]
#[tokio::test]
async fn submit_new_domain_returns_pending_review(ctx: &TestHarness) {
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

    let vars = create_resource_link_input(
        "https://newdomain.org/resources",
        "",
        "test@example.com"
    );

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

    // Verify domain was created in database with pending_review status
    let domain = sqlx::query!(
        r#"SELECT id, domain_url, status FROM domains WHERE domain_url = $1"#,
        "https://newdomain.org"
    )
    .fetch_one(&ctx.db_pool)
    .await
    .expect("Domain should be created");

    assert_eq!(domain.status, "pending_review");
}

/// Submitting a page from an approved domain should start scraping
#[test_context(TestHarness)]
#[tokio::test]
async fn submit_approved_domain_returns_pending_status(ctx: &TestHarness) {
    let client = ctx.graphql();

    // Setup: Create an approved domain
    let domain = Domain::create(
        "https://approveddomain.org".to_string(),
        None,
        "test".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create domain");

    // Approve the domain
    sqlx::query!(
        r#"UPDATE domains SET status = 'approved' WHERE id = $1"#,
        domain.id.into_uuid()
    )
    .execute(&ctx.db_pool)
    .await
    .expect("Failed to approve domain");

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
        "https://approveddomain.org/volunteer-page",
        "",
        "test@example.com"
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

/// Submitting same domain twice should not cause duplicate key error
#[test_context(TestHarness)]
#[tokio::test]
async fn submit_existing_pending_domain_returns_pending_review(ctx: &TestHarness) {
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
    let vars1 = create_resource_link_input(
        "https://samedomain.org/page1",
        "",
        "test@example.com"
    );

    let result1 = client.query_with_vars(mutation, vars1).await;

    assert_eq!(
        result1["submitResourceLink"]["status"].as_str().unwrap(),
        "pending_review"
    );

    // Second submission of same domain - should not error
    let vars2 = create_resource_link_input(
        "https://samedomain.org/page1",
        "",
        "test@example.com"
    );

    let result2 = client.query_with_vars(mutation, vars2).await;

    assert_eq!(
        result2["submitResourceLink"]["status"].as_str().unwrap(),
        "pending_review"
    );

    // Verify only one domain was created
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM domains WHERE domain_url = $1"
    )
    .bind("https://samedomain.org")
    .fetch_one(&ctx.db_pool)
    .await
    .expect("Failed to count domains");

    assert_eq!(count, 1, "Should only have one domain record");
}

/// Concurrent submissions of same domain should not cause race condition
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

    let vars1 = create_resource_link_input(
        "https://concurrent.org/page",
        "",
        "test@example.com"
    );

    let vars2 = create_resource_link_input(
        "https://concurrent.org/page",
        "",
        "test@example.com"
    );

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

    // Verify only one domain was created (no duplicate key error)
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM domains WHERE domain_url = $1"
    )
    .bind("https://concurrent.org")
    .fetch_one(&ctx.db_pool)
    .await
    .expect("Failed to count domains");

    assert_eq!(count, 1, "Should only have one domain despite concurrent requests");
}

/// Test: Manual scrape via GraphQL should create domain_snapshot entries
/// This is what happens when admin clicks "Run Scraper" button in UI
#[tokio::test]
async fn test_manual_scrape_creates_domain_snapshot() {
    use server_core::common::MemberId;
    use server_core::domains::scraping::models::{Domain, DomainSnapshot};
    use server_core::kernel::test_dependencies::{MockAI, MockWebScraper, TestDependencies};

    // Setup: Create test harness with mocked AI and web scraper
    let mock_scraper = MockWebScraper::new()
        .with_response("# Test Page\n\nTest volunteer opportunity content");

    let mock_ai = MockAI::new()
        .with_response(r#"[]"#)  // Empty array - AI extraction expects a sequence
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
        .bind("")  // Empty searchable text for test member
        .execute(&ctx.db_pool)
        .await
        .expect("Failed to create test member");

    // Setup: Create an approved domain (no domain_snapshots yet)
    let domain = Domain::create(
        "https://scrapetest.org".to_string(),
        None,
        "system".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create domain");

    // Approve the domain using model method
    Domain::approve(domain.id, reviewer_id, &ctx.db_pool)
        .await
        .expect("Failed to approve domain");

    // Verify no domain_snapshots exist yet using model method
    let initial_snapshots = DomainSnapshot::find_by_domain(&ctx.db_pool, domain.id)
        .await
        .unwrap_or_default();

    assert_eq!(initial_snapshots.len(), 0, "Expected no domain_snapshots before scraping");

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
        .query_with_vars(
            mutation,
            vars!("sourceId" => domain.id.to_string())
        )
        .await;

    // Verify GraphQL response
    assert_eq!(
        result["scrapeOrganization"]["status"].as_str().unwrap(),
        "completed"
    );

    // Verify: domain_snapshot should be created using model method
    let snapshots = DomainSnapshot::find_by_domain(&ctx.db_pool, domain.id)
        .await
        .expect("Failed to fetch domain_snapshots");

    assert_eq!(
        snapshots.len(), 1,
        "Expected exactly one domain_snapshot to be created when manually scraping"
    );

    let domain_snapshot = &snapshots[0];

    // Verify: domain_snapshot should be linked to page_snapshot
    assert!(
        domain_snapshot.page_snapshot_id.is_some(),
        "Expected domain_snapshot to be linked to page_snapshot"
    );
    assert_eq!(
        domain_snapshot.scrape_status, "scraped",
        "Expected scrape_status to be 'scraped'"
    );
    assert_eq!(
        domain_snapshot.page_url, "https://scrapetest.org",
        "Expected page_url to match domain URL"
    );
}
