//! Verify OTP service
//!
//! Single-step durable service (not a workflow — no multi-step orchestration needed).

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

#[restate_sdk::service]
#[name = "VerifyOtp"]
pub trait VerifyOtpService {
    async fn run(request: VerifyOtpRequest) -> Result<OtpVerified, HandlerError>;
}

pub struct VerifyOtpServiceImpl {
    deps: std::sync::Arc<ServerDeps>,
}

impl VerifyOtpServiceImpl {
    pub fn with_deps(deps: std::sync::Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl VerifyOtpService for VerifyOtpServiceImpl {
    async fn run(
        &self,
        ctx: Context<'_>,
        request: VerifyOtpRequest,
    ) -> Result<OtpVerified, HandlerError> {
        tracing::info!(phone_number = %request.phone_number, "Verifying OTP");

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
