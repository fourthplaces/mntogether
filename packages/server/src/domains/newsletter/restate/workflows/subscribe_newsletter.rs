//! Durable workflow for subscribing to a newsletter.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::common::EmptyRequest;
use crate::domains::newsletter::activities::subscribe::subscribe_to_newsletter;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeNewsletterRequest {
    pub form_id: Uuid,
    pub organization_id: Option<Uuid>,
}

impl_restate_serde!(SubscribeNewsletterRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeNewsletterResult {
    pub source_id: Option<Uuid>,
    pub ingest_email: Option<String>,
    pub status: String,
}

impl_restate_serde!(SubscribeNewsletterResult);

#[restate_sdk::workflow]
#[name = "SubscribeNewsletterWorkflow"]
pub trait SubscribeNewsletterWorkflow {
    async fn run(
        req: SubscribeNewsletterRequest,
    ) -> Result<SubscribeNewsletterResult, HandlerError>;

    #[shared]
    async fn get_status(req: EmptyRequest) -> Result<String, HandlerError>;
}

pub struct SubscribeNewsletterWorkflowImpl {
    deps: Arc<ServerDeps>,
}

impl SubscribeNewsletterWorkflowImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl SubscribeNewsletterWorkflow for SubscribeNewsletterWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        req: SubscribeNewsletterRequest,
    ) -> Result<SubscribeNewsletterResult, HandlerError> {
        info!(form_id = %req.form_id, "Starting newsletter subscription workflow");

        ctx.set("status", "Subscribing to newsletter...".to_string());

        // ctx.run requires Restate-serializable return types — use JSON string
        let result_json: String = ctx
            .run(|| async {
                let result =
                    subscribe_to_newsletter(req.form_id, req.organization_id, &self.deps)
                        .await
                        .map_err(|e| {
                            restate_sdk::errors::TerminalError::new(e.to_string())
                        })?;

                let json = serde_json::json!({
                    "source_id": result.source_id.to_string(),
                    "ingest_email": result.ingest_email,
                    "status": result.status,
                })
                .to_string();
                Ok(json)
            })
            .await?;

        let parsed: serde_json::Value = serde_json::from_str(&result_json)
            .map_err(|e| HandlerError::from(restate_sdk::errors::TerminalError::new(e.to_string())))?;

        let status = parsed["status"].as_str().unwrap_or("unknown").to_string();
        let source_id = parsed["source_id"]
            .as_str()
            .and_then(|s| Uuid::parse_str(s).ok());
        let ingest_email = parsed["ingest_email"].as_str().map(|s| s.to_string());

        let msg = format!("Subscription {}", status);
        ctx.set("status", msg);

        info!(
            form_id = %req.form_id,
            status = %status,
            "Newsletter subscription workflow completed"
        );

        Ok(SubscribeNewsletterResult {
            source_id,
            ingest_email,
            status,
        })
    }

    async fn get_status(
        &self,
        ctx: SharedWorkflowContext<'_>,
        _req: EmptyRequest,
    ) -> Result<String, HandlerError> {
        Ok(ctx
            .get::<String>("status")
            .await?
            .unwrap_or_else(|| "pending".to_string()))
    }
}
