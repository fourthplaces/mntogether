//! API-edge tests for the edition lifecycle state machine.
//!
//! Per CLAUDE.md these hit the HTTP layer and verify via model queries. No
//! raw SQL — fixture setup uses only the models exposed to activities.
//!
//! Coverage:
//!   * batch publish gates per-id on populated slots (the bug that let an
//!     empty Statewide edition go live — an edition that was populated when
//!     approved, then had its slots removed before batch publish ran).
//!   * batch approve has the same gate.
//!   * `publish → unpublish → publish` preserves `published_at` so the
//!     first-publication timestamp survives a revision cycle.
//!   * `unpublish` rejects editions that aren't currently published.

mod common;

use anyhow::Result;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::NaiveDate;
use common::TestHarness;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use server_core::common::PostId;
use server_core::domains::editions::activities;
use server_core::domains::editions::models::county::County;
use server_core::domains::editions::models::edition::Edition;
use server_core::domains::editions::models::edition_row::EditionRow;
use server_core::domains::editions::models::edition_slot::EditionSlot;
use server_core::domains::editions::models::row_template_config::RowTemplateConfig;
use server_core::domains::posts::models::{CreatePost, Post};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

// ─── Fixture helpers ─────────────────────────────────────────────────────────

async fn hennepin_id(pool: &PgPool) -> Result<Uuid> {
    let county = County::find_by_fips("27053", pool)
        .await?
        .expect("harness seeds Hennepin");
    Ok(county.id)
}

async fn any_row_template_id(pool: &PgPool) -> Result<Uuid> {
    let all = RowTemplateConfig::find_all(pool).await?;
    Ok(all.first().expect("migrations seed row templates").id)
}

/// Minimal post, no field groups — enough to satisfy the edition_slot FK.
async fn create_seed_post(pool: &PgPool) -> Result<PostId> {
    let post = Post::create(
        CreatePost::builder()
            .title("Edition-lifecycle test post")
            .body_raw("Short body for a fixture post used by the edition-lifecycle test suite.")
            .post_type("story".to_string())
            .weight("medium".to_string())
            .status("active".to_string())
            .build(),
        pool,
    )
    .await?;
    Ok(post.id)
}

/// Build an edition with a single populated slot so it passes the
/// `require_populated_edition` gate. Returns (edition, slot_id) so tests
/// that need to later un-populate can delete the slot through the model.
async fn populated_edition(h: &TestHarness) -> Result<(Edition, Uuid)> {
    let county = hennepin_id(&h.pool).await?;
    let edition = Edition::create(
        county,
        NaiveDate::from_ymd_opt(2026, 4, 20).unwrap(),
        NaiveDate::from_ymd_opt(2026, 4, 26).unwrap(),
        Some("Hennepin — test week"),
        &h.pool,
    )
    .await?;

    let row_template = any_row_template_id(&h.pool).await?;
    let row = EditionRow::create(edition.id, row_template, 0, &h.pool).await?;
    let post = create_seed_post(&h.pool).await?;
    let slot = EditionSlot::create(row.id, post.into_uuid(), "digest", 0, &h.pool).await?;
    Ok((edition, slot.id))
}

/// Strip every slot off an edition so the populated-slots gate fires on it.
/// Mirrors the real-world failure mode: an editor removed posts from an
/// approved edition and the batch publish didn't re-check before running.
async fn clear_all_slots(edition_id: Uuid, pool: &PgPool) -> Result<()> {
    for slot in EditionSlot::find_by_edition(edition_id, pool).await? {
        EditionSlot::delete(slot.id, pool).await?;
    }
    Ok(())
}

