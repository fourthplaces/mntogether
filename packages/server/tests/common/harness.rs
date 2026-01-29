//! Test harness with testcontainers for integration testing.
//!
//! Uses shared containers across all tests for dramatically improved performance.
//! Containers and migrations are initialized once on first test, then reused.

use anyhow::{Context, Result};
use seesaw_core::{EngineBuilder, EngineHandle, EventBus};
use server_core::domains::matching::{
    commands::MatchingCommand, effects::MatchingEffect, machines::MatchingMachine,
};
use server_core::domains::member::{
    commands::MemberCommand, effects::RegistrationEffect, machines::MemberMachine,
};
use server_core::domains::organization::{
    commands::OrganizationCommand,
    effects::{AIEffect, NeedEffect, ScraperEffect, ServerDeps, SyncEffect},
    machines::OrganizationMachine,
};
use server_core::kernel::{ServerKernel, TestDependencies};
use sqlx::PgPool;
use std::sync::Arc;
use test_context::AsyncTestContext;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::redis::Redis;
use tokio::sync::OnceCell;

use super::GraphQLClient;

/// Shared test infrastructure that persists across all tests.
/// Containers are started once and reused, migrations run once.
struct SharedTestInfra {
    db_url: String,
    redis_url: String,
    // Keep containers alive for the entire test run
    _postgres: ContainerAsync<Postgres>,
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
        let postgres = Postgres::default()
            .with_tag("16")
            .with_cmd(["-c", "max_connections=200"])
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

fn start_engine(handles: &mut Vec<EngineHandle>, engine: seesaw_core::Engine<ServerDeps>) {
    handles.push(engine.start());
}

fn start_domain_engines(deps: &ServerDeps, bus: &EventBus) -> Vec<EngineHandle> {
    let mut handles = Vec::new();

    // Organization domain
    start_engine(
        &mut handles,
        EngineBuilder::new(deps.clone())
            .with_machine(OrganizationMachine::new())
            .with_effect::<OrganizationCommand, _>(ScraperEffect)
            .with_effect::<OrganizationCommand, _>(AIEffect)
            .with_effect::<OrganizationCommand, _>(SyncEffect)
            .with_effect::<OrganizationCommand, _>(NeedEffect)
            .with_bus(bus.clone())
            .build(),
    );

    // Member domain
    start_engine(
        &mut handles,
        EngineBuilder::new(deps.clone())
            .with_machine(MemberMachine::new())
            .with_effect::<MemberCommand, _>(RegistrationEffect)
            .with_bus(bus.clone())
            .build(),
    );

    // Matching domain
    start_engine(
        &mut handles,
        EngineBuilder::new(deps.clone())
            .with_machine(MatchingMachine::new())
            .with_effect::<MatchingCommand, _>(MatchingEffect)
            .with_bus(bus.clone())
            .build(),
    );

    handles
}

/// Test harness that manages test infrastructure.
///
/// Uses shared containers across all tests for fast test execution.
/// Each test gets a fresh kernel and service instance, but reuses
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
    pub kernel: Arc<ServerKernel>,
    /// Database pool - use this for test fixtures.
    /// This is the same pool used by the kernel.
    pub db_pool: PgPool,
    /// Test dependencies for accessing mocks
    pub deps: TestDependencies,
    /// Handles for domain engines (kept alive to process events).
    _engine_handles: Vec<EngineHandle>,
}

impl TestHarness {
    /// Creates a new test harness using shared containers.
    ///
    /// This will:
    /// 1. Get or initialize shared PostgreSQL and Redis containers
    /// 2. Run database migrations (only on first call)
    /// 3. Initialize a fresh ServerKernel with test dependencies
    /// 4. Register all domain processors
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
    /// use server_core::kernel::{MockNeedExtractor, TestDependencies};
    ///
    /// let deps = TestDependencies::new()
    ///     .mock_extractor(
    ///         MockNeedExtractor::new()
    ///             .with_single_need("Volunteers Needed", "Help us!")
    ///     );
    /// let harness = TestHarness::with_deps(deps).await?;
    /// ```
    pub async fn with_deps(deps: TestDependencies) -> Result<Self> {
        // Get shared infrastructure (containers start + migrations run on first call only)
        let infra = SharedTestInfra::get().await;

        // Create a fresh pool for this test
        let db_pool = PgPool::connect(&infra.db_url)
            .await
            .context("Failed to connect to test database")?;

        // Create kernel with test dependencies
        let kernel = deps.clone().into_kernel(db_pool.clone());

        // Create ServerDeps for engines (from kernel dependencies)
        let server_deps = ServerDeps::new(
            kernel.db_pool.clone(),
            kernel.web_scraper.clone(),
            kernel.need_extractor.clone(),
            kernel.embedding_service.clone(),
            kernel.push_service.clone(),
        );

        // Start domain engines (same as production)
        let bus = kernel.bus.clone();
        let engine_handles = start_domain_engines(&server_deps, &bus);

        // Give engines time to subscribe to the event bus
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        Ok(Self {
            kernel,
            db_pool,
            deps,
            _engine_handles: engine_handles,
        })
    }

    /// Get a GraphQL client for this harness.
    pub fn graphql(&self) -> GraphQLClient {
        GraphQLClient::new(self.kernel.clone())
    }

    /// Get the event bus for this harness.
    /// Use this to call edge functions that require an EventBus.
    pub fn bus(&self) -> EventBus {
        self.kernel.bus.clone()
    }

    /// Wait for effects to settle after an action.
    ///
    /// Effects are executed by the seesaw Dispatcher via domain engines.
    /// Machines observe events and emit commands, which are then dispatched
    /// to their corresponding effects. This method yields to allow the
    /// event-driven pipeline to complete.
    pub async fn settle(&self) {
        // Allow time for the seesaw event pipeline to process:
        // 1. EventBus delivers events to subscribed engines
        // 2. Machines observe events and emit commands
        // 3. Dispatcher routes commands to effects (inline or background)
        // 4. Effects execute and emit fact events
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

impl Drop for TestHarness {
    fn drop(&mut self) {
        // Abort all engine handles to stop their background tasks.
        for handle in &self._engine_handles {
            handle.abort();
        }
    }
}

#[async_trait::async_trait]
impl AsyncTestContext for TestHarness {
    async fn setup() -> Self {
        Self::new().await.expect("Failed to create test harness")
    }

    async fn teardown(self) {
        // Engine handles are aborted in Drop
        self.db_pool.close().await;
    }
}
