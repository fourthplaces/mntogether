//! Test harness with testcontainers for integration testing.
//!
//! Uses shared containers across all tests for dramatically improved performance.
//! Containers and migrations are initialized once on first test, then reused.

use anyhow::{Context, Result};
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

/// Test harness that manages test infrastructure.
///
/// Uses shared containers across all tests for fast test execution.
/// Each test gets a fresh context, but reuses the same database and Redis containers.
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
    /// Database pool - use this for test fixtures.
    pub db_pool: PgPool,
    /// Firecrawl API key (can be mocked in tests)
    pub firecrawl_api_key: String,
    /// OpenAI API key (can be mocked in tests)
    pub openai_api_key: String,
}

#[async_trait::async_trait]
impl AsyncTestContext for TestHarness {
    async fn setup() -> Self {
        Self::new()
            .await
            .expect("Failed to create test harness")
    }

    async fn teardown(self) {
        // Database pool is automatically dropped
    }
}

impl TestHarness {
    /// Creates a new test harness using shared containers.
    ///
    /// This will:
    /// 1. Get or initialize shared PostgreSQL and Redis containers
    /// 2. Run database migrations (only on first call)
    /// 3. Create a fresh database connection pool
    pub async fn new() -> Result<Self> {
        // Get shared infrastructure (containers start + migrations run on first call only)
        let infra = SharedTestInfra::get().await;

        // Create a fresh pool for this test
        let db_pool = PgPool::connect(&infra.db_url)
            .await
            .context("Failed to connect to test database")?;

        Ok(Self {
            db_pool,
            firecrawl_api_key: "test-firecrawl-key".to_string(),
            openai_api_key: "test-openai-key".to_string(),
        })
    }

    /// Get a GraphQL client for this harness.
    pub fn graphql(&self) -> GraphQLClient {
        GraphQLClient::new(
            self.db_pool.clone(),
            self.firecrawl_api_key.clone(),
            self.openai_api_key.clone(),
        )
    }

    /// Wait for effects to settle (for event-driven systems).
    ///
    /// In our simple MVP, this is not needed yet, but we include it
    /// for future compatibility when we add event-driven features.
    pub async fn settle(&self) {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
