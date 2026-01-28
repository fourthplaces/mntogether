//! GraphQL client for integration testing.
//!
//! Executes GraphQL queries directly against the schema without HTTP overhead.

use juniper::Variables;
use serde_json::Value;
use server_core::kernel::ServerKernel;
use server_core::server::graphql::{create_schema, GraphQLContext, Schema};
use std::sync::Arc;

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
    /// Creates a new GraphQL client with the given kernel.
    pub fn new(kernel: Arc<ServerKernel>) -> Self {
        let context = GraphQLContext::new(kernel);

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
