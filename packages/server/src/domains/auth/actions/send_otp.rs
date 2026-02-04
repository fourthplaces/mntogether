//! Send OTP action

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::{error, info};

use crate::common::AppState;
use crate::domains::auth::events::AuthEvent;
use crate::domains::auth::models::{
    hash_phone_number, is_admin_identifier, is_test_identifier, Identifier,
};
use crate::kernel::ServerDeps;

/// Result of sending OTP
pub enum SendOtpResult {
    Sent,
    NotAuthorized,
}

/// Send OTP to phone number or email via Twilio.
///
/// Authorization: identifier must exist, or be an admin/test identifier.
/// Test identifiers skip actual Twilio send.
pub async fn send_otp(
    phone_number: String,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<SendOtpResult> {
    let phone_hash = hash_phone_number(&phone_number);
    let identifier_exists = Identifier::find_by_phone_hash(&phone_hash, &ctx.deps().db_pool)
        .await?
        .is_some();

    let is_admin = is_admin_identifier(&phone_number, &ctx.deps().admin_identifiers);
    let is_test = ctx.deps().test_identifier_enabled && is_test_identifier(&phone_number);

    // Must be registered, admin, or test identifier
    if !identifier_exists && !is_admin && !is_test {
        info!("Identifier not authorized: {}", phone_number);
        return Ok(SendOtpResult::NotAuthorized);
    }

    // Test identifiers skip actual OTP send
    if is_test {
        info!("Test identifier: skipping OTP send for {}", phone_number);
        ctx.emit(AuthEvent::OTPSent {
            phone_number: phone_number.clone(),
        });
        return Ok(SendOtpResult::Sent);
    }

    // Send OTP via Twilio
    ctx.deps()
        .twilio
        .send_otp(&phone_number)
        .await
        .map_err(|e| {
            error!("Failed to send OTP: {}", e);
            anyhow::anyhow!("Failed to send OTP: {}", e)
        })?;

    info!("OTP sent to {}", phone_number);
    ctx.emit(AuthEvent::OTPSent {
        phone_number: phone_number.clone(),
    });
    Ok(SendOtpResult::Sent)
}
