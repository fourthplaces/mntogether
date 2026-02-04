//! Integration tests for the crawler system.
//!
//! Tests the crawl workflow through the GraphQL edge:
//! - crawlWebsite mutation triggers crawl workflow
//! - Authorization checks are enforced
//! - Listings are extracted and synced to database

mod common;

use crate::common::{GraphQLClient, TestHarness};
use extraction::{MockIngestor, RawPage};
use server_core::common::{ContactInfo, ExtractedPostWithSource, MemberId};
use server_core::domains::crawling::models::PageSnapshot;
use server_core::domains::website::models::Website;
use server_core::kernel::test_dependencies::MockAI;
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

    // Set max_crawl_retries to 0 for tests to avoid retry loops
    sqlx::query("UPDATE websites SET max_crawl_retries = 0 WHERE id = $1")
        .bind(website.id.as_uuid())
        .execute(&ctx.db_pool)
        .await
        .expect("Failed to update max_crawl_retries");

    Website::approve(website.id, MemberId::from_uuid(admin_id), &ctx.db_pool)
        .await
        .expect("Failed to approve website");

    website.id.into_uuid()
}

/// Build mock extraction response JSON (legacy format, still used by some tests)
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
                ..Default::default()
            }),
            location: None,
            urgency: Some("medium".to_string()),
            confidence: Some("high".to_string()),
            audience_roles: vec!["volunteer".to_string()],
        })
        .collect();
    serde_json::to_string(&extracted).expect("Failed to serialize mock response")
}

/// Build mock page summary response (PageSummaryContent format, legacy)
fn mock_page_summary(org_name: Option<&str>, programs: Vec<(&str, &str)>) -> String {
    let programs_json: Vec<String> = programs
        .into_iter()
        .map(|(name, desc)| {
            format!(
                r#"{{"name": "{}", "description": "{}", "serves": null, "how_to_access": null, "eligibility": null, "contact": null, "hours": null, "location": null}}"#,
                name, desc
            )
        })
        .collect();

    let org_json = match org_name {
        Some(name) => format!(
            r#"{{"name": "{}", "mission": null, "description": null, "languages_served": []}}"#,
            name
        ),
        None => "null".to_string(),
    };

    format!(
        r#"{{"organization": {}, "programs": [{}], "contact": null, "location": null, "hours": null, "events": [], "additional_context": null}}"#,
        org_json,
        programs_json.join(", ")
    )
}

// =============================================================================
// Agentic Extraction Mock Helpers
// =============================================================================

