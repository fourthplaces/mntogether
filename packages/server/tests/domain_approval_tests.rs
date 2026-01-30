//! Integration tests for domain approval workflow via GraphQL.
//!
//! Tests the domain management mutations:
//! - approveDomain: Approve a pending domain
//! - rejectDomain: Reject a pending domain with reason
//! - suspendDomain: Suspend an approved domain with reason

mod common;

use crate::common::{GraphQLClient, TestHarness};
use juniper::Variables;
use server_core::common::{DomainId, MemberId};
use server_core::domains::scraping::models::Domain;
use test_context::test_context;
use uuid::Uuid;

// Helper to create admin user for testing
async fn create_admin_user(ctx: &TestHarness) -> Uuid {
    let admin_id = Uuid::new_v4();
    // Insert member into database so foreign key constraint is satisfied
    // Use unique expo_push_token for each test
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
// Approve Domain Tests
// =============================================================================

/// RED: This test will FAIL because approveDomain mutation doesn't exist yet
#[test_context(TestHarness)]
#[tokio::test]
async fn approve_domain_changes_status_to_approved(ctx: &TestHarness) {
    // Arrange: Create pending domain
    let domain = Domain::create(
        "https://test-approve.org".to_string(),
        None,
        "public_user".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create domain");

    // Verify it starts as pending_review
    assert_eq!(domain.status, "pending_review");

    // Create authenticated admin client
    let admin_id = create_admin_user(ctx).await;
    let client = GraphQLClient::with_auth_user(ctx.kernel.clone(), admin_id, true);

    // Act: Approve domain via GraphQL mutation
    let mutation = r#"
        mutation ApproveDomain($domainId: String!) {
            approveDomain(domainId: $domainId) {
                id
                status
                submittedBy
            }
        }
    "#;

    let mut vars = Variables::new();
    vars.insert(
        "domainId".to_string(),
        juniper::InputValue::scalar(domain.id.to_string()),
    );

    let result = client.query_with_vars(mutation, vars).await;

    // Assert: GraphQL response shows approved status
    assert_eq!(
        result["approveDomain"]["status"].as_str().unwrap(),
        "approved"
    );
    assert_eq!(
        result["approveDomain"]["id"].as_str().unwrap(),
        domain.id.to_string()
    );

    // Assert: Database state is updated using model
    let updated_domain = Domain::find_by_id(domain.id, &ctx.db_pool)
        .await
        .expect("Failed to find updated domain");

    assert_eq!(updated_domain.status, "approved");
    assert!(updated_domain.reviewed_by.is_some());
    assert!(updated_domain.reviewed_at.is_some());
    assert_eq!(
        updated_domain.reviewed_by.unwrap().into_uuid(),
        admin_id
    );
}

/// Test that approving a domain requires admin authentication
#[test_context(TestHarness)]
#[tokio::test]
async fn approve_domain_requires_admin_auth(ctx: &TestHarness) {
    // Arrange: Create pending domain
    let domain = Domain::create(
        "https://test-approve-auth.org".to_string(),
        None,
        "public_user".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create domain");

    // Try to approve with unauthenticated client
    let client = ctx.graphql();

    let mutation = r#"
        mutation ApproveDomain($domainId: String!) {
            approveDomain(domainId: $domainId) {
                id
                status
            }
        }
    "#;

    let mut vars = Variables::new();
    vars.insert(
        "domainId".to_string(),
        juniper::InputValue::scalar(domain.id.to_string()),
    );

    let result = client.execute_with_vars(mutation, vars).await;

    // Assert: Should return an error for unauthenticated request
    assert!(!result.is_ok(), "Expected error for unauthenticated request, got success");
    assert!(!result.errors.is_empty(), "Expected errors in response");
}

/// Test that approving a nonexistent domain returns an error
#[test_context(TestHarness)]
#[tokio::test]
async fn approve_nonexistent_domain_returns_error(ctx: &TestHarness) {
    // Create authenticated admin client
    let admin_id = create_admin_user(ctx).await;
    let client = GraphQLClient::with_auth_user(ctx.kernel.clone(), admin_id, true);

    let mutation = r#"
        mutation ApproveDomain($domainId: String!) {
            approveDomain(domainId: $domainId) {
                id
                status
            }
        }
    "#;

    let fake_id = Uuid::new_v4();
    let mut vars = Variables::new();
    vars.insert(
        "domainId".to_string(),
        juniper::InputValue::scalar(fake_id.to_string()),
    );

    let result = client.execute_with_vars(mutation, vars).await;

    // Assert: Should return an error for nonexistent domain
    assert!(!result.is_ok(), "Expected error for nonexistent domain, got success");
    assert!(!result.errors.is_empty(), "Expected errors in response");
}

// =============================================================================
// Reject Domain Tests
// =============================================================================

/// RED: This test will FAIL because rejectDomain mutation doesn't exist yet
#[test_context(TestHarness)]
#[tokio::test]
async fn reject_domain_changes_status_to_rejected(ctx: &TestHarness) {
    // Arrange: Create pending domain
    let domain = Domain::create(
        "https://test-reject.org".to_string(),
        None,
        "public_user".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create domain");

    // Create authenticated admin client
    let admin_id = create_admin_user(ctx).await;
    let client = GraphQLClient::with_auth_user(ctx.kernel.clone(), admin_id, true);

    // Act: Reject domain via GraphQL mutation
    let mutation = r#"
        mutation RejectDomain($domainId: String!, $reason: String!) {
            rejectDomain(domainId: $domainId, reason: $reason) {
                id
                status
            }
        }
    "#;

    let mut vars = Variables::new();
    vars.insert(
        "domainId".to_string(),
        juniper::InputValue::scalar(domain.id.to_string()),
    );
    vars.insert(
        "reason".to_string(),
        juniper::InputValue::scalar("Domain does not contain volunteer opportunities".to_string()),
    );

    let result = client.query_with_vars(mutation, vars).await;

    // Assert: GraphQL response shows rejected status
    assert_eq!(
        result["rejectDomain"]["status"].as_str().unwrap(),
        "rejected"
    );

    // Assert: Database state is updated using model
    let updated_domain = Domain::find_by_id(domain.id, &ctx.db_pool)
        .await
        .expect("Failed to find updated domain");

    assert_eq!(updated_domain.status, "rejected");
    assert!(updated_domain.reviewed_by.is_some());
    assert!(updated_domain.reviewed_at.is_some());
    assert_eq!(
        updated_domain.rejection_reason.as_deref().unwrap(),
        "Domain does not contain volunteer opportunities"
    );
}

/// Test that rejecting a domain requires admin authentication
#[test_context(TestHarness)]
#[tokio::test]
async fn reject_domain_requires_admin_auth(ctx: &TestHarness) {
    // Arrange: Create pending domain
    let domain = Domain::create(
        "https://test-reject-auth.org".to_string(),
        None,
        "public_user".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create domain");

    // Try to reject with unauthenticated client
    let client = ctx.graphql();

    let mutation = r#"
        mutation RejectDomain($domainId: String!, $reason: String!) {
            rejectDomain(domainId: $domainId, reason: $reason) {
                id
                status
            }
        }
    "#;

    let mut vars = Variables::new();
    vars.insert(
        "domainId".to_string(),
        juniper::InputValue::scalar(domain.id.to_string()),
    );
    vars.insert(
        "reason".to_string(),
        juniper::InputValue::scalar("test reason".to_string()),
    );

    let result = client.execute_with_vars(mutation, vars).await;

    // Assert: Should return an error for unauthenticated request
    assert!(!result.is_ok(), "Expected error for unauthenticated request, got success");
    assert!(!result.errors.is_empty(), "Expected errors in response");
}

// =============================================================================
// Suspend Domain Tests
// =============================================================================

/// RED: This test will FAIL because suspendDomain mutation doesn't exist yet
#[test_context(TestHarness)]
#[tokio::test]
async fn suspend_domain_changes_status_to_suspended(ctx: &TestHarness) {
    // Arrange: Create and approve a domain first
    let domain = Domain::create(
        "https://test-suspend.org".to_string(),
        None,
        "public_user".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create domain");

    // Create admin user
    let admin_id = create_admin_user(ctx).await;

    // Approve the domain using model method
    Domain::approve(domain.id, MemberId::from_uuid(admin_id), &ctx.db_pool)
        .await
        .expect("Failed to approve domain");

    // Get authenticated admin client
    let client = GraphQLClient::with_auth_user(ctx.kernel.clone(), admin_id, true);

    // Act: Suspend domain via GraphQL mutation
    let mutation = r#"
        mutation SuspendDomain($domainId: String!, $reason: String!) {
            suspendDomain(domainId: $domainId, reason: $reason) {
                id
                status
            }
        }
    "#;

    let mut vars = Variables::new();
    vars.insert(
        "domainId".to_string(),
        juniper::InputValue::scalar(domain.id.to_string()),
    );
    vars.insert(
        "reason".to_string(),
        juniper::InputValue::scalar("Domain violated terms of service".to_string()),
    );

    let result = client.query_with_vars(mutation, vars).await;

    // Assert: GraphQL response shows suspended status
    assert_eq!(
        result["suspendDomain"]["status"].as_str().unwrap(),
        "suspended"
    );

    // Assert: Database state is updated using model
    let updated_domain = Domain::find_by_id(domain.id, &ctx.db_pool)
        .await
        .expect("Failed to find updated domain");

    assert_eq!(updated_domain.status, "suspended");
    assert!(updated_domain.reviewed_by.is_some());
    assert!(updated_domain.reviewed_at.is_some());
    assert_eq!(
        updated_domain.rejection_reason.as_deref().unwrap(),
        "Domain violated terms of service"
    );
}

/// Test that suspending a domain requires admin authentication
#[test_context(TestHarness)]
#[tokio::test]
async fn suspend_domain_requires_admin_auth(ctx: &TestHarness) {
    // Arrange: Create pending domain
    let domain = Domain::create(
        "https://test-suspend-auth.org".to_string(),
        None,
        "public_user".to_string(),
        Some("test@example.com".to_string()),
        3,
        &ctx.db_pool,
    )
    .await
    .expect("Failed to create domain");

    // Try to suspend with unauthenticated client
    let client = ctx.graphql();

    let mutation = r#"
        mutation SuspendDomain($domainId: String!, $reason: String!) {
            suspendDomain(domainId: $domainId, reason: $reason) {
                id
                status
            }
        }
    "#;

    let mut vars = Variables::new();
    vars.insert(
        "domainId".to_string(),
        juniper::InputValue::scalar(domain.id.to_string()),
    );
    vars.insert(
        "reason".to_string(),
        juniper::InputValue::scalar("test reason".to_string()),
    );

    let result = client.execute_with_vars(mutation, vars).await;

    // Assert: Should return an error for unauthenticated request
    assert!(!result.is_ok(), "Expected error for unauthenticated request, got success");
    assert!(!result.errors.is_empty(), "Expected errors in response");
}
