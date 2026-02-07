//! Send OTP workflow

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};

use crate::domains::auth::activities;
use crate::domains::auth::types::OtpSent;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendOtpRequest {
    pub phone_number: String,
}

impl_restate_serde!(SendOtpRequest);

#[restate_sdk::workflow]
pub trait SendOtpWorkflow {
    async fn run(request: SendOtpRequest) -> Result<OtpSent, HandlerError>;
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
        request: SendOtpRequest,
    ) -> Result<OtpSent, HandlerError> {
        tracing::info!(phone_number = %request.phone_number, "Sending OTP");

        // Durable execution - will not retry on replay
        let result = ctx
            .run(|| async {
                activities::send_otp(request.phone_number.clone(), &self.deps)
                    .await
                    .map_err(Into::into)
            })
            .await?;

        Ok(result)
    }
}
