//! Integration tests for authentication and authorization.
//!
//! Tests all critical auth paths:
//! - Admin-only operations
//! - Member self-service operations
//! - Public operations
//! - Authorization denied scenarios

mod common;

use common::{fixtures, GraphQLClient, TestHarness};
use server_core::common::MemberId;
use server_core::domains::auth::JwtService;
use server_core::kernel::OpenAIClient;
use server_core::server::graphql::GraphQLContext;
use server_core::server::middleware::AuthUser;
use std::sync::Arc;
use test_context::test_context;
use twilio::{TwilioOptions, TwilioService};
use uuid::Uuid;

// ============================================================================
// Test Helpers
// ============================================================================

/// Create test instances of services needed by GraphQLContext
fn create_test_services() -> (Arc<TwilioService>, Arc<JwtService>, Arc<OpenAIClient>) {
    let twilio = Arc::new(TwilioService::new(TwilioOptions {
        account_sid: "test_account_sid".to_string(),
        auth_token: "test_auth_token".to_string(),
        service_id: "test_service_id".to_string(),
    }));
    let jwt_service = Arc::new(JwtService::new(
        "test_secret_key",
        "test_issuer".to_string(),
    ));
    let openai_client = Arc::new(OpenAIClient::new("test_api_key".to_string()));

    (twilio, jwt_service, openai_client)
}

/// Create a GraphQL client with admin authentication
fn graphql_admin(harness: &TestHarness) -> GraphQLClient {
    let auth_user = AuthUser {
        user_id: Uuid::new_v4().to_string(),
        member_id: MemberId::new(),
        phone_number: "+15555551234".to_string(),
        is_admin: true,
    };

    let (twilio, jwt_service, openai_client) = create_test_services();
    let context = GraphQLContext::new(
        harness.db_pool.clone(),
        harness.bus(),
        Some(auth_user),
        twilio,
        jwt_service,
        openai_client,
    );

    GraphQLClient::with_context(context)
}

/// Create a GraphQL client with regular user authentication
fn graphql_user(harness: &TestHarness, member_id: Uuid) -> GraphQLClient {
    let auth_user = AuthUser {
        user_id: member_id.to_string(),
        member_id: MemberId::from_uuid(member_id),
        phone_number: "+15555559999".to_string(),
        is_admin: false,
    };

    let (twilio, jwt_service, openai_client) = create_test_services();
    let context = GraphQLContext::new(
        harness.db_pool.clone(),
        harness.bus(),
        Some(auth_user),
        twilio,
        jwt_service,
        openai_client,
    );

    GraphQLClient::with_context(context)
}

/// Create a GraphQL client with no authentication
fn graphql_public(harness: &TestHarness) -> GraphQLClient {
    let (twilio, jwt_service, openai_client) = create_test_services();
    let context = GraphQLContext::new(
        harness.db_pool.clone(),
        harness.bus(),
        None, // No auth
        twilio,
        jwt_service,
        openai_client,
    );

    GraphQLClient::with_context(context)
}

// ============================================================================
// Admin-Only Operations Tests
// ============================================================================

#[test_context(TestHarness)]
#[tokio::test]
async fn test_approve_listing_requires_admin(ctx: &TestHarness) {
    // Create a test listing in pending_approval status
    let listing_id =
        fixtures::create_test_need_pending(&ctx.db_pool, None, "Test Need", "Description")
            .await
            .unwrap();

    // Try as non-admin - should fail
    let client_user = graphql_user(ctx, Uuid::new_v4());
    let query = format!(
        r#"mutation {{ approveListing(listingId: "{}") {{ id status }} }}"#,
        listing_id
    );
    let result = client_user.execute(&query).await;
    assert!(
        !result.is_ok(),
        "Non-admin should not be able to approve listings"
    );
    assert!(result.errors.iter().any(|e| e.contains("Admin")));

    // Try as admin - should succeed
    let client_admin = graphql_admin(ctx);
    let result = client_admin.execute(&query).await;
    assert!(result.is_ok(), "Admin should be able to approve listings");
}

#[test_context(TestHarness)]
#[tokio::test]
async fn test_delete_listing_requires_admin(ctx: &TestHarness) {
    let listing_id = fixtures::create_test_need_active(&ctx.db_pool, "Test", "Desc")
        .await
        .unwrap();

    // Try as non-admin
    let client_user = graphql_user(ctx, Uuid::new_v4());
    let query = format!(
        r#"mutation {{ deleteListing(listingId: "{}") }}"#,
        listing_id
    );
    let result = client_user.execute(&query).await;
    assert!(!result.is_ok());
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("Admin") || e.contains("Authentication")));

    // Try as admin
    let client_admin = graphql_admin(ctx);
    let result = client_admin.execute(&query).await;
    assert!(result.is_ok(), "Admin should be able to delete listings");
}

