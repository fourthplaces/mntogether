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

pub struct SendOtpWorkflow {
    pub deps: ServerDeps,
}

// #[restate_sdk::service(name = "SendOtp")]
impl SendOtpWorkflow {
    pub fn new(deps: ServerDeps) -> Self {
        Self { deps }
    }

    async fn run(
        &self,
        ctx: Context,
        request: Json<SendOtpRequest>,
    ) -> Result<Json<SendOtpResult>, HandlerError> {
        let request = request.into_inner();
        tracing::info!(phone_number = %request.phone_number, "Sending OTP");

        let _event = ctx
            .run("send_otp", || async {
                activities::send_otp(request.phone_number.clone(), &self.deps).await
            })
            .await
            .map_err(|e| HandlerError::new(format!("Send OTP failed: {}", e)))?;

        Ok(Json(SendOtpResult {
            success: true,
            phone_number: request.phone_number,
        }))
    }
}
