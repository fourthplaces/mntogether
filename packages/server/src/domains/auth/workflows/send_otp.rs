//! Send OTP workflow

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};

use crate::domains::auth::activities;
use crate::kernel::ServerDeps;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendOtpRequest {
    pub phone_number: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendOtpResult {
    pub success: bool,
    pub phone_number: String,
}

#[restate_sdk::workflow]
pub trait SendOtpWorkflow {
    async fn run(request: Json<SendOtpRequest>) -> Result<Json<SendOtpResult>, HandlerError>;
}

pub struct SendOtpWorkflowImpl {
    pub deps: ServerDeps,
}

impl SendOtpWorkflowImpl {
    pub fn new(deps: ServerDeps) -> Self {
        Self { deps }
    }
}

impl SendOtpWorkflow for SendOtpWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        request: Json<SendOtpRequest>,
    ) -> Result<Json<SendOtpResult>, HandlerError> {
        let request = request.into_inner();
        tracing::info!(phone_number = %request.phone_number, "Sending OTP");

        let _event = ctx
            .run(|| async {
                activities::send_otp(request.phone_number.clone(), &self.deps)
                    .await
                    .map_err(Into::into)
            })
            .await
            .map_err(|e| anyhow::anyhow!("Send OTP failed: {}", e))?;

        Ok(Json(SendOtpResult {
            success: true,
            phone_number: request.phone_number,
        }))
    }
}
