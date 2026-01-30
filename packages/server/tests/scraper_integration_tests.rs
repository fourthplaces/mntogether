mod common;

use common::harness::TestHarness;
use server_core::domains::listings::events::ListingEvent;
use server_core::domains::scraping::models::Domain;
use server_core::kernel::test_dependencies::MockWebScraper;
use server_core::kernel::TestDependencies;
use test_context::test_context;
use uuid::Uuid;

// =============================================================================
// Tests: Domain scraping with page_snapshots
// =============================================================================

#[test_context(TestHarness)]
#[tokio::test]
async fn test_scrape_source_creates_page_snapshot(ctx: &TestHarness) {
    // Setup: Create a test domain
    let domain = Domain::create(
        "https://example.org".to_string(),
        None,
        "test".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create domain");

    // Configure mock web scraper with test content
    let mock_scraper = MockWebScraper::new().with_response(
        r#"
# Food Bank Volunteers Needed

We are seeking volunteers to help at our food bank.

## Requirements
- Available on weekends
- Able to lift 25 lbs

Contact: foodbank@example.org
"#,
    );

    let test_deps = TestDependencies::new()
        .mock_scraper(mock_scraper);

    let test_harness = TestHarness::with_deps(test_deps)
        .await
        .expect("Failed to create test harness with custom deps");

    // Execute: Trigger scrape via event bus
    test_harness
        .bus()
        .send(ListingEvent::ScrapeSourceRequested {
            source_id: domain.id,
            job_id: Uuid::new_v4(),
            requested_by: Uuid::new_v4(),
            is_admin: true,
        })
        .await
        .expect("Failed to send scrape event");

    // Wait for effects to settle
    test_harness.settle().await;

    // Verify: Check that page_snapshot was created
    let page_snapshot_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM page_snapshots WHERE url = $1",
    )
    .bind("https://example.org")
    .fetch_one(&test_harness.db_pool)
    .await
    .expect("Failed to query page_snapshots");

    assert_eq!(
        page_snapshot_count, 1,
        "Expected exactly one page_snapshot to be created"
    );

    // Verify: Check that page_snapshot has the scraped content
    let page_snapshot = sqlx::query!(
        r#"
        SELECT id, url, markdown, crawled_at, extraction_status
        FROM page_snapshots
        WHERE url = $1
        "#,
        "https://example.org"
    )
    .fetch_one(&test_harness.db_pool)
    .await
    .expect("Failed to fetch page_snapshot");

    assert!(page_snapshot.markdown.is_some(), "Expected markdown content");
    assert!(
        page_snapshot
            .markdown
            .unwrap()
            .contains("Food Bank Volunteers Needed"),
        "Expected markdown to contain title"
    );
    assert_eq!(
        page_snapshot.extraction_status,
        Some("pending".to_string()),
        "Expected extraction_status to be pending"
    );
}

#[test_context(TestHarness)]
#[tokio::test]
async fn test_scrape_deduplication_by_content_hash(ctx: &TestHarness) {
    // Setup: Create a test domain
    let domain = Domain::create(
        "https://example.org".to_string(),
        None,
        "test".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create domain");

    let scrape_content = r#"
# Test Content
This is test content that will be scraped twice.
"#;

    // Configure mock web scraper with identical content for both scrapes
    let mock_scraper = MockWebScraper::new()
        .with_response(scrape_content)
        .with_response(scrape_content); // Same content twice

    let test_deps = TestDependencies::new()
        .mock_scraper(mock_scraper);

    let test_harness = TestHarness::with_deps(test_deps)
        .await
        .expect("Failed to create test harness with custom deps");

    // Execute: Trigger first scrape
    test_harness
        .bus()
        .send(ListingEvent::ScrapeSourceRequested {
            source_id: domain.id,
            job_id: Uuid::new_v4(),
            requested_by: Uuid::new_v4(),
            is_admin: true,
        })
        .await
        .expect("Failed to send first scrape event");

    test_harness.settle().await;

    // Execute: Trigger second scrape with identical content
    test_harness
        .bus()
        .send(ListingEvent::ScrapeSourceRequested {
            source_id: domain.id,
            job_id: Uuid::new_v4(),
            requested_by: Uuid::new_v4(),
            is_admin: true,
        })
        .await
        .expect("Failed to send second scrape event");

    test_harness.settle().await;

    // Verify: Should only have ONE page_snapshot (deduplication by content_hash)
    let page_snapshot_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM page_snapshots WHERE url = $1",
    )
    .bind("https://example.org")
    .fetch_one(&test_harness.db_pool)
    .await
    .expect("Failed to query page_snapshots");

    assert_eq!(
        page_snapshot_count, 1,
        "Expected deduplication - only one page_snapshot should exist for identical content"
    );
}