#[test_context(TestHarness)]
#[tokio::test]
async fn test_scrape_organization_requires_admin(ctx: &TestHarness) {
    let source_id = fixtures::create_test_source(&ctx.db_pool, "Test Org", "https://example.com")
        .await
        .unwrap();

    // Try as non-admin
    let client_user = graphql_user(ctx, Uuid::new_v4());
    let query = format!(
        r#"mutation {{ scrapeOrganization(sourceId: "{}") {{ status }} }}"#,
        source_id
    );
    let result = client_user.execute(&query).await;
    assert!(!result.is_ok());
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("Admin") || e.contains("Authentication")));

    // Admin case would work but we don't test actual scraping here
}

#[test_context(TestHarness)]
#[tokio::test]
async fn test_add_scrape_url_requires_admin(ctx: &TestHarness) {
    let source_id = fixtures::create_test_source(&ctx.db_pool, "Test", "https://example.com")
        .await
        .unwrap();

    // Try as non-admin
    let client_user = graphql_user(ctx, Uuid::new_v4());
    let query = format!(
        r#"mutation {{ addOrganizationScrapeUrl(sourceId: "{}", url: "https://example.com/new") }}"#,
        source_id
    );
    let result = client_user.execute(&query).await;
    assert!(!result.is_ok());
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("Admin") || e.contains("Authentication")));
}

#[test_context(TestHarness)]
#[tokio::test]
async fn test_remove_scrape_url_requires_admin(ctx: &TestHarness) {
    let source_id = fixtures::create_test_source(&ctx.db_pool, "Test", "https://example.com")
        .await
        .unwrap();

    // Try as non-admin
    let client_user = graphql_user(ctx, Uuid::new_v4());
    let query = format!(
        r#"mutation {{ removeOrganizationScrapeUrl(sourceId: "{}", url: "https://example.com") }}"#,
        source_id
    );
    let result = client_user.execute(&query).await;
    assert!(!result.is_ok());
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("Admin") || e.contains("Authentication")));
}

#[test_context(TestHarness)]
#[tokio::test]
async fn test_reject_listing_requires_admin(ctx: &TestHarness) {
    let listing_id = fixtures::create_test_need_pending(&ctx.db_pool, None, "Test", "Desc")
        .await
        .unwrap();

    // Try as non-admin
    let client_user = graphql_user(ctx, Uuid::new_v4());
    let query = format!(
        r#"mutation {{ rejectListing(listingId: "{}", reason: "Test reason") }}"#,
        listing_id
    );
    let result = client_user.execute(&query).await;
    assert!(!result.is_ok());
}

#[test_context(TestHarness)]
#[tokio::test]
async fn test_repost_listing_requires_admin(ctx: &TestHarness) {
    let listing_id = fixtures::create_test_need_active(&ctx.db_pool, "Test", "Desc")
        .await
        .unwrap();

    // Try as non-admin
    let client_user = graphql_user(ctx, Uuid::new_v4());
    let query = format!(
        r#"mutation {{ repostListing(listingId: "{}") {{ id }} }}"#,
        listing_id
    );
    let result = client_user.execute(&query).await;
    assert!(!result.is_ok());
}

