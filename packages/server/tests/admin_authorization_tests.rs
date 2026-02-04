//! Admin authorization security tests
//!
//! TDD: Tests are written FIRST to verify authorization gaps exist,
//! then implementations are added to make them pass.
//!
//! Each admin-only endpoint gets 3 tests:
//! 1. `*_as_admin_succeeds` - Admin can perform action
//! 2. `*_as_non_admin_fails` - Authenticated non-admin gets "Admin access required" error
//! 3. `*_unauthenticated_fails` - No auth gets "Valid JWT required" error

mod common;

use crate::common::{GraphQLClient, TestHarness};
use server_core::common::{MemberId, ProviderId};
use server_core::domains::providers::models::{CreateProvider, Provider, ProviderStatus};
use uuid::Uuid;

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a test harness with admin and non-admin users
async fn setup_test_harness() -> (TestHarness, Uuid, Uuid) {
    let harness = TestHarness::new()
        .await
        .expect("Failed to create test harness");

    // Create admin member (just need the ID - auth comes from GraphQL context, not database)
    let admin_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO members (id, expo_push_token, searchable_text, active, notification_count_this_week, created_at)
         VALUES ($1, $2, 'Admin user for testing', true, 0, NOW())"
    )
    .bind(admin_id)
    .bind(format!("ExponentPushToken[admin-{}]", admin_id))
    .execute(&harness.db_pool)
    .await
    .expect("Failed to create admin member");

    // Create non-admin member
    let non_admin_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO members (id, expo_push_token, searchable_text, active, notification_count_this_week, created_at)
         VALUES ($1, $2, 'Non-admin user for testing', true, 0, NOW())"
    )
    .bind(non_admin_id)
    .bind(format!("ExponentPushToken[user-{}]", non_admin_id))
    .execute(&harness.db_pool)
    .await
    .expect("Failed to create non-admin member");

    (harness, admin_id, non_admin_id)
}

/// Create a test website (needed for resources)
async fn create_test_website(harness: &TestHarness) -> Uuid {
    let website_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO websites (id, domain, status, created_at, updated_at)
           VALUES ($1, $2, 'approved', NOW(), NOW())"#,
    )
    .bind(website_id)
    .bind(format!("test-{}.example.com", website_id))
    .execute(&harness.db_pool)
    .await
    .expect("Failed to create test website");
    website_id
}

/// Create a pending provider for testing
async fn create_test_provider(harness: &TestHarness) -> Uuid {
    let provider_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO providers (id, name, status, created_at, updated_at)
           VALUES ($1, 'Test Provider', 'pending_review', NOW(), NOW())"#,
    )
    .bind(provider_id)
    .execute(&harness.db_pool)
    .await
    .expect("Failed to create test provider");
    provider_id
}

/// Assert error contains admin required message
fn assert_admin_required(errors: &[String]) {
    assert!(!errors.is_empty(), "Expected admin error but got no errors");
    let msg = &errors[0];
    assert!(
        msg.contains("Admin") || msg.contains("admin") || msg.contains("Unauthorized"),
        "Expected admin required error, got: {}",
        msg
    );
}

/// Assert error contains authentication or admin required message
/// For unauthenticated users, we may get either:
/// - "Unauthenticated: Valid JWT required" (if auth checked first)
/// - "Unauthorized: Admin access required" (if admin checked first, which returns false for no user)
fn assert_auth_required(errors: &[String]) {
    assert!(!errors.is_empty(), "Expected auth error but got no errors");
    let msg = &errors[0];
    assert!(
        msg.contains("Unauthenticated")
            || msg.contains("JWT")
            || msg.contains("Authentication")
            || msg.contains("Admin")
            || msg.contains("Unauthorized"),
        "Expected authentication/authorization required error, got: {}",
        msg
    );
}

// ============================================================================
// Provider Mutation Authorization Tests
// ============================================================================

// --- approve_provider ---

