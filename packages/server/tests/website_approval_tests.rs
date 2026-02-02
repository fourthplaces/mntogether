//! Integration tests for website approval workflow via GraphQL.
//!
//! Tests the website management mutations:
//! - approveWebsite: Approve a pending website
//! - rejectWebsite: Reject a pending website with reason
//! - suspendWebsite: Suspend an approved website with reason

mod common;

use crate::common::{GraphQLClient, TestHarness};
use juniper::Variables;
use server_core::common::{MemberId, WebsiteId};
use server_core::domains::website::models::Website;
use test_context::test_context;
use uuid::Uuid;

// Helper to create admin user for testing
async fn create_admin_user(ctx: &TestHarness) -> Uuid {
    let admin_id = Uuid::new_v4();
    // Insert member into database so foreign key constraint is satisfied
    // Use unique expo_push_token for each test
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
// Approve Website Tests
// =============================================================================

/// RED: This test will FAIL because approveWebsite mutation doesn't exist yet
#[test_context(TestHarness)]
#[tokio::test]
async fn approve_website_changes_status_to_approved(ctx: &TestHarness) {
    // Arrange: Create pending website
    let website = Website::create(
        "https://test-approve.org".to_string(),
        None,
        "public_user".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create website");

    // Verify it starts as pending_review
    assert_eq!(website.status, "pending_review");

    // Create authenticated admin client
    let admin_id = create_admin_user(ctx).await;
    let client = GraphQLClient::with_auth_user(ctx.kernel.clone(), admin_id, true);

    // Act: Approve website via GraphQL mutation
    let mutation = r#"
        mutation ApproveWebsite($websiteId: String!) {
            approveWebsite(websiteId: $websiteId) {
                id
                status
                submittedBy
            }
        }
    "#;

    let mut vars = Variables::new();
    vars.insert(
        "websiteId".to_string(),
        juniper::InputValue::scalar(website.id.to_string()),
    );

    let result = client.query_with_vars(mutation, vars).await;

    // Assert: GraphQL response shows approved status
    assert_eq!(
        result["approveWebsite"]["status"].as_str().unwrap(),
        "approved"
    );
    assert_eq!(
        result["approveWebsite"]["id"].as_str().unwrap(),
        website.id.to_string()
    );

    // Assert: Database state is updated using model
    let updated_website = Website::find_by_id(website.id, &ctx.db_pool)
        .await
        .expect("Failed to find updated website");

    assert_eq!(updated_website.status, "approved");
    assert!(updated_website.reviewed_by.is_some());
    assert!(updated_website.reviewed_at.is_some());
    assert_eq!(updated_website.reviewed_by.unwrap().into_uuid(), admin_id);
}

/// Test that approving a website requires admin authentication
#[test_context(TestHarness)]
#[tokio::test]
async fn approve_website_requires_admin_auth(ctx: &TestHarness) {
    // Arrange: Create pending website
    let website = Website::create(
        "https://test-approve-auth.org".to_string(),
        None,
        "public_user".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create website");

    // Try to approve with unauthenticated client
    let client = ctx.graphql();

    let mutation = r#"
        mutation ApproveWebsite($websiteId: String!) {
            approveWebsite(websiteId: $websiteId) {
                id
                status
            }
        }
    "#;

    let mut vars = Variables::new();
    vars.insert(
        "websiteId".to_string(),
        juniper::InputValue::scalar(website.id.to_string()),
    );

    let result = client.execute_with_vars(mutation, vars).await;

    // Assert: Should return an error for unauthenticated request
    assert!(
        !result.is_ok(),
        "Expected error for unauthenticated request, got success"
    );
    assert!(!result.errors.is_empty(), "Expected errors in response");
}

/// Test that approving a nonexistent website returns an error
#[test_context(TestHarness)]
#[tokio::test]
async fn approve_nonexistent_website_returns_error(ctx: &TestHarness) {
    // Create authenticated admin client
    let admin_id = create_admin_user(ctx).await;
    let client = GraphQLClient::with_auth_user(ctx.kernel.clone(), admin_id, true);

    let mutation = r#"
        mutation ApproveWebsite($websiteId: String!) {
            approveWebsite(websiteId: $websiteId) {
                id
                status
            }
        }
    "#;

    let fake_id = Uuid::new_v4();
    let mut vars = Variables::new();
    vars.insert(
        "websiteId".to_string(),
        juniper::InputValue::scalar(fake_id.to_string()),
    );

    let result = client.execute_with_vars(mutation, vars).await;

    // Assert: Should return an error for nonexistent website
    assert!(
        !result.is_ok(),
        "Expected error for nonexistent website, got success"
    );
    assert!(!result.errors.is_empty(), "Expected errors in response");
}

// =============================================================================
// Reject Website Tests
// =============================================================================

/// RED: This test will FAIL because rejectWebsite mutation doesn't exist yet
#[test_context(TestHarness)]
#[tokio::test]
async fn reject_website_changes_status_to_rejected(ctx: &TestHarness) {
    // Arrange: Create pending website
    let website = Website::create(
        "https://test-reject.org".to_string(),
        None,
        "public_user".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create website");

    // Create authenticated admin client
    let admin_id = create_admin_user(ctx).await;
    let client = GraphQLClient::with_auth_user(ctx.kernel.clone(), admin_id, true);

    // Act: Reject website via GraphQL mutation
    let mutation = r#"
        mutation RejectWebsite($websiteId: String!, $reason: String!) {
            rejectWebsite(websiteId: $websiteId, reason: $reason) {
                id
                status
            }
        }
    "#;

    let mut vars = Variables::new();
    vars.insert(
        "websiteId".to_string(),
        juniper::InputValue::scalar(website.id.to_string()),
    );
    vars.insert(
        "reason".to_string(),
        juniper::InputValue::scalar("Website does not contain volunteer opportunities".to_string()),
    );

    let result = client.query_with_vars(mutation, vars).await;

    // Assert: GraphQL response shows rejected status
    assert_eq!(
        result["rejectWebsite"]["status"].as_str().unwrap(),
        "rejected"
    );

    // Assert: Database state is updated using model
    let updated_website = Website::find_by_id(website.id, &ctx.db_pool)
        .await
        .expect("Failed to find updated website");

    assert_eq!(updated_website.status, "rejected");
    assert!(updated_website.reviewed_by.is_some());
    assert!(updated_website.reviewed_at.is_some());
    assert_eq!(
        updated_website.rejection_reason.as_deref().unwrap(),
        "Website does not contain volunteer opportunities"
    );
}

/// Test that rejecting a website requires admin authentication
#[test_context(TestHarness)]
#[tokio::test]
async fn reject_website_requires_admin_auth(ctx: &TestHarness) {
    // Arrange: Create pending website
    let website = Website::create(
        "https://test-reject-auth.org".to_string(),
        None,
        "public_user".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create website");

    // Try to reject with unauthenticated client
    let client = ctx.graphql();

    let mutation = r#"
        mutation RejectWebsite($websiteId: String!, $reason: String!) {
            rejectWebsite(websiteId: $websiteId, reason: $reason) {
                id
                status
            }
        }
    "#;

    let mut vars = Variables::new();
    vars.insert(
        "websiteId".to_string(),
        juniper::InputValue::scalar(website.id.to_string()),
    );
    vars.insert(
        "reason".to_string(),
        juniper::InputValue::scalar("test reason".to_string()),
    );

    let result = client.execute_with_vars(mutation, vars).await;

    // Assert: Should return an error for unauthenticated request
    assert!(
        !result.is_ok(),
        "Expected error for unauthenticated request, got success"
    );
    assert!(!result.errors.is_empty(), "Expected errors in response");
}

// =============================================================================
// Suspend Website Tests
// =============================================================================

/// RED: This test will FAIL because suspendWebsite mutation doesn't exist yet
#[test_context(TestHarness)]
#[tokio::test]
async fn suspend_website_changes_status_to_suspended(ctx: &TestHarness) {
    // Arrange: Create and approve a website first
    let website = Website::create(
        "https://test-suspend.org".to_string(),
        None,
        "public_user".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create website");

    // Create admin user
    let admin_id = create_admin_user(ctx).await;

    // Approve the website using model method
    Website::approve(website.id, MemberId::from_uuid(admin_id), &ctx.db_pool)
        .await
        .expect("Failed to approve website");

    // Get authenticated admin client
    let client = GraphQLClient::with_auth_user(ctx.kernel.clone(), admin_id, true);

    // Act: Suspend website via GraphQL mutation
    let mutation = r#"
        mutation SuspendWebsite($websiteId: String!, $reason: String!) {
            suspendWebsite(websiteId: $websiteId, reason: $reason) {
                id
                status
            }
        }
    "#;

    let mut vars = Variables::new();
    vars.insert(
        "websiteId".to_string(),
        juniper::InputValue::scalar(website.id.to_string()),
    );
    vars.insert(
        "reason".to_string(),
        juniper::InputValue::scalar("Website violated terms of service".to_string()),
    );

    let result = client.query_with_vars(mutation, vars).await;

    // Assert: GraphQL response shows suspended status
    assert_eq!(
        result["suspendWebsite"]["status"].as_str().unwrap(),
        "suspended"
    );

    // Assert: Database state is updated using model
    let updated_website = Website::find_by_id(website.id, &ctx.db_pool)
        .await
        .expect("Failed to find updated website");

    assert_eq!(updated_website.status, "suspended");
    assert!(updated_website.reviewed_by.is_some());
    assert!(updated_website.reviewed_at.is_some());
    assert_eq!(
        updated_website.rejection_reason.as_deref().unwrap(),
        "Website violated terms of service"
    );
}

/// Test that suspending a website requires admin authentication
#[test_context(TestHarness)]
#[tokio::test]
async fn suspend_website_requires_admin_auth(ctx: &TestHarness) {
    // Arrange: Create pending website
    let website = Website::create(
        "https://test-suspend-auth.org".to_string(),
        None,
        "public_user".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create website");

    // Try to suspend with unauthenticated client
    let client = ctx.graphql();

    let mutation = r#"
        mutation SuspendWebsite($websiteId: String!, $reason: String!) {
            suspendWebsite(websiteId: $websiteId, reason: $reason) {
                id
                status
            }
        }
    "#;

    let mut vars = Variables::new();
    vars.insert(
        "websiteId".to_string(),
        juniper::InputValue::scalar(website.id.to_string()),
    );
    vars.insert(
        "reason".to_string(),
        juniper::InputValue::scalar("test reason".to_string()),
    );

    let result = client.execute_with_vars(mutation, vars).await;

    // Assert: Should return an error for unauthenticated request
    assert!(
        !result.is_ok(),
        "Expected error for unauthenticated request, got success"
    );
    assert!(!result.errors.is_empty(), "Expected errors in response");
}
