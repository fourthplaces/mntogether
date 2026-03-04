use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::api::error::ApiResult;
use crate::api::state::AppState;
use crate::domains::auth::activities;
use crate::domains::auth::types::{OtpSent, OtpVerified};

// --- Request types ---

#[derive(Debug, Deserialize)]
pub struct SendOtpRequest {
    pub phone_number: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyOtpRequest {
    pub phone_number: String,
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    #[allow(dead_code)]
    pub session_token: Option<String>,
}

// --- Response types ---

#[derive(Debug, Serialize)]
pub struct LogoutResult {
    pub success: bool,
}

// --- Handlers ---

async fn send_otp(
    State(state): State<AppState>,
    Json(req): Json<SendOtpRequest>,
) -> ApiResult<Json<OtpSent>> {
    tracing::info!(phone_number = %req.phone_number, "Auth/send_otp");
    let result = activities::send_otp(req.phone_number, &state.deps).await?;
    Ok(Json(result))
}

async fn verify_otp(
    State(state): State<AppState>,
    Json(req): Json<VerifyOtpRequest>,
) -> ApiResult<Json<OtpVerified>> {
    tracing::info!(phone_number = %req.phone_number, "Auth/verify_otp");
    let result = activities::verify_otp(req.phone_number, req.code, &state.deps).await?;
    Ok(Json(result))
}

async fn logout(Json(_req): Json<LogoutRequest>) -> ApiResult<Json<LogoutResult>> {
    Ok(Json(LogoutResult { success: true }))
}

// --- Router ---

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/Auth/send_otp", post(send_otp))
        .route("/Auth/verify_otp", post(verify_otp))
        .route("/Auth/logout", post(logout))
}
