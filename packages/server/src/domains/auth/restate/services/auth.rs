//! Unified Auth service
//!
//! Combines send_otp, verify_otp, and logout into a single Restate service.
//! Replaces the separate SendOtp and VerifyOtp services.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::domains::auth::activities;
use crate::domains::auth::types::{OtpSent, OtpVerified};
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// --- Request types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendOtpRequest {
    pub phone_number: String,
}

impl_restate_serde!(SendOtpRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyOtpRequest {
    pub phone_number: String,
    pub code: String,
}

impl_restate_serde!(VerifyOtpRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutRequest {
    pub session_token: Option<String>,
}

impl_restate_serde!(LogoutRequest);

// --- Response types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutResult {
    pub success: bool,
}

impl_restate_serde!(LogoutResult);

// --- Service definition ---

#[restate_sdk::service]
#[name = "Auth"]
pub trait AuthService {
    async fn send_otp(request: SendOtpRequest) -> Result<OtpSent, HandlerError>;
    async fn verify_otp(request: VerifyOtpRequest) -> Result<OtpVerified, HandlerError>;
    async fn logout(request: LogoutRequest) -> Result<LogoutResult, HandlerError>;
}

pub struct AuthServiceImpl {
    deps: Arc<ServerDeps>,
}

impl AuthServiceImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}


impl AuthService for AuthServiceImpl {
    async fn send_otp(
        &self,
        ctx: Context<'_>,
        request: SendOtpRequest,
    ) -> Result<OtpSent, HandlerError> {
        tracing::info!(phone_number = %request.phone_number, "Auth/send_otp");

        let result = ctx
            .run(|| async {
                activities::send_otp(request.phone_number.clone(), &self.deps)
                    .await
                    .map_err(Into::into)
            })
            .await?;

        Ok(result)
    }

    async fn verify_otp(
        &self,
        ctx: Context<'_>,
        request: VerifyOtpRequest,
    ) -> Result<OtpVerified, HandlerError> {
        tracing::info!(phone_number = %request.phone_number, "Auth/verify_otp");

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

    async fn logout(
        &self,
        _ctx: Context<'_>,
        _request: LogoutRequest,
    ) -> Result<LogoutResult, HandlerError> {
        // JWT-based auth: logout is client-side (discard token).
        // No server-side session to invalidate.
        Ok(LogoutResult { success: true })
    }
}
