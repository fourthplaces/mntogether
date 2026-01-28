//! Integration tests for Organization Need GraphQL endpoints.
//!
//! Tests need queries, mutations, and human-in-the-loop approval workflow.

mod common;

use crate::common::{
    clean_needs, create_test_need_active, create_test_need_full, create_test_need_pending,
    TestHarness,
};
use serde_json::json;
use server_core::domains::organization::models::NeedStatus;
use test_context::test_context;

// =============================================================================
// Need Queries
// Tests for fetching need data via GraphQL queries
// =============================================================================

/// Query active needs returns only needs with active status.
#[test_context(TestHarness)]
#[tokio::test]
async fn query_needs_returns_only_active(ctx: &TestHarness) {
    clean_needs(&ctx.db_pool).await.unwrap();
    let client = ctx.graphql();

    // Create needs with different statuses
    create_test_need_active(&ctx.db_pool, "Active Need 1", "Description 1")
        .await
        .unwrap();
    create_test_need_active(&ctx.db_pool, "Active Need 2", "Description 2")
        .await
        .unwrap();
    create_test_need_pending(&ctx.db_pool, None, "Pending Need", "Description 3")
        .await
        .unwrap();

    let query = r#"
        query GetNeeds {
            needs {
                nodes {
                    id
                    title
                    status
                }
                totalCount
            }
        }
    "#;

    let result = client.query(query).await;

    assert_eq!(result["needs"]["totalCount"].as_i64().unwrap(), 2);
    let nodes = result["needs"]["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 2);

    for node in nodes {
        assert_eq!(node["status"].as_str().unwrap(), "ACTIVE");
    }
}

/// Query needs with pending_approval status returns only pending needs.
#[test_context(TestHarness)]
#[tokio::test]
async fn query_needs_with_status_filter(ctx: &TestHarness) {
    clean_needs(&ctx.db_pool).await.unwrap();
    let client = ctx.graphql();

    create_test_need_active(&ctx.db_pool, "Active Need", "Description 1")
        .await
        .unwrap();
    create_test_need_pending(&ctx.db_pool, None, "Pending Need 1", "Description 2")
        .await
        .unwrap();
    create_test_need_pending(&ctx.db_pool, None, "Pending Need 2", "Description 3")
        .await
        .unwrap();

    let query = r#"
        query GetNeeds {
            needs(status: PENDING_APPROVAL) {
                nodes {
                    id
                    title
                    status
                }
                totalCount
            }
        }
    "#;

    let result = client.query(query).await;

    assert_eq!(result["needs"]["totalCount"].as_i64().unwrap(), 2);
    let nodes = result["needs"]["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 2);

    for node in nodes {
        assert_eq!(node["status"].as_str().unwrap(), "PENDING_APPROVAL");
    }
}

/// Query needs supports pagination with limit and offset.
#[test_context(TestHarness)]
#[tokio::test]
async fn query_needs_pagination(ctx: &TestHarness) {
    clean_needs(&ctx.db_pool).await.unwrap();
    let client = ctx.graphql();

    // Create 5 active needs
    for i in 1..=5 {
        create_test_need_active(&ctx.db_pool, &format!("Need {}", i), "Description")
            .await
            .unwrap();
    }

    let query = r#"
        query GetNeeds($limit: Int, $offset: Int) {
            needs(limit: $limit, offset: $offset) {
                nodes {
                    id
                    title
                }
                totalCount
                hasNextPage
            }
        }
    "#;

    // First page (limit 2, offset 0)
    let result = client
        .query_with_vars(query, vars!("limit" => 2, "offset" => 0))
        .await;

    assert_eq!(result["needs"]["totalCount"].as_i64().unwrap(), 5);
    assert_eq!(result["needs"]["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(result["needs"]["hasNextPage"].as_bool().unwrap(), true);

    // Second page (limit 2, offset 2)
    let result = client
        .query_with_vars(query, vars!("limit" => 2, "offset" => 2))
        .await;

    assert_eq!(result["needs"]["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(result["needs"]["hasNextPage"].as_bool().unwrap(), true);

    // Last page (limit 2, offset 4)
    let result = client
        .query_with_vars(query, vars!("limit" => 2, "offset" => 4))
        .await;

    assert_eq!(result["needs"]["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(result["needs"]["hasNextPage"].as_bool().unwrap(), false);
}

/// Query single need by ID returns full details.
#[test_context(TestHarness)]
#[tokio::test]
async fn query_need_by_id_returns_full_details(ctx: &TestHarness) {
    clean_needs(&ctx.db_pool).await.unwrap();
    let client = ctx.graphql();

    let contact_json = json!({
        "email": "volunteer@example.org",
        "phone": "(612) 555-1234"
    });

    let need_id = create_test_need_full(
        &ctx.db_pool,
        "Community Center",
        "English Tutors Needed",
        "We need volunteers to teach English to refugee families",
        "English tutors needed for refugee families",
        Some(contact_json),
        Some("urgent"),
        NeedStatus::Active,
    )
    .await
    .unwrap();

    let query = r#"
        query GetNeed($id: ID!) {
            need(id: $id) {
                id
                organizationName
                title
                tldr
                description
                contactInfo {
                    email
                    phone
                }
                urgency
                status
            }
        }
    "#;

    let result = client
        .query_with_vars(query, vars!("id" => need_id.to_string()))
        .await;

    let need = &result["need"];
    assert_eq!(
        need["organizationName"].as_str().unwrap(),
        "Community Center"
    );
    assert_eq!(need["title"].as_str().unwrap(), "English Tutors Needed");
    assert_eq!(
        need["tldr"].as_str().unwrap(),
        "English tutors needed for refugee families"
    );
    assert_eq!(
        need["contactInfo"]["email"].as_str().unwrap(),
        "volunteer@example.org"
    );
    assert_eq!(need["urgency"].as_str().unwrap(), "urgent");
    assert_eq!(need["status"].as_str().unwrap(), "ACTIVE");
}

// =============================================================================
// Human-in-the-Loop Approval Tests
// Tests for admin approval workflow
// =============================================================================

/// Approving a need changes status to active.
#[test_context(TestHarness)]
#[tokio::test]
async fn approve_need_changes_status_to_active(ctx: &TestHarness) {
    clean_needs(&ctx.db_pool).await.unwrap();
    let client = ctx.graphql();

    let need_id = create_test_need_pending(
        &ctx.db_pool,
        None,
        "Spanish Translator Needed",
        "We need Spanish translators",
    )
    .await
    .unwrap();

    let mutation = r#"
        mutation ApproveNeed($needId: ID!) {
            approveNeed(needId: $needId) {
                id
                status
            }
        }
    "#;

    let result = client
        .query_with_vars(mutation, vars!("needId" => need_id.to_string()))
        .await;

    assert_eq!(result["approveNeed"]["status"].as_str().unwrap(), "ACTIVE");

    // Verify in database
    let row = sqlx::query!(
        r#"SELECT status FROM organization_needs WHERE id = $1"#,
        need_id
    )
    .fetch_one(&ctx.db_pool)
    .await
    .unwrap();

    assert_eq!(row.status.as_deref().unwrap(), "active");
}

/// Editing and approving a need updates content and status.
#[test_context(TestHarness)]
#[tokio::test]
async fn edit_and_approve_need_updates_content(ctx: &TestHarness) {
    clean_needs(&ctx.db_pool).await.unwrap();
    let client = ctx.graphql();

    let need_id = create_test_need_pending(
        &ctx.db_pool,
        None,
        "Food Pantry Help",
        "We need help with food pantry",
    )
    .await
    .unwrap();

    let mutation = r#"
        mutation EditAndApproveNeed($needId: ID!, $input: EditNeedInput!) {
            editAndApproveNeed(needId: $needId, input: $input) {
                id
                title
                description
                tldr
                status
            }
        }
    "#;

    // Note: This is simplified - in actual test we'd need proper input object construction
    let input = json!({
        "title": "Food Pantry Volunteers Needed",
        "description": "Help us sort and distribute food donations every Saturday",
        "tldr": "Food pantry volunteers needed on Saturdays"
    });

    let result = client
        .query_with_vars(
            mutation,
            vars!("needId" => need_id.to_string(), "input" => input),
        )
        .await;

    assert_eq!(
        result["editAndApproveNeed"]["title"].as_str().unwrap(),
        "Food Pantry Volunteers Needed"
    );
    assert_eq!(
        result["editAndApproveNeed"]["tldr"].as_str().unwrap(),
        "Food pantry volunteers needed on Saturdays"
    );
    assert_eq!(
        result["editAndApproveNeed"]["status"].as_str().unwrap(),
        "ACTIVE"
    );
}

/// Rejecting a need changes status to rejected.
#[test_context(TestHarness)]
#[tokio::test]
async fn reject_need_changes_status_to_rejected(ctx: &TestHarness) {
    clean_needs(&ctx.db_pool).await.unwrap();
    let client = ctx.graphql();

    let need_id = create_test_need_pending(&ctx.db_pool, None, "Spam Need", "This is clearly spam")
        .await
        .unwrap();

    let mutation = r#"
        mutation RejectNeed($needId: ID!, $reason: String!) {
            rejectNeed(needId: $needId, reason: $reason)
        }
    "#;

    let result = client
        .query_with_vars(
            mutation,
            vars!("needId" => need_id.to_string(), "reason" => "Spam content"),
        )
        .await;

    assert_eq!(result["rejectNeed"].as_bool().unwrap(), true);

    // Verify in database
    let row = sqlx::query!(
        r#"SELECT status FROM organization_needs WHERE id = $1"#,
        need_id
    )
    .fetch_one(&ctx.db_pool)
    .await
    .unwrap();

    assert_eq!(row.status.as_deref().unwrap(), "rejected");
}

// =============================================================================
// Content Hash Sync Tests
// Tests for duplicate detection
// =============================================================================

/// Content hash is generated for new needs.
#[test_context(TestHarness)]
#[tokio::test]
async fn content_hash_generated_for_new_needs(ctx: &TestHarness) {
    clean_needs(&ctx.db_pool).await.unwrap();

    let need_id = create_test_need_pending(&ctx.db_pool, None, "Test Need", "Test description")
        .await
        .unwrap();

    let row = sqlx::query!(
        r#"SELECT content_hash FROM organization_needs WHERE id = $1"#,
        need_id
    )
    .fetch_one(&ctx.db_pool)
    .await
    .unwrap();

    // Content hash should be generated (64 hex chars from SHA256)
    let hash = row.content_hash.unwrap();
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}