#[tokio::test]
async fn approve_provider_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client.execute(&format!(
        r#"mutation {{ approveProvider(providerId: "{}", reviewedById: "{}") {{ id status }} }}"#,
        provider_id, admin_id
    )).await;

    assert!(
        result.is_ok(),
        "Admin should be able to approve provider. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn approve_provider_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client.execute(&format!(
        r#"mutation {{ approveProvider(providerId: "{}", reviewedById: "{}") {{ id status }} }}"#,
        provider_id, non_admin_id
    )).await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to approve provider"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn approve_provider_unauthenticated_fails() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    let client = harness.graphql();
    let result = client.execute(&format!(
        r#"mutation {{ approveProvider(providerId: "{}", reviewedById: "{}") {{ id status }} }}"#,
        provider_id, admin_id
    )).await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to approve provider"
    );
    assert_auth_required(&result.errors);
}

// --- reject_provider ---

#[tokio::test]
async fn reject_provider_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client.execute(&format!(
        r#"mutation {{ rejectProvider(providerId: "{}", reason: "Test reason", reviewedById: "{}") {{ id status }} }}"#,
        provider_id, admin_id
    )).await;

    assert!(
        result.is_ok(),
        "Admin should be able to reject provider. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn reject_provider_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client.execute(&format!(
        r#"mutation {{ rejectProvider(providerId: "{}", reason: "Test reason", reviewedById: "{}") {{ id status }} }}"#,
        provider_id, non_admin_id
    )).await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to reject provider"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn reject_provider_unauthenticated_fails() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    let client = harness.graphql();
    let result = client.execute(&format!(
        r#"mutation {{ rejectProvider(providerId: "{}", reason: "Test reason", reviewedById: "{}") {{ id status }} }}"#,
        provider_id, admin_id
    )).await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to reject provider"
    );
    assert_auth_required(&result.errors);
}

// --- update_provider ---

#[tokio::test]
async fn update_provider_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client.execute(&format!(
        r#"mutation {{ updateProvider(providerId: "{}", input: {{ name: "Updated Name" }}) {{ id name }} }}"#,
        provider_id
    )).await;

    assert!(
        result.is_ok(),
        "Admin should be able to update provider. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn update_provider_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client.execute(&format!(
        r#"mutation {{ updateProvider(providerId: "{}", input: {{ name: "Updated Name" }}) {{ id name }} }}"#,
        provider_id
    )).await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to update provider"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn update_provider_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    let client = harness.graphql();
    let result = client.execute(&format!(
        r#"mutation {{ updateProvider(providerId: "{}", input: {{ name: "Updated Name" }}) {{ id name }} }}"#,
        provider_id
    )).await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to update provider"
    );
    assert_auth_required(&result.errors);
}

// --- delete_provider ---

#[tokio::test]
async fn delete_provider_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client
        .execute(&format!(
            r#"mutation {{ deleteProvider(providerId: "{}") }}"#,
            provider_id
        ))
        .await;

    assert!(
        result.is_ok(),
        "Admin should be able to delete provider. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn delete_provider_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client
        .execute(&format!(
            r#"mutation {{ deleteProvider(providerId: "{}") }}"#,
            provider_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to delete provider"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn delete_provider_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    let client = harness.graphql();
    let result = client
        .execute(&format!(
            r#"mutation {{ deleteProvider(providerId: "{}") }}"#,
            provider_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to delete provider"
    );
    assert_auth_required(&result.errors);
}

// --- add_provider_tag ---

#[tokio::test]
async fn add_provider_tag_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client.execute(&format!(
        r#"mutation {{ addProviderTag(providerId: "{}", tagKind: "specialty", tagValue: "therapy") {{ id }} }}"#,
        provider_id
    )).await;

    assert!(
        result.is_ok(),
        "Admin should be able to add provider tag. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn add_provider_tag_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client.execute(&format!(
        r#"mutation {{ addProviderTag(providerId: "{}", tagKind: "specialty", tagValue: "therapy") {{ id }} }}"#,
        provider_id
    )).await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to add provider tag"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn add_provider_tag_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    let client = harness.graphql();
    let result = client.execute(&format!(
        r#"mutation {{ addProviderTag(providerId: "{}", tagKind: "specialty", tagValue: "therapy") {{ id }} }}"#,
        provider_id
    )).await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to add provider tag"
    );
    assert_auth_required(&result.errors);
}

// --- remove_provider_tag ---

