//! Verify OTP action

use anyhow::Result;
use tracing::{error, info};
use uuid::Uuid;

use crate::domains::auth::models::{
    hash_phone_number, is_admin_identifier, is_test_identifier, Identifier,
};
use crate::domains::auth::types::OtpVerified;
use crate::domains::member::models::Member;
use crate::kernel::ServerDeps;

/// Error returned when OTP verification fails
#[derive(Debug, thiserror::Error)]
#[error("Verification failed: {reason}")]
pub struct VerificationFailedError {
    pub reason: String,
}

/// Verify OTP code, create member if needed, and return OtpVerified.
///
/// Test identifiers skip Twilio verification.
/// Creates member + identifier on first successful verification.
///
/// Returns OtpVerified with token on success.
pub async fn verify_otp(
    phone_number: String,
    code: String,
    deps: &ServerDeps,
) -> Result<OtpVerified> {
    let is_test = deps.test_identifier_enabled && is_test_identifier(&phone_number);

    // Verify with Twilio (unless test identifier)
    if !is_test {
        if let Err(e) = deps.twilio.verify_otp(&phone_number, &code).await {
            error!("OTP verification failed: {}", e);
            return Err(VerificationFailedError {
                reason: e.to_string(),
            }
            .into());
        }
    } else {
        info!(
            "Test identifier: skipping Twilio verification for {}",
            phone_number
        );
    }

    // Find or create member
    let phone_hash = hash_phone_number(&phone_number);
    let (member_id, is_admin) =
        match Identifier::find_by_phone_hash(&phone_hash, &deps.db_pool).await? {
            Some(identifier) => (identifier.member_id, identifier.is_admin),
            None => {
                let is_admin = is_admin_identifier(&phone_number, &deps.admin_identifiers);
                let member_id = create_member(&phone_number, &deps.db_pool).await?;
                Identifier::create(member_id, phone_hash, is_admin, &deps.db_pool).await?;
                info!("Created new member {} for {}", member_id, phone_number);
                (member_id, is_admin)
            }
        };

    info!("OTP verified for member {}", member_id);

    // Create JWT token
    let token = deps
        .jwt_service
        .create_token(member_id, phone_number.clone(), is_admin)?;

    Ok(OtpVerified {
        member_id,
        phone_number,
        is_admin,
        token,
    })
}

/// Create a new member for the given identifier.
async fn create_member(phone_number: &str, pool: &sqlx::PgPool) -> Result<Uuid> {
    let member = Member {
        id: Uuid::new_v4(),
        expo_push_token: format!("pending:{}", phone_number),
        searchable_text: String::new(),
        latitude: None,
        longitude: None,
        location_name: None,
        active: true,
        notification_count_this_week: 0,
        paused_until: None,
        created_at: chrono::Utc::now(),
    };

    let member = member.insert(pool).await?;
    Ok(member.id)
}
