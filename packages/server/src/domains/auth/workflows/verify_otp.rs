//! Verify OTP workflow

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};

use crate::domains::auth::activities;
use crate::domains::auth::types::OtpVerified;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyOtpRequest {
    pub phone_number: String,
    pub code: String,
}

impl_restate_serde!(VerifyOtpRequest);

#[restate_sdk::workflow]
pub trait VerifyOtpWorkflow {
    async fn run(request: VerifyOtpRequest) -> Result<OtpVerified, HandlerError>;
}

pub struct VerifyOtpWorkflowImpl {
    pub deps: ServerDeps,
}

impl VerifyOtpWorkflowImpl {
    pub fn new(deps: ServerDeps) -> Self {
        Self { deps }
    }
}

impl VerifyOtpWorkflow for VerifyOtpWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        request: VerifyOtpRequest,
    ) -> Result<OtpVerified, HandlerError> {
        tracing::info!(phone_number = %request.phone_number, "Verifying OTP");

        // Durable execution - will not retry on replay
        let result = ctx
            .run(|| async {
                activities::verify_otp(
                    request.phone_number.clone(),
                    request.code.clone(),
                    &self.deps,
                )
                .await
                .map_err(Into::into)
            })
            .await?;

        Ok(result)
    }
}