#[test_context(TestHarness)]
#[tokio::test]
async fn test_expire_post_requires_admin(ctx: &TestHarness) {
    // Create a post (requires a listing first)
    let listing_id = fixtures::create_test_need_active(&ctx.db_pool, "Test", "Desc")
        .await
        .unwrap();

    // Create a post for this listing
    let post_id = sqlx::query_scalar!(
        "INSERT INTO organization_posts (need_id, title, description, status)
         VALUES ($1, 'Test Post', 'Desc', 'active') RETURNING id",
        listing_id
    )
    .fetch_one(&ctx.db_pool)
    .await
    .unwrap();

    // Try as non-admin
    let client_user = graphql_user(ctx, Uuid::new_v4());
    let query = format!(r#"mutation {{ expirePost(postId: "{}") }}"#, post_id);
    let result = client_user.execute(&query).await;
    assert!(!result.is_ok());
}

#[test_context(TestHarness)]
#[tokio::test]
async fn test_archive_post_requires_admin(ctx: &TestHarness) {
    let listing_id = fixtures::create_test_need_active(&ctx.db_pool, "Test", "Desc")
        .await
        .unwrap();

    let post_id = sqlx::query_scalar!(
        "INSERT INTO organization_posts (need_id, title, description, status)
         VALUES ($1, 'Test Post', 'Desc', 'active') RETURNING id",
        listing_id
    )
    .fetch_one(&ctx.db_pool)
    .await
    .unwrap();

    // Try as non-admin
    let client_user = graphql_user(ctx, Uuid::new_v4());
    let query = format!(r#"mutation {{ archivePost(postId: "{}") }}"#, post_id);
    let result = client_user.execute(&query).await;
    assert!(!result.is_ok());
}

// ============================================================================
// Member Self-Service Operations Tests
// ============================================================================

#[test_context(TestHarness)]
#[tokio::test]
async fn test_update_own_member_status_allowed(ctx: &TestHarness) {
    // Create a member
    let member_id = sqlx::query_scalar!(
        "INSERT INTO members (expo_push_token, searchable_text, active)
         VALUES ('test_token', 'test member', true) RETURNING id"
    )
    .fetch_one(&ctx.db_pool)
    .await
    .unwrap();

    // Member can update their own status
    let client = graphql_user(ctx, member_id);
    let query = format!(
        r#"mutation {{ updateMemberStatus(memberId: "{}", active: false) {{ id active }} }}"#,
        member_id
    );

    let result = client.execute(&query).await;
    ctx.settle().await;

    assert!(
        result.is_ok(),
        "Member should be able to update their own status"
    );

    // Verify status was updated
    let updated_active = sqlx::query_scalar!("SELECT active FROM members WHERE id = $1", member_id)
        .fetch_one(&ctx.db_pool)
        .await
        .unwrap();
    assert!(
        !updated_active,
        "Member status should be updated to inactive"
    );
}

#[test_context(TestHarness)]
#[tokio::test]
async fn test_update_other_member_status_denied(ctx: &TestHarness) {
    // Create two members
    let member1 = sqlx::query_scalar!(
        "INSERT INTO members (expo_push_token, searchable_text, active)
         VALUES ('token1', 'member 1', true) RETURNING id"
    )
    .fetch_one(&ctx.db_pool)
    .await
    .unwrap();

    let member2 = sqlx::query_scalar!(
        "INSERT INTO members (expo_push_token, searchable_text, active)
         VALUES ('token2', 'member 2', true) RETURNING id"
    )
    .fetch_one(&ctx.db_pool)
    .await
    .unwrap();

    // Member 1 tries to update Member 2's status - should fail
    let client = graphql_user(ctx, member1);
    let query = format!(
        r#"mutation {{ updateMemberStatus(memberId: "{}", active: false) {{ id }} }}"#,
        member2
    );

    let result = client.execute(&query).await;
    ctx.settle().await;

    assert!(
        !result.is_ok(),
        "Member should not be able to update another member's status"
    );
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("own status") || e.contains("admin")));
}

#[test_context(TestHarness)]
#[tokio::test]
async fn test_admin_can_update_any_member_status(ctx: &TestHarness) {
    // Create a member
    let member_id = sqlx::query_scalar!(
        "INSERT INTO members (expo_push_token, searchable_text, active)
         VALUES ('test_token', 'test member', true) RETURNING id"
    )
    .fetch_one(&ctx.db_pool)
    .await
    .unwrap();

    // Admin can update any member's status
    let client = graphql_admin(ctx);
    let query = format!(
        r#"mutation {{ updateMemberStatus(memberId: "{}", active: false) {{ id active }} }}"#,
        member_id
    );

    let result = client.execute(&query).await;
    ctx.settle().await;

    assert!(
        result.is_ok(),
        "Admin should be able to update any member's status"
    );
}

// ============================================================================
// Public Operations Tests
// ============================================================================

#[test_context(TestHarness)]
#[tokio::test]
async fn test_submit_listing_public(ctx: &TestHarness) {
    let client = graphql_public(ctx);
    let query = r#"
        mutation {
            submitListing(input: {
                organizationName: "Public Org"
                title: "Public Need"
                description: "Anyone can submit"
            }) {
                id
                title
            }
        }
    "#;

    let result = client.execute(query).await;
    ctx.settle().await;

    assert!(
        result.is_ok(),
        "Public listing submission should work without auth"
    );
}

#[test_context(TestHarness)]
#[tokio::test]
async fn test_submit_resource_link_public(ctx: &TestHarness) {
    let client = graphql_public(ctx);
    let query = r#"
        mutation {
            submitResourceLink(input: {
                url: "https://example.org/needs"
                context: "Found this resource"
            }) {
                status
            }
        }
    "#;

    let result = client.execute(query).await;
    ctx.settle().await;

    assert!(
        result.is_ok(),
        "Public resource link submission should work without auth"
    );
}