/// POST a JSON body to the admin router. `TestDependencies` installs an
/// AdminUser-bypassing auth path, so no cookie is needed.
async fn post_json(h: &TestHarness, path: &str, body: &Value) -> Result<(StatusCode, Value)> {
    let req = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(body)?))?;

    let resp = h.router.clone().oneshot(req).await?;
    let status = resp.status();
    let bytes = resp.into_body().collect().await?.to_bytes();
    let json: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)?
    };
    Ok((status, json))
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn batch_publish_skips_empty_editions_with_reason() {
    let h = TestHarness::new().await.expect("harness");
    let (edition, _slot) = populated_edition(&h).await.expect("fixture");

    // Walk to `approved` legitimately (gates fire on populated edition)…
    activities::review_edition(edition.id, &h.deps).await.unwrap();
    activities::approve_edition(edition.id, &h.deps).await.unwrap();
    // …then strip the slot so batch publish sees an empty edition. This is
    // the real bug scenario: the raw-SQL batch used to flip status without
    // re-checking after slots were removed.
    clear_all_slots(edition.id, &h.pool).await.unwrap();

    let (status, body) = post_json(
        &h,
        "/Editions/batch_publish_editions",
        &json!({ "ids": [edition.id] }),
    )
    .await
    .expect("post");

    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert_eq!(body["succeeded"].as_i64(), Some(0), "nothing should publish");
    assert_eq!(body["failed"].as_i64(), Some(1));

    let errors = body["errors"].as_array().expect("errors array");
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0]["edition_id"].as_str(),
        Some(edition.id.to_string().as_str())
    );
    let msg = errors[0]["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("no populated slots"),
        "server should explain the skip reason, got: {msg}"
    );

    // Verify via model — the edition didn't actually flip.
    let fresh = Edition::find_by_id(edition.id, &h.pool)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fresh.status, "approved", "batch must not have published");
    assert!(fresh.published_at.is_none(), "published_at stayed null");
}

#[tokio::test]
async fn batch_approve_skips_empty_editions_with_reason() {
    let h = TestHarness::new().await.expect("harness");
    let (edition, _slot) = populated_edition(&h).await.expect("fixture");

    // Reach in_review legitimately, then empty the slots before batch approve.
    activities::review_edition(edition.id, &h.deps).await.unwrap();
    clear_all_slots(edition.id, &h.pool).await.unwrap();

    let (status, body) = post_json(
        &h,
        "/Editions/batch_approve_editions",
        &json!({ "ids": [edition.id] }),
    )
    .await
    .expect("post");

    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert_eq!(body["succeeded"].as_i64(), Some(0));
    assert_eq!(body["failed"].as_i64(), Some(1));
    let msg = body["errors"][0]["message"].as_str().unwrap_or_default();
    assert!(msg.contains("no populated slots"), "got: {msg}");

    let fresh = Edition::find_by_id(edition.id, &h.pool)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fresh.status, "in_review");
}

#[tokio::test]
async fn publish_unpublish_republish_preserves_published_at() {
    let h = TestHarness::new().await.expect("harness");
    let (edition, _slot) = populated_edition(&h).await.expect("fixture");

    activities::review_edition(edition.id, &h.deps).await.unwrap();
    activities::approve_edition(edition.id, &h.deps).await.unwrap();
    let first_publish = activities::publish_edition(edition.id, &h.deps)
        .await
        .unwrap();
    let original_ts = first_publish
        .published_at
        .expect("publish stamps published_at");

    // Unpublish via the HTTP endpoint (the path the admin UI calls).
    let (status, body) = post_json(
        &h,
        "/Editions/unpublish_edition",
        &json!({ "id": edition.id }),
    )
    .await
    .expect("post");
    assert_eq!(status, StatusCode::OK, "body = {body}");
    assert_eq!(body["status"].as_str(), Some("approved"));

    let after_unpublish = Edition::find_by_id(edition.id, &h.pool)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after_unpublish.status, "approved");
    assert_eq!(
        after_unpublish.published_at, first_publish.published_at,
        "published_at preserved across unpublish"
    );

    // Re-publish — COALESCE in the UPDATE must keep the original stamp so
    // "first went live at X" survives the edit cycle.
    let republished = activities::publish_edition(edition.id, &h.deps)
        .await
        .unwrap();
    assert_eq!(republished.status, "published");
    assert_eq!(
        republished.published_at,
        Some(original_ts),
        "re-publish keeps original published_at (first-publication is audit)"
    );
}

#[tokio::test]
async fn unpublish_requires_published_status() {
    let h = TestHarness::new().await.expect("harness");
    let (edition, _slot) = populated_edition(&h).await.expect("fixture");

    activities::review_edition(edition.id, &h.deps).await.unwrap();
    activities::approve_edition(edition.id, &h.deps).await.unwrap();

    let err = activities::unpublish_edition(edition.id, &h.deps)
        .await
        .expect_err("unpublish must reject non-published editions");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("only published editions"),
        "error should name the state-machine rule, got: {msg}"
    );

    let fresh = Edition::find_by_id(edition.id, &h.pool)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fresh.status, "approved", "state didn't change on error");
}
