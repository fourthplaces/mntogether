//! API-edge tests for `POST /Posts/create_post` (Root Signal ingest).
//!
//! Per CLAUDE.md these must hit the HTTP layer — never bypass the API. All
//! state checks go through model methods, not raw SQL.
//!
//! Coverage:
//!   * valid envelope → 201 with the documented response shape
//!   * body-length and required-field 422s
//!   * editor-only field rejection
//!   * idempotent retries (same post_id, `idempotency_key_seen_before: true`)
//!   * idempotency conflict (same key, different payload → 409)
//!   * content-hash dedup (no duplicate insert, refreshed published_at)
//!   * organisation dedup (no duplicate org row)
//!   * individual-source consent gate (soft-fail → in_review)
//!   * revision: prior post archived
//!   * multi-citation (citations[] → citation_ids[] in 201)
//!   * unknown service_area → 422; unknown topic → auto-creates + in_review
//!   * auth: missing Bearer → 401; wrong scope → 403

mod common;

use common::{minimal_update_envelope, TestHarness};
use server_core::common::PostId;
use server_core::domains::organization::models::Organization;
use server_core::domains::posts::models::{Post, PostSource};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn valid_envelope_returns_201_and_post_id() {
    let h = TestHarness::new().await.expect("harness");
    let token = h.issue_test_key().await.expect("issue key");

    let env = minimal_update_envelope();
    let (status, body) = h.ingest(&token, Some(Uuid::now_v7()), &env).await.expect("ingest");

    assert_eq!(status.as_u16(), 201, "body = {body}");
    let post_id = body
        .get("post_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .expect("post_id is UUID");
    assert_eq!(body["status"].as_str(), Some("active"));
    assert!(body["organization_id"].is_string(), "organization_id returned");

    // Verify via model — post actually landed with submission_type = 'ingested'.
    let post = Post::find_by_id(PostId::from_uuid(post_id), &h.pool)
        .await
        .expect("query")
        .expect("post exists");
    assert_eq!(post.submission_type.as_deref(), Some("ingested"));
    assert_eq!(post.status, "active");
    assert_eq!(post.post_type, "update");
}

#[tokio::test]
async fn body_raw_too_short_returns_422_below_min_length() {
    let h = TestHarness::new().await.expect("harness");
    let token = h.issue_test_key().await.expect("key");

    let mut env = minimal_update_envelope();
    env["body_raw"] = json!("Too short.");
    let (status, body) = h.ingest(&token, Some(Uuid::now_v7()), &env).await.expect("ingest");

    assert_eq!(status.as_u16(), 422);
    let errors = body["errors"].as_array().expect("errors array");
    assert!(errors
        .iter()
        .any(|e| e["field"] == "body_raw" && e["code"] == "below_min_length"));
}

#[tokio::test]
async fn editor_only_fields_rejected() {
    let h = TestHarness::new().await.expect("harness");
    let token = h.issue_test_key().await.expect("key");

    let mut env = minimal_update_envelope();
    env["is_urgent"] = json!(true);
    env["pencil_mark"] = json!("star");
    let (status, body) = h.ingest(&token, Some(Uuid::now_v7()), &env).await.expect("ingest");

    assert_eq!(status.as_u16(), 422);
    let errors = body["errors"].as_array().unwrap();
    let codes: Vec<_> = errors.iter().map(|e| e["code"].as_str().unwrap()).collect();
    assert!(codes.iter().any(|&c| c == "editor_only_field"));
    // Both fields flagged, not just one.
    let fields: Vec<_> = errors.iter().map(|e| e["field"].as_str().unwrap()).collect();
    assert!(fields.contains(&"is_urgent"));
    assert!(fields.contains(&"pencil_mark"));
}

#[tokio::test]
async fn unknown_service_area_hard_fails() {
    let h = TestHarness::new().await.expect("harness");
    let token = h.issue_test_key().await.expect("key");

    let mut env = minimal_update_envelope();
    env["tags"]["service_area"] = json!(["not-a-real-county"]);
    let (status, body) = h.ingest(&token, Some(Uuid::now_v7()), &env).await.expect("ingest");

    assert_eq!(status.as_u16(), 422);
    let errors = body["errors"].as_array().unwrap();
    assert!(errors.iter().any(|e| e["code"] == "unknown_service_area"));
}

#[tokio::test]
async fn unknown_topic_auto_creates_and_lands_in_review() {
    let h = TestHarness::new().await.expect("harness");
    let token = h.issue_test_key().await.expect("key");

    let mut env = minimal_update_envelope();
    env["tags"]["topic"] = json!(["a-brand-new-topic-slug"]);
    let (status, body) = h.ingest(&token, Some(Uuid::now_v7()), &env).await.expect("ingest");

    assert_eq!(status.as_u16(), 201, "body = {body}");
    assert_eq!(body["status"], "in_review");
}

#[tokio::test]
async fn idempotent_retry_returns_same_post_id() {
    let h = TestHarness::new().await.expect("harness");
    let token = h.issue_test_key().await.expect("key");
    let idem = Uuid::now_v7();
    let env = minimal_update_envelope();

    let (s1, b1) = h.ingest(&token, Some(idem), &env).await.expect("first");
    let (s2, b2) = h.ingest(&token, Some(idem), &env).await.expect("second");

    assert_eq!(s1.as_u16(), 201);
    assert_eq!(s2.as_u16(), 201);
    assert_eq!(b1["post_id"], b2["post_id"], "same post_id on retry");
    assert_eq!(b2["idempotency_key_seen_before"], true);
}

#[tokio::test]
async fn idempotency_conflict_different_payload_returns_409() {
    let h = TestHarness::new().await.expect("harness");
    let token = h.issue_test_key().await.expect("key");
    let idem = Uuid::now_v7();

    let env_a = minimal_update_envelope();
    let mut env_b = minimal_update_envelope();
    env_b["title"] = json!("A Completely Different Title That Still Meets The Length Rule");

    let (s1, _) = h.ingest(&token, Some(idem), &env_a).await.expect("first");
    assert_eq!(s1.as_u16(), 201);

    let (s2, _body) = h.ingest(&token, Some(idem), &env_b).await.expect("second");
    assert_eq!(s2.as_u16(), 409);
}

#[tokio::test]
async fn content_hash_dedup_returns_existing_post() {
    let h = TestHarness::new().await.expect("harness");
    let token = h.issue_test_key().await.expect("key");

    let env = minimal_update_envelope();
    // Two different idempotency keys — dedup must happen on content hash.
    let (s1, b1) = h.ingest(&token, Some(Uuid::now_v7()), &env).await.expect("first");
    let (s2, b2) = h.ingest(&token, Some(Uuid::now_v7()), &env).await.expect("second");

    assert_eq!(s1.as_u16(), 201);
    assert_eq!(s2.as_u16(), 201);
    assert_eq!(b1["post_id"], b2["post_id"], "content-hash dedup returns same post");
}

#[tokio::test]
async fn organization_dedup_reuses_existing_org() {
    let h = TestHarness::new().await.expect("harness");
    let token = h.issue_test_key().await.expect("key");

    // Two posts with same org website → one org row.
    let mut env_a = minimal_update_envelope();
    env_a["title"] = json!("First Post About Sabathani Tax Program 2026");
    env_a["source"]["source_url"] = json!("https://sabathani.org/programs/tax-help-2026/a");
    let (sa, ba) = h.ingest(&token, Some(Uuid::now_v7()), &env_a).await.expect("a");
    assert_eq!(sa.as_u16(), 201);

    let mut env_b = minimal_update_envelope();
    env_b["title"] = json!("Second Post About Sabathani Tax Program 2026");
    env_b["source"]["source_url"] = json!("https://sabathani.org/programs/tax-help-2026/b");
    let (sb, bb) = h.ingest(&token, Some(Uuid::now_v7()), &env_b).await.expect("b");
    assert_eq!(sb.as_u16(), 201);

    assert_eq!(
        ba["organization_id"], bb["organization_id"],
        "dedup ladder: same website domain → same org_id"
    );

    // One organization row in total.
    let all = Organization::list(&h.pool).await.expect("list orgs");
    let sabathani_count = all.iter().filter(|o| o.name == "Sabathani Community Center").count();
    assert_eq!(sabathani_count, 1, "exactly one Sabathani org row");
}

#[tokio::test]
async fn individual_source_no_consent_lands_in_review() {
    let h = TestHarness::new().await.expect("harness");
    let token = h.issue_test_key().await.expect("key");

    let mut env = minimal_update_envelope();
    env["source"] = json!({
        "kind": "individual",
        "individual": {
            "display_name": "Jamie Ochoa",
            "handle": "jamielocal",
            "platform": "instagram",
            "platform_url": "https://instagram.com/jamielocal",
            "verified_identity": false,
            "consent_to_publish": false
        },
        "source_url": "https://instagram.com/p/Abc123/",
        "attribution_line": "Instagram: @jamielocal",
        "extraction_confidence": 88
    });

    let (status, body) = h.ingest(&token, Some(Uuid::now_v7()), &env).await.expect("ingest");
    assert_eq!(status.as_u16(), 201, "body = {body}");
    assert_eq!(body["status"], "in_review");
    assert!(body["individual_id"].is_string());
}

#[tokio::test]
async fn revision_archives_prior_post() {
    let h = TestHarness::new().await.expect("harness");
    let token = h.issue_test_key().await.expect("key");

    let (s1, b1) = h
        .ingest(&token, Some(Uuid::now_v7()), &minimal_update_envelope())
        .await
        .expect("first");
    assert_eq!(s1.as_u16(), 201);
    let prior_id: Uuid =
        serde_json::from_value(b1["post_id"].clone()).expect("post_id parses");

    let mut env2 = minimal_update_envelope();
    env2["title"] = json!("Sabathani Tax Help Extended Further Through May 3 (Correction)");
    env2["editorial"] = json!({
        "revision_of_post_id": prior_id.to_string(),
        "duplicate_of_id": null
    });
    let (s2, _b2) = h.ingest(&token, Some(Uuid::now_v7()), &env2).await.expect("revision");
    assert_eq!(s2.as_u16(), 201);

    let prior = Post::find_by_id(PostId::from_uuid(prior_id), &h.pool)
        .await
        .expect("query")
        .expect("exists");
    assert_eq!(prior.status, "archived", "prior post archived on revision");
}

#[tokio::test]
async fn multi_citation_returns_citation_ids() {
    let h = TestHarness::new().await.expect("harness");
    let token = h.issue_test_key().await.expect("key");

    let mut env = minimal_update_envelope();
    env["citations"] = json!([
        {
            "source_url": "https://sabathani.org/programs/tax-help-2026/",
            "retrieved_at": "2026-04-20T14:00:00Z",
            "content_hash": format!("sha256:{}", "a".repeat(64)),
            "snippet": "Sabathani tax-prep extended through April 29.",
            "confidence": 93,
            "is_primary": true,
            "kind": "organization",
            "organization": { "name": "Sabathani Community Center", "website": "https://sabathani.org/" }
        },
        {
            "source_url": "https://www.example-newspaper.com/sabathani-tax-help",
            "retrieved_at": "2026-04-20T14:30:00Z",
            "content_hash": format!("sha256:{}", "b".repeat(64)),
            "confidence": 78,
            "kind": "organization",
            "organization": { "name": "Example Neighborhood Paper", "website": "https://www.example-newspaper.com/" }
        }
    ]);

    let (status, body) = h.ingest(&token, Some(Uuid::now_v7()), &env).await.expect("ingest");
    assert_eq!(status.as_u16(), 201, "body = {body}");
    let citation_ids = body["citation_ids"].as_array().expect("citation_ids array");
    assert_eq!(citation_ids.len(), 2, "one row per citation");

    // Verify via model.
    let post_id = Uuid::parse_str(body["post_id"].as_str().unwrap()).unwrap();
    let sources = PostSource::find_by_post(PostId::from_uuid(post_id), &h.pool)
        .await
        .expect("sources");
    assert_eq!(sources.len(), 2);
    let primary_count = sources.iter().filter(|s| s.is_primary).count();
    assert_eq!(primary_count, 1, "exactly one primary citation");
}

#[tokio::test]
async fn missing_bearer_returns_401() {
    let h = TestHarness::new().await.expect("harness");

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/Posts/create_post")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&minimal_update_envelope()).unwrap(),
        ))
        .unwrap();

    use tower::ServiceExt;
    let resp = h.router.clone().oneshot(req).await.expect("call");
    assert_eq!(resp.status().as_u16(), 401);
}

#[tokio::test]
async fn key_without_scope_returns_403() {
    let h = TestHarness::new().await.expect("harness");
    // Issue a key with no scope.
    let issued = server_core::domains::posts::models::ApiKey::issue(
        "noscope-client",
        "test",
        &vec![],
        &h.pool,
    )
    .await
    .expect("issue");

    let (status, body) = h
        .ingest(&issued.plaintext, Some(Uuid::now_v7()), &minimal_update_envelope())
        .await
        .expect("ingest");
    assert_eq!(status.as_u16(), 403, "body = {body}");
}

#[tokio::test]
async fn editorial_kind_rejected() {
    let h = TestHarness::new().await.expect("harness");
    let token = h.issue_test_key().await.expect("key");

    let mut env = minimal_update_envelope();
    env["source"]["kind"] = json!("editorial");
    let (status, body) = h.ingest(&token, Some(Uuid::now_v7()), &env).await.expect("ingest");

    assert_eq!(status.as_u16(), 422);
    let errors = body["errors"].as_array().unwrap();
    assert!(errors
        .iter()
        .any(|e| e["code"] == "editorial_source_forbidden"));
}