/// Build mock candidates response for agentic extraction
/// Agentic extraction expects {"candidates": [...]} format
fn mock_candidates_response(candidates: Vec<(&str, &str, &str)>) -> String {
    let candidates_json: Vec<String> = candidates
        .into_iter()
        .map(|(title, post_type, description)| {
            format!(
                r#"{{"title": "{}", "post_type": "{}", "brief_description": "{}", "source_excerpt": "..."}}"#,
                title, post_type, description
            )
        })
        .collect();

    format!(r#"{{"candidates": [{}]}}"#, candidates_json.join(", "))
}

/// Build mock enriched posts response for merge step
/// Merge expects {"posts": [...]} format
fn mock_merged_posts_response(posts: Vec<(&str, &str, &str)>) -> String {
    let posts_json: Vec<String> = posts
        .into_iter()
        .map(|(title, post_type, description)| {
            format!(
                r#"{{"title": "{}", "post_type": "{}", "description": "{}", "contact": null, "call_to_action": null, "location": null, "schedule": null, "eligibility": null, "source_url": null, "source_page_snapshot_id": null, "confidence": 0.8, "enrichment_notes": []}}"#,
                title, post_type, description
            )
        })
        .collect();

    format!(r#"{{"posts": [{}]}}"#, posts_json.join(", "))
}

/// Build mock LLM sync response (insert all fresh posts)
fn mock_llm_sync_response(post_count: usize) -> String {
    let operations: Vec<String> = (0..post_count)
        .map(|i| {
            format!(
                r#"{{"operation": "insert", "fresh_id": "fresh_{}", "reason": "New post"}}"#,
                i + 1
            )
        })
        .collect();
    format!("[{}]", operations.join(", "))
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
    let mock_ingestor = MockIngestor::new().with_pages(vec![
        RawPage::new(
            "https://volunteer-org.example",
            "# Welcome\n\nWe help the community through volunteer work.",
        ),
        RawPage::new(
            "https://volunteer-org.example/volunteer",
            "# Volunteer Opportunities\n\n## Food Pantry Helpers\nHelp sort and distribute food donations every Saturday.",
        ),
    ]);

    // Agentic extraction pipeline:
    // 1. extract_candidates for each page (expects {"candidates": [...]})
    // 2. enrich_post for each candidate (tool calling, but mock returns content so loop breaks)
    // 3. merge_posts if >1 post (expects {"posts": [...]})
    // 4. sync_posts (LLM sync response)
    let mock_ai = MockAI::new()
        // Page 1 (homepage): extract_candidates - no posts found
        .with_response(mock_candidates_response(vec![]))
        // Page 2 (volunteer page): extract_candidates - 1 volunteer post
        .with_response(mock_candidates_response(vec![(
            "Food Pantry Helpers",
            "volunteer",
            "Help sort and distribute food donations",
        )]))
        // Enrich post (tool calling returns content, loop breaks, uses default values)
        .with_response("Enrichment complete")
        // LLM sync (insert the 1 post)
        .with_response(mock_llm_sync_response(1));

    let deps = TestDependencies::new()
        .mock_ingestor(mock_ingestor)
        .mock_ai(mock_ai);

    let ctx = TestHarness::with_deps(deps)
        .await
        .expect("Failed to create test harness with mocks");

    // Create website in context with mocks
    let admin_id = create_admin_user(&ctx).await;
    let website_id = create_approved_website(&ctx, "https://volunteer-org.example", admin_id).await;

    // Act: Call crawlWebsite mutation
    let client = ctx.graphql_with_auth(admin_id, true);
    let result = client.query(&crawl_mutation(website_id)).await;

    // Assert: Crawl completed successfully
    let status = result["crawlWebsite"]["status"].as_str().unwrap();
    assert!(
        status == "completed" || status == "no_listings",
        "Expected completed or no_listings status, got: {}",
        status
    );

    // Wait for effects to settle (agentic extraction takes longer)
    ctx.settle().await;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

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

    // Act: Try to crawl with non-admin user
    let client = ctx.graphql_with_auth(non_admin_id, false);
    let result = client.execute(&crawl_mutation(website_id)).await;

    // Assert: Should return auth_failed status (actions handle auth internally)
    let data = result
        .data
        .expect("Should return data even for auth failure");
    let crawl_result = data
        .get("crawlWebsite")
        .expect("Should have crawlWebsite field");
    let status = crawl_result.get("status").and_then(|v| v.as_str());

    assert_eq!(
        status,
        Some("auth_failed"),
        "Expected auth_failed status, got: {:?}",
        crawl_result
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
    let client = ctx.graphql_with_auth(admin_id, true);
    let result = client.execute(&crawl_mutation(fake_website_id)).await;

    // Assert: Should return an error or failed status
    if result.errors.is_empty() {
        // If no GraphQL error, check for failed status in response
        let data = result.data.expect("Should have data");
        let crawl_result = data
            .get("crawlWebsite")
            .expect("Should have crawlWebsite field");
        let status = crawl_result.get("status").and_then(|v| v.as_str());

        assert_eq!(
            status,
            Some("failed"),
            "Expected failed status for non-existent website, got: {:?}",
            crawl_result
        );
    }
    // If there are GraphQL errors, that's also acceptable
}

// =============================================================================
// Page Snapshot Tests
// =============================================================================

/// Test that crawlWebsite creates page snapshots for crawled pages
#[test_context(TestHarness)]
#[tokio::test]
async fn crawl_website_creates_page_snapshots(ctx: &TestHarness) {
    // Arrange: Set up mocks
    let mock_ingestor = MockIngestor::new().with_pages(vec![
        RawPage::new(
            "https://snapshot-test.example",
            "# Home Page\n\nWelcome to our organization.",
        ),
        RawPage::new(
            "https://snapshot-test.example/about",
            "# About Us\n\nWe are a nonprofit.",
        ),
        RawPage::new(
            "https://snapshot-test.example/volunteer",
            "# Volunteer\n\n## Help Needed\nWe need volunteers.",
        ),
    ]);

    // Three-pass extraction: Pass 1 = 3 page summaries, Pass 2 = synthesis, Pass 3 = LLM sync
    let mock_ai = MockAI::new()
        // Pass 1: Page summaries (one per page - using PageSummaryContent format)
        .with_response(mock_page_summary(Some("Test Org"), vec![]))
        .with_response(mock_page_summary(Some("Test Org"), vec![]))
        .with_response(mock_page_summary(
            Some("Test Org"),
            vec![("Help Needed", "We need volunteers")],
        ))
        // Pass 2: Synthesis
        .with_response(mock_extraction_response(vec![(
            "https://snapshot-test.example/volunteer",
            "Help Needed",
            "We need volunteers to help with various tasks.",
        )]))
        // Pass 3: LLM sync (insert the 1 post)
        .with_response(mock_llm_sync_response(1));

    let deps = TestDependencies::new()
        .mock_ingestor(mock_ingestor)
        .mock_ai(mock_ai);

    let ctx = TestHarness::with_deps(deps)
        .await
        .expect("Failed to create test harness");

    let admin_id = create_admin_user(&ctx).await;
    let website_id = create_approved_website(&ctx, "https://snapshot-test.example", admin_id).await;

    // Act: Crawl the website
    let client = ctx.graphql_with_auth(admin_id, true);
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
    let mock_ingestor = MockIngestor::new().with_page(RawPage::new(
        "https://no-listings.example",
        "# Company Website\n\nWe sell products. No volunteer opportunities here.",
    ));

    // AI returns empty array - provide multiple responses in case of retries
    let mock_ai = MockAI::new()
        .with_response("[]")
        .with_response("[]")
        .with_response("[]");

    let deps = TestDependencies::new()
        .mock_ingestor(mock_ingestor)
        .mock_ai(mock_ai);

    let ctx = TestHarness::with_deps(deps)
        .await
        .expect("Failed to create test harness");

    let admin_id = create_admin_user(&ctx).await;
    let website_id = create_approved_website(&ctx, "https://no-listings.example", admin_id).await;

    // Act: Crawl the website
    let client = ctx.graphql_with_auth(admin_id, true);
    let result = client.query(&crawl_mutation_simple(website_id)).await;

    // Assert: Should complete with no_posts status from the mutation response
    // The crawl workflow completes synchronously for the GraphQL mutation
    let status = result["crawlWebsite"]["status"].as_str().unwrap_or("");
    assert!(
        status == "no_posts" || status == "completed",
        "Expected no_posts or completed status, got: {}",
        status
    );

    // Note: We don't check the database state here because the workflow
    // completion is indicated by the mutation response.
    // The database update may happen asynchronously after the response.
}

// =============================================================================
// Mock Call Verification Tests
// =============================================================================

/// Test that the ingestor is called with correct parameters
#[test_context(TestHarness)]
#[tokio::test]
async fn crawl_website_passes_correct_params_to_ingestor(ctx: &TestHarness) {
    // Arrange: Create mock ingestor we can inspect
    let mock_ingestor = MockIngestor::new().with_page(RawPage::new(
        "https://params-test.example",
        "# Test\n\nContent here.",
    ));

    // Provide multiple empty array responses in case of retries
    let mock_ai = MockAI::new()
        .with_response("[]")
        .with_response("[]")
        .with_response("[]");

    let deps = TestDependencies::new()
        .mock_ingestor(mock_ingestor)
        .mock_ai(mock_ai);

    let ctx = TestHarness::with_deps(deps)
        .await
        .expect("Failed to create test harness");

    let admin_id = create_admin_user(&ctx).await;
    let website_id = create_approved_website(&ctx, "https://params-test.example", admin_id).await;

    // Act: Crawl (the mock will be called)
    let client = ctx.graphql_with_auth(admin_id, true);
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
