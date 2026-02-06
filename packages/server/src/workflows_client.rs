//! Restate workflow client
//!
//! Simple HTTP client for invoking Restate workflows.

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;

/// Client for invoking Restate workflows via HTTP
#[derive(Clone)]
pub struct WorkflowClient {
    base_url: String,
    http_client: Arc<reqwest::Client>,
}

impl WorkflowClient {
    /// Create a new workflow client
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http_client: Arc::new(reqwest::Client::new()),
        }
    }

    /// Invoke a workflow service
    ///
    /// # Arguments
    /// * `service_name` - Name of the workflow service (e.g., "CrawlWebsite")
    /// * `handler_name` - Name of the handler method (e.g., "run")
    /// * `request` - Request payload
    pub async fn invoke<Req, Res>(
        &self,
        service_name: &str,
        handler_name: &str,
        request: Req,
    ) -> Result<Res>
    where
        Req: Serialize,
        Res: DeserializeOwned,
    {
        let url = format!("{}/{}/{}", self.base_url, service_name, handler_name);

        tracing::debug!(
            service = service_name,
            handler = handler_name,
            url = %url,
            "Invoking Restate workflow"
        );

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send workflow request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "unknown error".to_string());
            anyhow::bail!("Workflow invocation failed ({}): {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to deserialize workflow response")
    }

    /// Start a workflow without waiting for completion (fire-and-forget)
    pub async fn start_workflow<Req>(
        &self,
        service_name: &str,
        handler_name: &str,
        request: Req,
    ) -> Result<String>
    where
        Req: Serialize,
    {
        let url = format!("{}/{}/{}/send", self.base_url, service_name, handler_name);

        tracing::debug!(
            service = service_name,
            handler = handler_name,
            url = %url,
            "Starting Restate workflow (async)"
        );

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to start workflow")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "unknown error".to_string());
            anyhow::bail!("Failed to start workflow ({}): {}", status, body);
        }

        // Return invocation ID if available
        Ok("workflow_started".to_string())
    }
}
