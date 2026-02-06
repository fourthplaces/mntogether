//! Test harness with testcontainers for integration testing.
//!
//! Uses shared containers across all tests for dramatically improved performance.
//! Containers and migrations are initialized once on first test, then reused.

use anyhow::{Context, Result};
use seesaw_core::{Engine, Runtime, RuntimeConfig};
use seesaw_postgres::PostgresStore;
use server_core::domains::agents::effects::agent_effect;
use server_core::domains::auth::effects::auth_effect;
use server_core::domains::crawling::effects::{crawling_pipeline_effect, mark_no_listings_effect};
use server_core::domains::member::effects::member_effect;
use server_core::domains::posts::effects::post_composite_effect;
use server_core::domains::providers::effects::provider_effect;
use server_core::domains::website::effects::website_effect;
use server_core::domains::website_approval::effects::website_approval_effect;
use server_core::kernel::{ServerDeps, TestDependencies};
use server_core::server::graphql::context::AppQueueEngine;
use sqlx::PgPool;
use std::sync::Arc;
use test_context::AsyncTestContext;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};
use testcontainers_modules::redis::Redis;
use tokio::sync::OnceCell;

use super::GraphQLClient;

// =============================================================================
// Shared Test Infrastructure
// =============================================================================

/// Shared test infrastructure that persists across all tests.
/// Containers are started once and reused, migrations run once.
struct SharedTestInfra {
    db_url: String,
    redis_url: String,
    // Keep containers alive for the entire test run
    _postgres: ContainerAsync<GenericImage>,
    _redis: ContainerAsync<Redis>,
}

/// Global shared infrastructure - initialized once, reused by all tests.
static SHARED_INFRA: OnceCell<SharedTestInfra> = OnceCell::const_new();

impl SharedTestInfra {
    /// Initialize shared infrastructure (containers + migrations).
    /// This is called once on the first test.
    async fn init() -> Result<Self> {
        // Initialize tracing subscriber to respect RUST_LOG environment variable.
        // Uses try_init() to avoid panicking if already initialized.
        // Run tests with: RUST_LOG=debug cargo test -- --nocapture
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_test_writer()
            .try_init();

        // Start Postgres container with pgvector
        // Use pgvector/pgvector image which includes the vector extension
        let postgres = GenericImage::new("pgvector/pgvector", "pg16")
            .with_wait_for(testcontainers::core::WaitFor::message_on_stderr(
                "database system is ready to accept connections",
            ))
            .with_exposed_port(testcontainers::core::ContainerPort::Tcp(5432))
            .with_env_var("POSTGRES_PASSWORD", "postgres")
            .with_env_var("POSTGRES_USER", "postgres")
            .with_env_var("POSTGRES_DB", "postgres")
            .start()
            .await
            .context("Failed to start Postgres container")?;

        let pg_host = postgres.get_host().await?;
        let pg_port = postgres.get_host_port_ipv4(5432).await?;
        let db_url = format!(
            "postgresql://postgres:postgres@{}:{}/postgres",
            pg_host, pg_port
        );

        // Start Redis container
        let redis = Redis::default()
            .start()
            .await
            .context("Failed to start Redis container")?;

        let redis_host = redis.get_host().await?;
        let redis_port = redis.get_host_port_ipv4(6379).await?;
        let redis_url = format!("redis://{}:{}", redis_host, redis_port);

        // Run migrations once on the shared database
        let pool = PgPool::connect(&db_url)
            .await
            .context("Failed to connect to Postgres for migrations")?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .context("Failed to run migrations")?;

        Ok(Self {
            db_url,
            redis_url,
            _postgres: postgres,
            _redis: redis,
        })
    }

    /// Get or initialize the shared infrastructure.
    pub(super) async fn get() -> &'static Self {
        SHARED_INFRA
            .get_or_init(|| async {
                Self::init()
                    .await
                    .expect("Failed to initialize shared test infrastructure")
            })
            .await
    }
}

/// Test harness that manages test infrastructure.
///
/// Uses shared containers across all tests for fast test execution.
/// Each test gets fresh ServerDeps and a QueueEngine instance, but reuses
/// the same database and Redis containers.
///
/// # Example using test-context
///
/// ```ignore
/// use test_context::test_context;
///
/// #[test_context(TestHarness)]
/// #[tokio::test]
/// async fn my_test(ctx: &TestHarness) {
///     let client = ctx.graphql();
///     // ... test code
/// }
/// ```
pub struct TestHarness {
    /// Server dependencies for direct access
    pub server_deps: Arc<ServerDeps>,
    /// Database pool - use this for test fixtures.
    pub db_pool: PgPool,
    /// Test dependencies for accessing mocks
    pub deps: TestDependencies,
    /// Queue engine for processing events
    pub queue_engine: Arc<AppQueueEngine>,
}

