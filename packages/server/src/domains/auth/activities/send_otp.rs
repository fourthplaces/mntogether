//! Send OTP action

use anyhow::Result;
use tracing::{error, info};

use crate::domains::auth::events::AuthEvent;
use crate::domains::auth::models::{
    hash_phone_number, is_admin_identifier, is_test_identifier, Identifier,
};
use crate::kernel::ServerDeps;

/// Error returned when identifier is not authorized
#[derive(Debug, thiserror::Error)]
#[error("Not authorized")]
pub struct NotAuthorizedError;

/// Send OTP to phone number or email via Twilio.
///
/// Authorization: identifier must exist, or be an admin/test identifier.
/// Test identifiers skip actual Twilio send.
///
/// Returns `AuthEvent::OTPSent` on success.
pub async fn send_otp(phone_number: String, deps: &ServerDeps) -> Result<AuthEvent> {
    let phone_hash = hash_phone_number(&phone_number);
    let identifier_exists = Identifier::find_by_phone_hash(&phone_hash, &deps.db_pool)
        .await?
        .is_some();

    let is_admin = is_admin_identifier(&phone_number, &deps.admin_identifiers);
    let is_test = deps.test_identifier_enabled && is_test_identifier(&phone_number);

    // Must be registered, admin, or test identifier
    if !identifier_exists && !is_admin && !is_test {
        info!("Identifier not authorized: {}", phone_number);
        return Err(NotAuthorizedError.into());
    }

    // Test identifiers skip actual OTP send
    if is_test {
        info!("Test identifier: skipping OTP send for {}", phone_number);
        return Ok(AuthEvent::OTPSent {
            phone_number: phone_number.clone(),
        });
    }

    // Send OTP via Twilio
    deps.twilio.send_otp(&phone_number).await.map_err(|e| {
        error!("Failed to send OTP: {}", e);
        anyhow::anyhow!("Failed to send OTP: {}", e)
    })?;

    info!("OTP sent to {}", phone_number);
    Ok(AuthEvent::OTPSent {
        phone_number: phone_number.clone(),
    })
}
