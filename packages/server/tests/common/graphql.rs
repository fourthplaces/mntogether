//! GraphQL client for integration testing.
//!
//! Executes GraphQL queries directly against the schema without HTTP overhead.

use juniper::Variables;
use serde_json::Value;
use server_core::domains::auth::JwtService;
use server_core::kernel::test_dependencies::{MockEmbeddingService, MockPiiDetector, MockPushNotificationService};
use server_core::kernel::{OpenAIClient, ServerDeps, ServerKernel, TwilioAdapter};
use server_core::server::graphql::context::AppEngine;
use server_core::server::graphql::{create_schema, GraphQLContext, Schema};
use std::sync::Arc;
use twilio::{TwilioOptions, TwilioService};

/// GraphQL client for executing queries and mutations in tests.
pub struct GraphQLClient {
    schema: Schema,
    context: GraphQLContext,
}

/// Result of a GraphQL execution.
#[derive(Debug)]
pub struct GraphQLResult {
    pub data: Option<Value>,
    pub errors: Vec<String>,
}

impl GraphQLResult {
    /// Returns true if the execution had no errors.
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// Unwraps the data, panicking if there were errors.
    pub fn unwrap(self) -> Value {
        if !self.errors.is_empty() {
            panic!("GraphQL errors: {:?}", self.errors);
        }
        self.data.expect("No data returned")
    }

    /// Gets a value at the given JSON path.
    ///
    /// # Example
    /// ```ignore
    /// let name = result.get("need.title").as_str();
    /// ```
    pub fn get(&self, path: &str) -> Value {
        let data = self.data.as_ref().expect("No data returned");
        let mut current = data;
        for key in path.split('.') {
            current = &current[key];
        }
        current.clone()
    }
}

impl GraphQLClient {
    /// Creates a new GraphQL client with the given kernel and engine.
    pub fn new(kernel: Arc<ServerKernel>, engine: Arc<AppEngine>) -> Self {
        // Create test instances of services needed by GraphQLContext
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

        // Create minimal server_deps for testing (extraction is None)
        let server_deps = Arc::new(create_test_server_deps(kernel.clone()));

        let context = GraphQLContext::new(
            kernel.db_pool.clone(),
            engine,
            server_deps,
            None, // No auth user by default
            twilio,
            jwt_service,
            openai_client,
        );

        Self {
            schema: create_schema(),
            context,
        }
    }

    /// Creates a new GraphQL client with an authenticated user.
    pub fn with_auth_user(
        kernel: Arc<ServerKernel>,
        engine: Arc<AppEngine>,
        user_id: uuid::Uuid,
        is_admin: bool,
    ) -> Self {
        use server_core::server::middleware::AuthUser;

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

        let auth_user = AuthUser {
            user_id: user_id.to_string(),
            member_id: server_core::common::MemberId::from_uuid(user_id),
            phone_number: "+15555555555".to_string(),
            is_admin,
        };

        // Create minimal server_deps for testing (extraction is None)
        let server_deps = Arc::new(create_test_server_deps(kernel.clone()));

        let context = GraphQLContext::new(
            kernel.db_pool.clone(),
            engine,
            server_deps,
            Some(auth_user),
            twilio,
            jwt_service,
            openai_client,
        );

        Self {
            schema: create_schema(),
            context,
        }
    }

    /// Creates a new GraphQL client with a custom context (for auth testing).
    pub fn with_context(context: GraphQLContext) -> Self {
        Self {
            schema: create_schema(),
            context,
        }
    }

    /// Execute a GraphQL query/mutation.
    pub async fn execute(&self, query: &str) -> GraphQLResult {
        self.execute_with_vars(query, Variables::new()).await
    }

    /// Execute a GraphQL query/mutation with variables.
    pub async fn execute_with_vars(&self, query: &str, variables: Variables) -> GraphQLResult {
        let (result, errors) =
            juniper::execute(query, None, &self.schema, &variables, &self.context)
                .await
                .expect("GraphQL execution failed");

        // Convert juniper::Value to serde_json::Value
        let data = Some(serde_json::to_value(&result).expect("Failed to serialize GraphQL result"));

        let error_messages: Vec<String> = errors
            .iter()
            .map(|e| e.error().message().to_string())
            .collect();

        GraphQLResult {
            data,
            errors: error_messages,
        }
    }

    /// Execute a query and expect success, returning the data.
    pub async fn query(&self, query: &str) -> Value {
        self.execute(query).await.unwrap()
    }

    /// Execute a query with variables and expect success.
    pub async fn query_with_vars(&self, query: &str, variables: Variables) -> Value {
        self.execute_with_vars(query, variables).await.unwrap()
    }
}

/// Create test server deps with mock services
fn create_test_server_deps(kernel: Arc<ServerKernel>) -> ServerDeps {
    use extraction::{MockIngestor, MockWebSearcher};

    // Create mock services for testing
    let openai_client = Arc::new(OpenAIClient::new("test_api_key".to_string()));
    let twilio = Arc::new(TwilioService::new(TwilioOptions {
        account_sid: "test".to_string(),
        auth_token: "test".to_string(),
        service_id: "test".to_string(),
    }));

    use server_core::kernel::jobs::NoopJobQueue;
    use server_core::domains::auth::JwtService;

    // Create job queue and JWT service for tests
    let job_queue: Arc<dyn server_core::kernel::jobs::JobQueue> = Arc::new(NoopJobQueue::new());
    let jwt_service = Arc::new(JwtService::new("test_secret", "test_issuer".to_string()));

    ServerDeps::new(
        kernel.db_pool.clone(),
        Arc::new(MockIngestor::new()),
        openai_client.clone(),
        Arc::new(MockEmbeddingService::new()),
        Arc::new(MockPushNotificationService::new()),
        Arc::new(TwilioAdapter::new(twilio)),
        Arc::new(MockWebSearcher::new()),
        Arc::new(MockPiiDetector::new()),
        None, // No extraction service in tests
        job_queue,
        jwt_service,
        false,
        vec![],
    )
}
