//! Verify OTP action

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::{error, info};
use uuid::Uuid;

use crate::common::AppState;
use crate::domains::auth::events::AuthEvent;
use crate::domains::auth::models::{
    hash_phone_number, is_admin_identifier, is_test_identifier, Identifier,
};
use crate::domains::member::models::Member;
use crate::kernel::ServerDeps;

/// Result of verifying OTP
pub enum VerifyOtpResult {
    Verified { member_id: Uuid, is_admin: bool },
    Failed { reason: String },
}

/// Verify OTP code and create member if needed.
///
/// Test identifiers skip Twilio verification.
/// Creates member + identifier on first successful verification.
pub async fn verify_otp(
    phone_number: String,
    code: String,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<VerifyOtpResult> {
    let is_test = ctx.deps().test_identifier_enabled && is_test_identifier(&phone_number);

    // Verify with Twilio (unless test identifier)
    if !is_test {
        if let Err(e) = ctx.deps().twilio.verify_otp(&phone_number, &code).await {
            error!("OTP verification failed: {}", e);
            return Ok(VerifyOtpResult::Failed {
                reason: e.to_string(),
            });
        }
    } else {
        info!("Test identifier: skipping Twilio verification for {}", phone_number);
    }

    // Find or create member
    let phone_hash = hash_phone_number(&phone_number);
    let (member_id, is_admin) =
        match Identifier::find_by_phone_hash(&phone_hash, &ctx.deps().db_pool).await? {
            Some(identifier) => (identifier.member_id, identifier.is_admin),
            None => {
                let is_admin = is_admin_identifier(&phone_number, &ctx.deps().admin_identifiers);
                let member_id = create_member(&phone_number, &ctx.deps().db_pool).await?;
                Identifier::create(member_id, phone_hash, is_admin, &ctx.deps().db_pool).await?;
                info!("Created new member {} for {}", member_id, phone_number);
                (member_id, is_admin)
            }
        };

    info!("OTP verified for member {}", member_id);
    ctx.emit(AuthEvent::OTPVerified {
        member_id,
        phone_number: phone_number.clone(),
        is_admin,
    });

    Ok(VerifyOtpResult::Verified { member_id, is_admin })
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
