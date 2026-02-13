//! Durable workflow for confirming a newsletter subscription.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::common::EmptyRequest;
use crate::domains::newsletter::activities::confirm::confirm_newsletter;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmNewsletterRequest {
    pub source_id: Uuid,
}

impl_restate_serde!(ConfirmNewsletterRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmNewsletterResult {
    pub source_id: Uuid,
    pub status: String,
}

impl_restate_serde!(ConfirmNewsletterResult);

#[restate_sdk::workflow]
#[name = "ConfirmNewsletterWorkflow"]
pub trait ConfirmNewsletterWorkflow {
    async fn run(req: ConfirmNewsletterRequest) -> Result<ConfirmNewsletterResult, HandlerError>;

    #[shared]
    async fn get_status(req: EmptyRequest) -> Result<String, HandlerError>;
}

pub struct ConfirmNewsletterWorkflowImpl {
    deps: Arc<ServerDeps>,
}

impl ConfirmNewsletterWorkflowImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl ConfirmNewsletterWorkflow for ConfirmNewsletterWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        req: ConfirmNewsletterRequest,
    ) -> Result<ConfirmNewsletterResult, HandlerError> {
        info!(source_id = %req.source_id, "Starting newsletter confirmation workflow");

        ctx.set("status", "Confirming subscription...".to_string());

        // ctx.run requires Restate-serializable return types — use a status string
        let status: String = ctx
            .run(|| async {
                let result = confirm_newsletter(req.source_id, &self.deps)
                    .await
                    .map_err(|e| restate_sdk::errors::TerminalError::new(e.to_string()))?;
                Ok(result.status)
            })
            .await?;

        let msg = format!("Confirmation {}", status);
        ctx.set("status", msg);

        info!(
            source_id = %req.source_id,
            status = %status,
            "Newsletter confirmation workflow completed"
        );

        Ok(ConfirmNewsletterResult {
            source_id: req.source_id,
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