impl TestHarness {
    /// Creates a new test harness using shared containers.
    ///
    /// This will:
    /// 1. Get or initialize shared PostgreSQL and Redis containers
    /// 2. Run database migrations (only on first call)
    /// 3. Initialize ServerDeps with test dependencies
    /// 4. Build QueueEngine with all domain effects
    pub async fn new() -> Result<Self> {
        Self::with_deps(TestDependencies::new()).await
    }

    /// Creates a test harness with custom dependencies.
    ///
    /// Use this to inject mock services with pre-configured responses.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use server_core::kernel::TestDependencies;
    ///
    /// let deps = TestDependencies::new()
    ///     .mock_ingestor(MockIngestor::new());
    /// let harness = TestHarness::with_deps(deps).await?;
    /// ```
    pub async fn with_deps(deps: TestDependencies) -> Result<Self> {
        // Get shared infrastructure (containers start + migrations run on first call only)
        let infra = SharedTestInfra::get().await;

        // Create a fresh pool for this test
        let db_pool = PgPool::connect(&infra.db_url)
            .await
            .context("Failed to connect to test database")?;

        // Build ServerDeps from test dependencies
        let server_deps = deps.clone().into_server_deps(db_pool.clone());
        let server_deps_arc = Arc::new(server_deps.clone());

        // Build QueueEngine with all domain effects (seesaw 0.8.2)
        let seesaw_store = PostgresStore::new(db_pool.clone());
        let queue_engine = Engine::new(server_deps, seesaw_store)
            .with_effect(auth_effect())
            .with_effect(member_effect())
            .with_effect(agent_effect())
            .with_effect(website_effect())
            .with_effect(mark_no_listings_effect())
            .with_effect(crawling_pipeline_effect())
            .with_effect(post_composite_effect())
            .with_effect(website_approval_effect())
            .with_effect(provider_effect());
        let queue_engine = Arc::new(queue_engine);

        // Start seesaw runtime workers
        let _runtime = Runtime::start(&queue_engine, RuntimeConfig::default());

        // Give engine time to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        Ok(Self {
            server_deps: server_deps_arc,
            db_pool,
            deps,
            queue_engine,
        })
    }

    /// Get a GraphQL client for this harness.
    pub fn graphql(&self) -> GraphQLClient {
        GraphQLClient::new(
            self.db_pool.clone(),
            self.server_deps.clone(),
            self.queue_engine.clone(),
        )
    }

    /// Get a GraphQL client with an authenticated user.
    pub fn graphql_with_auth(&self, user_id: uuid::Uuid, is_admin: bool) -> GraphQLClient {
        GraphQLClient::with_auth_user(
            self.db_pool.clone(),
            self.server_deps.clone(),
            self.queue_engine.clone(),
            user_id,
            is_admin,
        )
    }

    /// Emit an event through the queue engine.
    pub async fn emit<
        E: Clone + Send + Sync + serde::Serialize + serde::de::DeserializeOwned + 'static,
    >(
        &self,
        event: E,
    ) {
        let _ = self.queue_engine.process(event).await;
    }

    /// Wait for effects to settle after an action.
    ///
    /// Effects are executed by the seesaw QueueEngine workers.
    /// This method yields to allow the event-driven pipeline to complete.
    pub async fn settle(&self) {
        // Allow time for the seesaw event pipeline to process:
        // 1. EventWorkers poll and process events
        // 2. EffectWorkers execute queued effects
        for _ in 0..10 {
            tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
            tokio::task::yield_now().await;
        }
    }

    /// Wait for a condition to become true, with retries.
    ///
    /// This is more robust than `settle()` for cases where you need to wait
    /// for a specific state change. It polls the condition every 25ms for
    /// up to 500ms total.
    pub async fn wait_for<F, Fut>(&self, condition: F) -> bool
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = bool>,
    {
        for _ in 0..20 {
            if condition().await {
                return true;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(25)).await;
            tokio::task::yield_now().await;
        }
        false
    }
}

impl AsyncTestContext for TestHarness {
    async fn setup() -> Self {
        Self::new().await.expect("Failed to create test harness")
    }

    async fn teardown(self) {
        self.db_pool.close().await;
    }
}
