//! Verify OTP workflow

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domains::auth::activities;
use crate::kernel::ServerDeps;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyOtpRequest {
    pub phone_number: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyOtpResult {
    pub member_id: Uuid,
    pub phone_number: String,
    pub is_admin: bool,
    pub token: String,
}

#[restate_sdk::workflow]
pub trait VerifyOtpWorkflow {
    async fn run(request: Json<VerifyOtpRequest>) -> Result<Json<VerifyOtpResult>, HandlerError>;
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
        request: Json<VerifyOtpRequest>,
    ) -> Result<Json<VerifyOtpResult>, HandlerError> {
        let request = request.into_inner();
        tracing::info!(phone_number = %request.phone_number, "Verifying OTP");

        let event = ctx
            .run(|| async {
                activities::verify_otp(request.phone_number.clone(), request.code.clone(), &self.deps)
                    .await
                    .map_err(Into::into)
            })
            .await
            .map_err(|e| anyhow::anyhow!("Verify OTP failed: {}", e))?;

        // Extract data from event
        use crate::domains::auth::events::AuthEvent;
        let AuthEvent::OTPVerified {
            member_id,
            phone_number,
            is_admin,
            token,
        } = event
        else {
            return Err(anyhow::anyhow!("Unexpected event type").into());
        };

        Ok(Json(VerifyOtpResult {
            member_id,
            phone_number,
            is_admin,
            token,
        }))
    }
}