#[test_context(TestHarness)]
#[tokio::test]
async fn test_scrape_different_content_creates_new_snapshot(ctx: &TestHarness) {
    // Setup: Create a test domain
    let domain = Domain::create(
        "https://example.org".to_string(),
        None,
        "test".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create domain");

    // Configure mock web scraper with different content for each scrape
    let mock_scraper = MockWebScraper::new()
        .with_response("# First Version\nOriginal content")
        .with_response("# Second Version\nUpdated content");

    let test_deps = TestDependencies::new()
        .mock_scraper(mock_scraper);

    let test_harness = TestHarness::with_deps(test_deps)
        .await
        .expect("Failed to create test harness with custom deps");

    // Execute: Trigger first scrape
    test_harness
        .bus()
        .send(ListingEvent::ScrapeSourceRequested {
            source_id: domain.id,
            job_id: Uuid::new_v4(),
            requested_by: Uuid::new_v4(),
            is_admin: true,
        })
        .await
        .expect("Failed to send first scrape event");

    test_harness.settle().await;

    // Execute: Trigger second scrape with different content
    test_harness
        .bus()
        .send(ListingEvent::ScrapeSourceRequested {
            source_id: domain.id,
            job_id: Uuid::new_v4(),
            requested_by: Uuid::new_v4(),
            is_admin: true,
        })
        .await
        .expect("Failed to send second scrape event");

    test_harness.settle().await;

    // Verify: Should have TWO page_snapshots (different content_hash)
    let page_snapshot_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM page_snapshots WHERE url = $1",
    )
    .bind("https://example.org")
    .fetch_one(&test_harness.db_pool)
    .await
    .expect("Failed to query page_snapshots");

    assert_eq!(
        page_snapshot_count, 2,
        "Expected two page_snapshots when content changes"
    );
}

#[test_context(TestHarness)]
#[tokio::test]
async fn test_scrape_failure_does_not_create_page_snapshot(ctx: &TestHarness) {
    // Setup: Create a test domain
    let domain = Domain::create(
        "https://example.org".to_string(),
        None,
        "test".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create domain");

    // Configure mock web scraper with no responses (will cause error)
    let mock_scraper = MockWebScraper::new(); // No responses configured

    let test_deps = TestDependencies::new()
        .mock_scraper(mock_scraper);

    let test_harness = TestHarness::with_deps(test_deps)
        .await
        .expect("Failed to create test harness with custom deps");

    // Execute: Trigger scrape that will fail
    test_harness
        .bus()
        .send(ListingEvent::ScrapeSourceRequested {
            source_id: domain.id,
            job_id: Uuid::new_v4(),
            requested_by: Uuid::new_v4(),
            is_admin: true,
        })
        .await
        .expect("Failed to send scrape event");

    test_harness.settle().await;

    // Verify: No page_snapshot should be created on failure
    let page_snapshot_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM page_snapshots WHERE url = $1",
    )
    .bind("https://example.org")
    .fetch_one(&test_harness.db_pool)
    .await
    .expect("Failed to query page_snapshots");

    assert_eq!(
        page_snapshot_count, 0,
        "Expected no page_snapshot when scraping fails"
    );
}

#[test_context(TestHarness)]
#[tokio::test]
async fn test_domain_snapshot_links_to_page_snapshot(ctx: &TestHarness) {
    // Setup: Create a test domain
    let domain = Domain::create(
        "https://example.org".to_string(),
        None,
        "test".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create domain");

    // Create a domain_snapshot entry (simulating user submitting a specific page)
    let domain_snapshot_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO domain_snapshots (domain_id, page_url, scrape_status)
        VALUES ($1, $2, 'pending')
        RETURNING id
        "#,
    )
    .bind(domain.id.into_uuid())
    .bind("https://example.org")
    .fetch_one(&ctx.db_pool)
    .await
    .expect("Failed to create domain_snapshot");

    // Configure mock web scraper
    let mock_scraper = MockWebScraper::new().with_response("# Test Page\nTest content");

    let test_deps = TestDependencies::new()
        .mock_scraper(mock_scraper);

    let test_harness = TestHarness::with_deps(test_deps)
        .await
        .expect("Failed to create test harness with custom deps");

    // Execute: Trigger scrape
    test_harness
        .bus()
        .send(ListingEvent::ScrapeSourceRequested {
            source_id: domain.id,
            job_id: Uuid::new_v4(),
            requested_by: Uuid::new_v4(),
            is_admin: true,
        })
        .await
        .expect("Failed to send scrape event");

    test_harness.settle().await;

    // Verify: domain_snapshot should be linked to page_snapshot
    let domain_snapshot = sqlx::query!(
        r#"
        SELECT page_snapshot_id, scrape_status
        FROM domain_snapshots
        WHERE id = $1
        "#,
        domain_snapshot_id
    )
    .fetch_one(&test_harness.db_pool)
    .await
    .expect("Failed to fetch domain_snapshot");

    assert!(
        domain_snapshot.page_snapshot_id.is_some(),
        "Expected domain_snapshot to be linked to page_snapshot"
    );
    assert_eq!(
        domain_snapshot.scrape_status, "scraped",
        "Expected scrape_status to be 'scraped'"
    );
}