#[tokio::test]
async fn remove_provider_tag_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    // First add a tag
    let tag_id = Uuid::new_v4();
    sqlx::query("INSERT INTO tags (id, kind, value, created_at) VALUES ($1, 'test', 'tag', NOW())")
        .bind(tag_id)
        .execute(&harness.db_pool)
        .await
        .expect("Failed to create tag");
    sqlx::query(
        "INSERT INTO taggables (tag_id, taggable_type, taggable_id) VALUES ($1, 'provider', $2)",
    )
    .bind(tag_id)
    .bind(provider_id)
    .execute(&harness.db_pool)
    .await
    .expect("Failed to add tag to provider");

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client
        .execute(&format!(
            r#"mutation {{ removeProviderTag(providerId: "{}", tagId: "{}") }}"#,
            provider_id, tag_id
        ))
        .await;

    assert!(
        result.is_ok(),
        "Admin should be able to remove provider tag. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn remove_provider_tag_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;
    let tag_id = Uuid::new_v4();

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client
        .execute(&format!(
            r#"mutation {{ removeProviderTag(providerId: "{}", tagId: "{}") }}"#,
            provider_id, tag_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to remove provider tag"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn remove_provider_tag_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;
    let tag_id = Uuid::new_v4();

    let client = harness.graphql();
    let result = client
        .execute(&format!(
            r#"mutation {{ removeProviderTag(providerId: "{}", tagId: "{}") }}"#,
            provider_id, tag_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to remove provider tag"
    );
    assert_auth_required(&result.errors);
}

// --- add_provider_contact ---

#[tokio::test]
async fn add_provider_contact_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client.execute(&format!(
        r#"mutation {{ addProviderContact(providerId: "{}", contactType: "email", contactValue: "test@example.com") {{ id }} }}"#,
        provider_id
    )).await;

    assert!(
        result.is_ok(),
        "Admin should be able to add provider contact. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn add_provider_contact_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client.execute(&format!(
        r#"mutation {{ addProviderContact(providerId: "{}", contactType: "email", contactValue: "test@example.com") {{ id }} }}"#,
        provider_id
    )).await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to add provider contact"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn add_provider_contact_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    let client = harness.graphql();
    let result = client.execute(&format!(
        r#"mutation {{ addProviderContact(providerId: "{}", contactType: "email", contactValue: "test@example.com") {{ id }} }}"#,
        provider_id
    )).await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to add provider contact"
    );
    assert_auth_required(&result.errors);
}

// --- remove_provider_contact ---