#[test_context(TestHarness)]
#[tokio::test]
async fn test_track_post_view_public(ctx: &TestHarness) {
    // Create a post
    let listing_id = fixtures::create_test_need_active(&ctx.db_pool, "Test", "Desc")
        .await
        .unwrap();

    let post_id = sqlx::query_scalar!(
        "INSERT INTO organization_posts (need_id, title, description, status)
         VALUES ($1, 'Test Post', 'Desc', 'active') RETURNING id",
        listing_id
    )
    .fetch_one(&ctx.db_pool)
    .await
    .unwrap();

    let client = graphql_public(ctx);
    let query = format!(r#"mutation {{ trackPostView(postId: "{}") }}"#, post_id);

    let result = client.execute(&query).await;
    ctx.settle().await;

    assert!(
        result.is_ok(),
        "Public post view tracking should work without auth"
    );
}

#[test_context(TestHarness)]
#[tokio::test]
async fn test_track_post_click_public(ctx: &TestHarness) {
    let listing_id = fixtures::create_test_need_active(&ctx.db_pool, "Test", "Desc")
        .await
        .unwrap();

    let post_id = sqlx::query_scalar!(
        "INSERT INTO organization_posts (need_id, title, description, status)
         VALUES ($1, 'Test Post', 'Desc', 'active') RETURNING id",
        listing_id
    )
    .fetch_one(&ctx.db_pool)
    .await
    .unwrap();

    let client = graphql_public(ctx);
    let query = format!(r#"mutation {{ trackPostClick(postId: "{}") }}"#, post_id);

    let result = client.execute(&query).await;
    ctx.settle().await;

    assert!(
        result.is_ok(),
        "Public post click tracking should work without auth"
    );
}

#[test_context(TestHarness)]
#[tokio::test]
async fn test_register_member_public(ctx: &TestHarness) {
    let client = graphql_public(ctx);
    let query = r#"
        mutation {
            registerMember(
                expoPushToken: "ExponentPushToken[test123]"
                searchableText: "I can help with web development"
                city: "Minneapolis"
                state: "MN"
            ) {
                id
                searchableText
            }
        }
    "#;

    let result = client.execute(query).await;
    ctx.settle().await;

    assert!(
        result.is_ok(),
        "Public member registration should work without auth"
    );
}

// ============================================================================
// Authentication Required Tests
// ============================================================================

#[test_context(TestHarness)]
#[tokio::test]
async fn test_update_member_status_requires_auth(ctx: &TestHarness) {
    let member_id = Uuid::new_v4();

    let client = graphql_public(ctx);
    let query = format!(
        r#"mutation {{ updateMemberStatus(memberId: "{}", active: false) {{ id }} }}"#,
        member_id
    );

    let result = client.execute(&query).await;

    assert!(
        !result.is_ok(),
        "updateMemberStatus should require authentication"
    );
    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("Authentication required")));
}

// ============================================================================
// Authorization Flow Integration Tests
// ============================================================================

#[test_context(TestHarness)]
#[tokio::test]
async fn test_authorization_flow_end_to_end(ctx: &TestHarness) {
    // Create a listing as public user
    let client_public = graphql_public(ctx);
    let query = r#"
        mutation {
            submitListing(input: {
                organizationName: "Community Org"
                title: "Volunteers Needed"
                description: "We need help"
            }) {
                id
                status
            }
        }
    "#;

    let result = client_public.execute(query).await;
    ctx.settle().await;
    assert!(result.is_ok(), "Public submission should work");

    let data = result.data.as_ref().unwrap();
    let listing_id = data["submitListing"]["id"].as_str().unwrap().to_string();
    let status = data["submitListing"]["status"].as_str().unwrap();
    assert_eq!(status, "pending_approval", "New listings should be pending");

    // Try to approve as non-admin - should fail
    let client_user = graphql_user(ctx, Uuid::new_v4());
    let query_approve = format!(
        r#"mutation {{ approveListing(listingId: "{}") {{ id }} }}"#,
        listing_id
    );
    let result = client_user.execute(&query_approve).await;
    assert!(!result.is_ok(), "Non-admin cannot approve");

    // Approve as admin - should succeed
    let client_admin = graphql_admin(ctx);
    let result = client_admin.execute(&query_approve).await;
    ctx.settle().await;
    assert!(result.is_ok(), "Admin can approve");
}
