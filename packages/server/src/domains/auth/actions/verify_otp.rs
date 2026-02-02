//! Verify OTP action

use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::{error, info};
use uuid::Uuid;

use crate::common::AppState;
use crate::domains::auth::events::AuthEvent;
use crate::domains::auth::models::{hash_phone_number, is_admin_identifier, Identifier};
use crate::domains::member::models::Member;
use crate::kernel::ServerDeps;

/// Result of verifying OTP
pub enum VerifyOtpResult {
    Verified {
        member_id: Uuid,
        is_admin: bool,
    },
    Failed {
        reason: String,
    },
}

/// Verify OTP code.
///
/// Called directly from GraphQL mutation via `process()`.
/// Emits `OTPVerified` or `OTPFailed` fact event.
/// Returns member info needed for JWT creation.
pub async fn verify_otp(
    phone_number: String,
    code: String,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<VerifyOtpResult> {
    // TEST IDENTIFIER BYPASS: Only available in debug builds (development)
    #[cfg(debug_assertions)]
    if ctx.deps().test_identifier_enabled
        && code == "123456"
        && (phone_number == "+1234567890" || phone_number == "test@example.com")
    {
        info!("Test identifier bypass activated for {}", phone_number);

        // Get or create member info for test identifier
        let phone_hash = hash_phone_number(&phone_number);
        let identifier = match Identifier::find_by_phone_hash(&phone_hash, &ctx.deps().db_pool)
            .await?
        {
            Some(id) => id,
            None => {
                info!(
                    "Auto-creating test member and identifier: {}",
                    phone_number
                );
                let is_admin = is_admin_identifier(&phone_number, &ctx.deps().admin_identifiers);

                // Create member record first
                let member = Member {
                    id: uuid::Uuid::new_v4(),
                    expo_push_token: format!("test:{}", phone_number),
                    searchable_text: format!("Test: {}", phone_number),
                    latitude: None,
                    longitude: None,
                    location_name: None,
                    active: true,
                    notification_count_this_week: 0,
                    paused_until: None,
                    created_at: chrono::Utc::now(),
                };

                let member = member
                    .insert(&ctx.deps().db_pool)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to create test member: {}", e))?;

                Identifier::create(member.id, phone_hash, is_admin, &ctx.deps().db_pool)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to create test identifier: {}", e))?
            }
        };

        ctx.emit(AuthEvent::OTPVerified {
            member_id: identifier.member_id,
            phone_number: phone_number.clone(),
            is_admin: identifier.is_admin,
        });
        return Ok(VerifyOtpResult::Verified {
            member_id: identifier.member_id,
            is_admin: identifier.is_admin,
        });
    }

    // Fail fast in production if test auth is attempted
    #[cfg(not(debug_assertions))]
    if ctx.deps().test_identifier_enabled {
        panic!(
            "SECURITY: TEST_IDENTIFIER_ENABLED=true detected in production build! \
             Test authentication bypass must never be enabled in release builds."
        );
    }

    // 1. Verify OTP with Twilio (supports phone numbers and emails)
    match ctx.deps().twilio.verify_otp(&phone_number, &code).await {
        Ok(_) => {
            // 2. Get member info
            let phone_hash = hash_phone_number(&phone_number);
            let identifier = Identifier::find_by_phone_hash(&phone_hash, &ctx.deps().db_pool)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Identifier not found after verification"))?;

            info!(
                "OTP verified successfully for member {}",
                identifier.member_id
            );
            ctx.emit(AuthEvent::OTPVerified {
                member_id: identifier.member_id,
                phone_number: phone_number.clone(),
                is_admin: identifier.is_admin,
            });
            Ok(VerifyOtpResult::Verified {
                member_id: identifier.member_id,
                is_admin: identifier.is_admin,
            })
        }
        Err(e) => {
            error!("OTP verification failed: {}", e);
            let reason = e.to_string();
            ctx.emit(AuthEvent::OTPFailed {
                phone_number: phone_number.clone(),
                reason: reason.clone(),
            });
            Ok(VerifyOtpResult::Failed { reason })
        }
    }
}
