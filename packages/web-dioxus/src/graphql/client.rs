//! GraphQL client for making requests to the API server

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::OnceLock;

static API_URL: OnceLock<String> = OnceLock::new();

/// Initialize the API URL. Call this at startup.
pub fn init_api_url(url: String) {
    API_URL.set(url).ok();
}

/// Get the configured API URL
pub fn get_api_url() -> &'static str {
    API_URL.get().map(|s| s.as_str()).unwrap_or("/graphql")
}

/// GraphQL request body
#[derive(Debug, Serialize)]
pub struct GraphQLRequest<V: Serialize> {
    pub query: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<V>,
}

/// GraphQL response wrapper
#[derive(Debug, Deserialize)]
pub struct GraphQLResponse<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<GraphQLError>>,
}

/// GraphQL error
#[derive(Debug, Deserialize)]
pub struct GraphQLError {
    pub message: String,
    pub locations: Option<Vec<GraphQLErrorLocation>>,
    pub path: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLErrorLocation {
    pub line: i32,
    pub column: i32,
}

/// Error type for GraphQL operations
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("GraphQL error: {0}")]
    GraphQL(String),

    #[error("No data returned")]
    NoData,

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// GraphQL client for making requests
#[derive(Clone)]
pub struct GraphQLClient {
    client: reqwest::Client,
    endpoint: String,
    auth_token: Option<String>,
}

impl GraphQLClient {
    /// Create a new GraphQL client
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            endpoint: endpoint.into(),
            auth_token: None,
        }
    }

    /// Create a client with authentication token
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    /// Execute a GraphQL query
    pub async fn query<V, R>(&self, query: &'static str, variables: Option<V>) -> Result<R, ClientError>
    where
        V: Serialize,
        R: DeserializeOwned,
    {
        let request = GraphQLRequest { query, variables };

        let mut req = self.client.post(&self.endpoint).json(&request);

        if let Some(token) = &self.auth_token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let response = req.send().await?;
        let graphql_response: GraphQLResponse<R> = response.json().await?;

        if let Some(errors) = graphql_response.errors {
            if let Some(first_error) = errors.first() {
                return Err(ClientError::GraphQL(first_error.message.clone()));
            }
        }

        graphql_response.data.ok_or(ClientError::NoData)
    }

    /// Execute a GraphQL mutation (same as query, but semantically different)
    pub async fn mutate<V, R>(&self, mutation: &'static str, variables: Option<V>) -> Result<R, ClientError>
    where
        V: Serialize,
        R: DeserializeOwned,
    {
        self.query(mutation, variables).await
    }
}

/// Create a client for server-side requests (direct to API)
#[cfg(feature = "server")]
pub fn server_client() -> GraphQLClient {
    let url = std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8080/graphql".to_string());
    GraphQLClient::new(url)
}

/// Create a client for client-side requests
#[cfg(feature = "web")]
pub fn browser_client() -> GraphQLClient {
    // In browser, use relative URL or configured endpoint
    let url = get_api_url();
    GraphQLClient::new(url)
}