#[tokio::test]
async fn remove_provider_contact_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    // First add a contact
    let contact_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO contacts (id, contactable_type, contactable_id, contact_type, contact_value, is_public, display_order, created_at)
         VALUES ($1, 'provider', $2, 'email', 'test@example.com', true, 0, NOW())"
    )
    .bind(contact_id)
    .bind(provider_id)
    .execute(&harness.db_pool)
    .await
    .expect("Failed to create contact");

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client
        .execute(&format!(
            r#"mutation {{ removeProviderContact(contactId: "{}") }}"#,
            contact_id
        ))
        .await;

    assert!(
        result.is_ok(),
        "Admin should be able to remove provider contact. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn remove_provider_contact_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;
    let contact_id = Uuid::new_v4();

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client
        .execute(&format!(
            r#"mutation {{ removeProviderContact(contactId: "{}") }}"#,
            contact_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to remove provider contact"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn remove_provider_contact_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;
    let contact_id = Uuid::new_v4();

    let client = harness.graphql();
    let result = client
        .execute(&format!(
            r#"mutation {{ removeProviderContact(contactId: "{}") }}"#,
            contact_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to remove provider contact"
    );
    assert_auth_required(&result.errors);
}

// ============================================================================
// Resource Mutation Authorization Tests
// ============================================================================

// --- approve_resource ---

#[tokio::test]
async fn approve_resource_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let resource_id = create_test_resource(&harness).await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client
        .execute(&format!(
            r#"mutation {{ approveResource(resourceId: "{}") {{ id status }} }}"#,
            resource_id
        ))
        .await;

    assert!(
        result.is_ok(),
        "Admin should be able to approve resource. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn approve_resource_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;
    let resource_id = create_test_resource(&harness).await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client
        .execute(&format!(
            r#"mutation {{ approveResource(resourceId: "{}") {{ id status }} }}"#,
            resource_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to approve resource"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn approve_resource_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;
    let resource_id = create_test_resource(&harness).await;

    let client = harness.graphql();
    let result = client
        .execute(&format!(
            r#"mutation {{ approveResource(resourceId: "{}") {{ id status }} }}"#,
            resource_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to approve resource"
    );
    assert_auth_required(&result.errors);
}

// --- reject_resource ---

#[tokio::test]
async fn reject_resource_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let resource_id = create_test_resource(&harness).await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client.execute(&format!(
        r#"mutation {{ rejectResource(resourceId: "{}", reason: "Test reason") {{ id status }} }}"#,
        resource_id
    )).await;

    assert!(
        result.is_ok(),
        "Admin should be able to reject resource. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn reject_resource_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;
    let resource_id = create_test_resource(&harness).await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client.execute(&format!(
        r#"mutation {{ rejectResource(resourceId: "{}", reason: "Test reason") {{ id status }} }}"#,
        resource_id
    )).await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to reject resource"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn reject_resource_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;
    let resource_id = create_test_resource(&harness).await;

    let client = harness.graphql();
    let result = client.execute(&format!(
        r#"mutation {{ rejectResource(resourceId: "{}", reason: "Test reason") {{ id status }} }}"#,
        resource_id
    )).await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to reject resource"
    );
    assert_auth_required(&result.errors);
}

// --- delete_resource ---

#[tokio::test]
async fn delete_resource_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let resource_id = create_test_resource(&harness).await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client
        .execute(&format!(
            r#"mutation {{ deleteResource(resourceId: "{}") }}"#,
            resource_id
        ))
        .await;

    assert!(
        result.is_ok(),
        "Admin should be able to delete resource. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn delete_resource_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;
    let resource_id = create_test_resource(&harness).await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client
        .execute(&format!(
            r#"mutation {{ deleteResource(resourceId: "{}") }}"#,
            resource_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to delete resource"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn delete_resource_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;
    let resource_id = create_test_resource(&harness).await;

    let client = harness.graphql();
    let result = client
        .execute(&format!(
            r#"mutation {{ deleteResource(resourceId: "{}") }}"#,
            resource_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to delete resource"
    );
    assert_auth_required(&result.errors);
}

// ============================================================================
// Query Authorization Tests
// ============================================================================

// --- pending_providers ---

#[tokio::test]
async fn pending_providers_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client
        .execute(r#"query { pendingProviders { id name } }"#)
        .await;

    assert!(
        result.is_ok(),
        "Admin should be able to view pending providers. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn pending_providers_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client
        .execute(r#"query { pendingProviders { id name } }"#)
        .await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to view pending providers"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn pending_providers_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;

    let client = harness.graphql();
    let result = client
        .execute(r#"query { pendingProviders { id name } }"#)
        .await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to view pending providers"
    );
    assert_auth_required(&result.errors);
}

// --- pending_websites ---

#[tokio::test]
async fn pending_websites_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client
        .execute(r#"query { pendingWebsites { id url } }"#)
        .await;

    assert!(
        result.is_ok(),
        "Admin should be able to view pending websites. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn pending_websites_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client
        .execute(r#"query { pendingWebsites { id url } }"#)
        .await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to view pending websites"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn pending_websites_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;

    let client = harness.graphql();
    let result = client
        .execute(r#"query { pendingWebsites { id url } }"#)
        .await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to view pending websites"
    );
    assert_auth_required(&result.errors);
}

// ============================================================================
// Member Status Authorization Tests
// ============================================================================

/// Create a test member that can be updated
async fn create_test_target_member(harness: &TestHarness) -> Uuid {
    let member_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO members (id, expo_push_token, searchable_text, active, notification_count_this_week, created_at)
         VALUES ($1, $2, 'Target member for testing', true, 0, NOW())"
    )
    .bind(member_id)
    .bind(format!("ExponentPushToken[target-{}]", member_id))
    .execute(&harness.db_pool)
    .await
    .expect("Failed to create target member");
    member_id
}

#[tokio::test]
async fn update_member_status_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let target_member_id = create_test_target_member(&harness).await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client
        .execute(&format!(
            r#"mutation {{ updateMemberStatus(memberId: "{}", active: false) {{ id active }} }}"#,
            target_member_id
        ))
        .await;

    assert!(
        result.is_ok(),
        "Admin should be able to update member status. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn update_member_status_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;
    let target_member_id = create_test_target_member(&harness).await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client
        .execute(&format!(
            r#"mutation {{ updateMemberStatus(memberId: "{}", active: false) {{ id active }} }}"#,
            target_member_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to update member status"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn update_member_status_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;
    let target_member_id = create_test_target_member(&harness).await;

    let client = harness.graphql();
    let result = client
        .execute(&format!(
            r#"mutation {{ updateMemberStatus(memberId: "{}", active: false) {{ id active }} }}"#,
            target_member_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to update member status"
    );
    assert_auth_required(&result.errors);
}

// ============================================================================
// Post Tag Authorization Tests
// ============================================================================

/// Create a test post for tag operations
async fn create_test_post(harness: &TestHarness) -> Uuid {
    let post_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO posts (
            id, organization_name, title, description, post_type, category, status, source_language, created_at, updated_at
        ) VALUES (
            $1, 'Test Organization', 'Test Post', 'Test description', 'service', 'general', 'active', 'en', NOW(), NOW()
        )"#
    )
    .bind(post_id)
    .execute(&harness.db_pool)
    .await
    .expect("Failed to create test post");
    post_id
}

/// Create a test tag for removal tests (with unique value per call)
async fn create_test_tag(harness: &TestHarness) -> Uuid {
    let tag_id = Uuid::new_v4();
    let unique_value = format!("test-value-{}", tag_id);
    sqlx::query(
        r#"INSERT INTO tags (id, kind, value, display_name, created_at)
           VALUES ($1, 'test', $2, 'Test Tag', NOW())"#,
    )
    .bind(tag_id)
    .bind(&unique_value)
    .execute(&harness.db_pool)
    .await
    .expect("Failed to create test tag");
    tag_id
}

/// Link a tag to a post for removal tests
/// Note: taggable_type is 'listing' (posts table was renamed from listings)
async fn link_tag_to_post(harness: &TestHarness, post_id: Uuid, tag_id: Uuid) {
    sqlx::query(
        "INSERT INTO taggables (tag_id, taggable_type, taggable_id) VALUES ($1, 'listing', $2)",
    )
    .bind(tag_id)
    .bind(post_id)
    .execute(&harness.db_pool)
    .await
    .expect("Failed to link tag to post");
}

// --- update_post_tags ---

#[tokio::test]
async fn update_post_tags_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let post_id = create_test_post(&harness).await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client.execute(&format!(
        r#"mutation {{ updatePostTags(postId: "{}", tags: [{{ kind: "category", value: "test" }}]) {{ id }} }}"#,
        post_id
    )).await;

    assert!(
        result.is_ok(),
        "Admin should be able to update post tags. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn update_post_tags_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;
    let post_id = create_test_post(&harness).await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client.execute(&format!(
        r#"mutation {{ updatePostTags(postId: "{}", tags: [{{ kind: "category", value: "test" }}]) {{ id }} }}"#,
        post_id
    )).await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to update post tags"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn update_post_tags_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;
    let post_id = create_test_post(&harness).await;

    let client = harness.graphql();
    let result = client.execute(&format!(
        r#"mutation {{ updatePostTags(postId: "{}", tags: [{{ kind: "category", value: "test" }}]) {{ id }} }}"#,
        post_id
    )).await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to update post tags"
    );
    assert_auth_required(&result.errors);
}

// --- add_post_tag ---

#[tokio::test]
async fn add_post_tag_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let post_id = create_test_post(&harness).await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client.execute(&format!(
        r#"mutation {{ addPostTag(postId: "{}", tagKind: "category", tagValue: "new-tag") {{ id }} }}"#,
        post_id
    )).await;

    assert!(
        result.is_ok(),
        "Admin should be able to add post tag. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn add_post_tag_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;
    let post_id = create_test_post(&harness).await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client.execute(&format!(
        r#"mutation {{ addPostTag(postId: "{}", tagKind: "category", tagValue: "new-tag") {{ id }} }}"#,
        post_id
    )).await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to add post tag"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn add_post_tag_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;
    let post_id = create_test_post(&harness).await;

    let client = harness.graphql();
    let result = client.execute(&format!(
        r#"mutation {{ addPostTag(postId: "{}", tagKind: "category", tagValue: "new-tag") {{ id }} }}"#,
        post_id
    )).await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to add post tag"
    );
    assert_auth_required(&result.errors);
}

// --- remove_post_tag ---

#[tokio::test]
async fn remove_post_tag_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let post_id = create_test_post(&harness).await;
    let tag_id = create_test_tag(&harness).await;
    link_tag_to_post(&harness, post_id, tag_id).await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client
        .execute(&format!(
            r#"mutation {{ removePostTag(postId: "{}", tagId: "{}") }}"#,
            post_id, tag_id
        ))
        .await;

    assert!(
        result.is_ok(),
        "Admin should be able to remove post tag. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn remove_post_tag_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;
    let post_id = create_test_post(&harness).await;
    let tag_id = create_test_tag(&harness).await;
    link_tag_to_post(&harness, post_id, tag_id).await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client
        .execute(&format!(
            r#"mutation {{ removePostTag(postId: "{}", tagId: "{}") }}"#,
            post_id, tag_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to remove post tag"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn remove_post_tag_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;
    let post_id = create_test_post(&harness).await;
    let tag_id = create_test_tag(&harness).await;
    link_tag_to_post(&harness, post_id, tag_id).await;

    let client = harness.graphql();
    let result = client
        .execute(&format!(
            r#"mutation {{ removePostTag(postId: "{}", tagId: "{}") }}"#,
            post_id, tag_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to remove post tag"
    );
    assert_auth_required(&result.errors);
}

// ============================================================================
// Admin-Only Query Authorization Tests
// ============================================================================

// --- websites query ---

#[tokio::test]
async fn websites_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client
        .execute(r#"query { websites { nodes { id } } }"#)
        .await;

    assert!(
        result.is_ok(),
        "Admin should be able to query websites. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn websites_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client
        .execute(r#"query { websites { nodes { id } } }"#)
        .await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to query websites"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn websites_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;

    let client = harness.graphql();
    let result = client
        .execute(r#"query { websites { nodes { id } } }"#)
        .await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to query websites"
    );
    assert_auth_required(&result.errors);
}

// --- website query ---

#[tokio::test]
async fn website_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let website_id = create_test_website(&harness).await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client
        .execute(&format!(
            r#"query {{ website(id: "{}") {{ id }} }}"#,
            website_id
        ))
        .await;

    assert!(
        result.is_ok(),
        "Admin should be able to query website. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn website_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;
    let website_id = create_test_website(&harness).await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client
        .execute(&format!(
            r#"query {{ website(id: "{}") {{ id }} }}"#,
            website_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to query website"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn website_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;
    let website_id = create_test_website(&harness).await;

    let client = harness.graphql();
    let result = client
        .execute(&format!(
            r#"query {{ website(id: "{}") {{ id }} }}"#,
            website_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to query website"
    );
    assert_auth_required(&result.errors);
}

// --- website_assessment query ---

#[tokio::test]
async fn website_assessment_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let website_id = create_test_website(&harness).await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client
        .execute(&format!(
            r#"query {{ websiteAssessment(websiteId: "{}") {{ id }} }}"#,
            website_id
        ))
        .await;

    // May return null if no assessment exists, but should not error
    assert!(
        result.is_ok(),
        "Admin should be able to query websiteAssessment. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn website_assessment_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;
    let website_id = create_test_website(&harness).await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client
        .execute(&format!(
            r#"query {{ websiteAssessment(websiteId: "{}") {{ id }} }}"#,
            website_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to query websiteAssessment"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn website_assessment_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;
    let website_id = create_test_website(&harness).await;

    let client = harness.graphql();
    let result = client
        .execute(&format!(
            r#"query {{ websiteAssessment(websiteId: "{}") {{ id }} }}"#,
            website_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to query websiteAssessment"
    );
    assert_auth_required(&result.errors);
}

// --- member query ---

#[tokio::test]
async fn member_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client
        .execute(&format!(
            r#"query {{ member(id: "{}") {{ id }} }}"#,
            admin_id
        ))
        .await;

    assert!(
        result.is_ok(),
        "Admin should be able to query member. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn member_as_non_admin_fails() {
    let (harness, admin_id, non_admin_id) = setup_test_harness().await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client
        .execute(&format!(
            r#"query {{ member(id: "{}") {{ id }} }}"#,
            admin_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to query member"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn member_unauthenticated_fails() {
    let (harness, admin_id, _) = setup_test_harness().await;

    let client = harness.graphql();
    let result = client
        .execute(&format!(
            r#"query {{ member(id: "{}") {{ id }} }}"#,
            admin_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to query member"
    );
    assert_auth_required(&result.errors);
}

// --- members query ---

#[tokio::test]
async fn members_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client
        .execute(r#"query { members { nodes { id } } }"#)
        .await;

    assert!(
        result.is_ok(),
        "Admin should be able to query members. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn members_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client
        .execute(r#"query { members { nodes { id } } }"#)
        .await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to query members"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn members_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;

    let client = harness.graphql();
    let result = client
        .execute(r#"query { members { nodes { id } } }"#)
        .await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to query members"
    );
    assert_auth_required(&result.errors);
}

// --- provider query ---

#[tokio::test]
async fn provider_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client
        .execute(&format!(
            r#"query {{ provider(id: "{}") {{ id }} }}"#,
            provider_id
        ))
        .await;

    assert!(
        result.is_ok(),
        "Admin should be able to query provider. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn provider_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client
        .execute(&format!(
            r#"query {{ provider(id: "{}") {{ id }} }}"#,
            provider_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to query provider"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn provider_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;
    let provider_id = create_test_provider(&harness).await;

    let client = harness.graphql();
    let result = client
        .execute(&format!(
            r#"query {{ provider(id: "{}") {{ id }} }}"#,
            provider_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to query provider"
    );
    assert_auth_required(&result.errors);
}

// --- providers query ---

#[tokio::test]
async fn providers_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client
        .execute(r#"query { providers { nodes { id } } }"#)
        .await;

    assert!(
        result.is_ok(),
        "Admin should be able to query providers. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn providers_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client
        .execute(r#"query { providers { nodes { id } } }"#)
        .await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to query providers"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn providers_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;

    let client = harness.graphql();
    let result = client
        .execute(r#"query { providers { nodes { id } } }"#)
        .await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to query providers"
    );
    assert_auth_required(&result.errors);
}

// --- organization query ---

/// Create a test organization with unique name
async fn create_test_org(harness: &TestHarness) -> Uuid {
    let org_id = Uuid::new_v4();
    let unique_name = format!("Test Org {}", org_id);
    sqlx::query(
        r#"INSERT INTO organizations (id, name, created_at, updated_at)
           VALUES ($1, $2, NOW(), NOW())"#,
    )
    .bind(org_id)
    .bind(&unique_name)
    .execute(&harness.db_pool)
    .await
    .expect("Failed to create test org");
    org_id
}

#[tokio::test]
async fn organization_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let org_id = create_test_org(&harness).await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client
        .execute(&format!(
            r#"query {{ organization(id: "{}") {{ id }} }}"#,
            org_id
        ))
        .await;

    assert!(
        result.is_ok(),
        "Admin should be able to query organization. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn organization_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;
    let org_id = create_test_org(&harness).await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client
        .execute(&format!(
            r#"query {{ organization(id: "{}") {{ id }} }}"#,
            org_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to query organization"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn organization_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;
    let org_id = create_test_org(&harness).await;

    let client = harness.graphql();
    let result = client
        .execute(&format!(
            r#"query {{ organization(id: "{}") {{ id }} }}"#,
            org_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to query organization"
    );
    assert_auth_required(&result.errors);
}

// --- organizations query ---

#[tokio::test]
async fn organizations_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client
        .execute(r#"query { organizations { nodes { id } } }"#)
        .await;

    assert!(
        result.is_ok(),
        "Admin should be able to query organizations. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn organizations_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client
        .execute(r#"query { organizations { nodes { id } } }"#)
        .await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to query organizations"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn organizations_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;

    let client = harness.graphql();
    let result = client
        .execute(r#"query { organizations { nodes { id } } }"#)
        .await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to query organizations"
    );
    assert_auth_required(&result.errors);
}

// =============================================================================
// create_organization mutation
// =============================================================================

#[tokio::test]
async fn create_organization_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client
        .execute(r#"mutation { createOrganization(name: "Test Org") { id name } }"#)
        .await;

    assert!(
        result.is_ok(),
        "Admin should be able to create organization. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn create_organization_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client
        .execute(r#"mutation { createOrganization(name: "Test Org") { id name } }"#)
        .await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to create organization"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn create_organization_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;

    let client = harness.graphql();
    let result = client
        .execute(r#"mutation { createOrganization(name: "Test Org") { id name } }"#)
        .await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to create organization"
    );
    assert_auth_required(&result.errors);
}

// =============================================================================
// add_organization_tags mutation
// =============================================================================

#[tokio::test]
async fn add_organization_tags_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let org_id = create_test_org(&harness).await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client.execute(&format!(
        r#"mutation {{ addOrganizationTags(organizationId: "{}", tags: [{{ kind: "category", value: "test" }}]) {{ id }} }}"#,
        org_id
    )).await;

    assert!(
        result.is_ok(),
        "Admin should be able to add organization tags. Errors: {:?}",
        result.errors
    );
}

#[tokio::test]
async fn add_organization_tags_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;
    let org_id = create_test_org(&harness).await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client.execute(&format!(
        r#"mutation {{ addOrganizationTags(organizationId: "{}", tags: [{{ kind: "category", value: "test" }}]) {{ id }} }}"#,
        org_id
    )).await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to add organization tags"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn add_organization_tags_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;
    let org_id = create_test_org(&harness).await;

    let client = harness.graphql();
    let result = client.execute(&format!(
        r#"mutation {{ addOrganizationTags(organizationId: "{}", tags: [{{ kind: "category", value: "test" }}]) {{ id }} }}"#,
        org_id
    )).await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to add organization tags"
    );
    assert_auth_required(&result.errors);
}

// =============================================================================
// generate_website_assessment mutation
// =============================================================================

#[tokio::test]
async fn generate_website_assessment_as_admin_succeeds() {
    let (harness, admin_id, _) = setup_test_harness().await;
    let website_id = create_test_website(&harness).await;

    let client = harness.graphql_with_auth(admin_id, true);
    let result = client
        .execute(&format!(
            r#"mutation {{ generateWebsiteAssessment(websiteId: "{}") }}"#,
            website_id
        ))
        .await;

    // This will likely error on missing AI deps in test env, but we only care about auth check
    // If it passes auth, the error will be about something else (not auth)
    if !result.is_ok() {
        let err_str = format!("{:?}", result.errors);
        assert!(
            !err_str.contains("Admin")
                && !err_str.contains("admin")
                && !err_str.contains("Authentication")
                && !err_str.contains("authentication"),
            "Admin should pass auth check. Error: {}",
            err_str
        );
    }
}

#[tokio::test]
async fn generate_website_assessment_as_non_admin_fails() {
    let (harness, _, non_admin_id) = setup_test_harness().await;
    let website_id = create_test_website(&harness).await;

    let client = harness.graphql_with_auth(non_admin_id, false);
    let result = client
        .execute(&format!(
            r#"mutation {{ generateWebsiteAssessment(websiteId: "{}") }}"#,
            website_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Non-admin should NOT be able to generate website assessment"
    );
    assert_admin_required(&result.errors);
}

#[tokio::test]
async fn generate_website_assessment_unauthenticated_fails() {
    let (harness, _, _) = setup_test_harness().await;
    let website_id = create_test_website(&harness).await;

    let client = harness.graphql();
    let result = client
        .execute(&format!(
            r#"mutation {{ generateWebsiteAssessment(websiteId: "{}") }}"#,
            website_id
        ))
        .await;

    assert!(
        !result.is_ok(),
        "Unauthenticated should NOT be able to generate website assessment"
    );
    assert_auth_required(&result.errors);
}
