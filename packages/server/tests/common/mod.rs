//! Integration-test harness.
//!
//! Spins up:
//!   * a fresh Postgres container via `testcontainers`
//!   * migrations applied from `packages/server/migrations/`
//!   * a full `AppState` backed by `TestDependencies`
//!   * an Axum `Router` ready for `tower::ServiceExt::oneshot` calls
//!
//! Plus helpers for seeding a county, issuing a service-client API key, and
//! posting a JSON envelope through the ingest endpoint.

use anyhow::Result;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::runners::AsyncRunner;
use testcontainers::ContainerAsync;
use testcontainers_modules::postgres::Postgres;
use tower::ServiceExt;
use uuid::Uuid;

use server_core::api;
use server_core::api::state::AppState;
use server_core::domains::editions::models::county::County;
use server_core::domains::posts::models::ApiKey;
use server_core::domains::tag::models::tag::Tag;
use server_core::kernel::{ServerDeps, TestDependencies};

pub struct TestHarness {
    /// Kept so the container stays alive for the duration of the test.
    #[allow(dead_code)]
    pub container: ContainerAsync<Postgres>,
    pub pool: PgPool,
    pub deps: Arc<ServerDeps>,
    pub router: Router,
}

impl TestHarness {
    pub async fn new() -> Result<Self> {
        let container = Postgres::default().start().await?;
        let port = container.get_host_port_ipv4(5432).await?;
        let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

        let pool = PgPool::connect(&url).await?;

        // Apply every migration in packages/server/migrations/ in filename order.
        sqlx::migrate!("./migrations").run(&pool).await?;

        let deps_raw = TestDependencies::new().into_server_deps(pool.clone());
        let deps = Arc::new(deps_raw);
        let state = AppState { deps: deps.clone() };
        let router = api::router(state);

        // Seed a real county + the `statewide` tag + a baseline safety tag so
        // the service_area hard-fail path has a known-good slug to hit.
        County::upsert("27053", "Hennepin", "MN", 44.9778, -93.2650, 100, false, &pool).await?;
        Tag::find_or_create("service_area", "hennepin-county", Some("Hennepin County".into()), &pool).await?;
        Tag::find_or_create("service_area", "statewide", Some("Statewide".into()), &pool).await?;
        Tag::find_or_create("safety", "no-id-required", Some("No ID Required".into()), &pool).await?;

        Ok(TestHarness {
            container,
            pool,
            deps,
            router,
        })
    }

    /// Mint an `rsk_test_*` API key with the ingest scope and return the
    /// plaintext token.
    pub async fn issue_test_key(&self) -> Result<String> {
        let issued = ApiKey::issue(
            "integration-test",
            "test",
            &vec!["posts:create".to_string()],
            &self.pool,
        )
        .await?;
        Ok(issued.plaintext)
    }

    /// POST an envelope to `/Posts/create_post`. Returns (status, json body).
    pub async fn ingest(
        &self,
        token: &str,
        idempotency_key: Option<Uuid>,
        body: &Value,
    ) -> Result<(StatusCode, Value)> {
        let mut req = Request::builder()
            .method("POST")
            .uri("/Posts/create_post")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"));
        if let Some(k) = idempotency_key {
            req = req.header("x-idempotency-key", k.to_string());
        }
        let req = req.body(Body::from(serde_json::to_vec(body)?))?;

        let resp = self.router.clone().oneshot(req).await?;
        let status = resp.status();
        let bytes = resp.into_body().collect().await?.to_bytes();
        let json: Value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)?
        };
        Ok((status, json))
    }
}

/// Valid minimum-shape envelope for an `update` post — lets tests mutate one
/// field at a time to test specific validation paths.
pub fn minimal_update_envelope() -> Value {
    serde_json::json!({
        "title": "Sabathani Community Center Extends Tax-Help Hours Through April 29",
        "post_type": "update",
        "weight": "light",
        "priority": 55,
        "body_raw": "The free tax-preparation program at Sabathani Community Center has extended its hours through April 29 to accommodate the surge of late-filers and amended-return requests. Drop-ins accepted Monday–Thursday 4–8 PM and Saturday 10–2; no appointment needed. The program is staffed by IRS-certified volunteers and can handle federal and state returns plus amended returns back three years.",
        "body_light": "Sabathani free tax help extended through April 29.",
        "published_at": "2026-04-20T14:00:00-05:00",
        "source_language": "en",
        "is_evergreen": false,
        "tags": {
            "service_area": ["hennepin-county"],
            "topic": ["community", "employment"],
            "safety": []
        },
        "source": {
            "kind": "organization",
            "organization": {
                "name": "Sabathani Community Center",
                "website": "https://sabathani.org/"
            },
            "source_url": "https://sabathani.org/programs/tax-help-2026/",
            "attribution_line": "Sabathani Community Center program page",
            "extraction_confidence": 93
        },
        "meta": {
            "kicker": "Taxes",
            "byline": "Sabathani Community Center"
        },
        "field_groups": {
            "contacts": [
                { "contact_type": "phone", "contact_value": "612-827-5981", "contact_label": "Tax help line" }
            ]
        },
        "editorial": { "revision_of_post_id": null, "duplicate_of_id": null }
    })
}
