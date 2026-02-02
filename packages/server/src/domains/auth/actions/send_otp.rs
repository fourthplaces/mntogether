use anyhow::Result;
use seesaw_core::EffectContext;
use tracing::{error, info};

use crate::domains::auth::events::AuthEvent;
use crate::domains::auth::models::{hash_phone_number, is_admin_identifier, Identifier};
use crate::domains::chatrooms::ChatRequestState;
use crate::domains::member::models::Member;
use crate::domains::posts::effects::ServerDeps;

/// Send OTP to phone number or email via Twilio.
/// Returns OTPSent on success, PhoneNotRegistered if identifier not found.
pub async fn send_otp(
    phone_number: String,
    ctx: &EffectContext<ServerDeps, ChatRequestState>,
) -> Result<AuthEvent> {
    // Production safety check - test identifier should never be enabled in production
    if ctx.deps().test_identifier_enabled && !cfg!(debug_assertions) {
        error!("SECURITY WARNING: TEST_IDENTIFIER_ENABLED is true in production build!");
    }

    // 1. Check if identifier is registered
    let phone_hash = hash_phone_number(&phone_number);
    let identifier = Identifier::find_by_phone_hash(&phone_hash, &ctx.deps().db_pool).await?;

    // 2. Auto-create identifier for admin emails if not registered
    if identifier.is_none() {
        let is_admin = is_admin_identifier(&phone_number, &ctx.deps().admin_identifiers);

        if is_admin {
            info!("Auto-creating admin member and identifier for: {}", phone_number);

            // Create member record first (required for foreign key)
            let member = Member {
                id: uuid::Uuid::new_v4(),
                expo_push_token: format!("admin:{}", phone_number),
                searchable_text: format!("Admin: {}", phone_number),
                latitude: None,
                longitude: None,
                location_name: None,
                active: true,
                notification_count_this_week: 0,
                paused_until: None,
                created_at: chrono::Utc::now(),
            };

            let member = member.insert(&ctx.deps().db_pool).await.map_err(|e| {
                error!("Failed to create admin member: {}", e);
                anyhow::anyhow!("Failed to create admin member: {}", e)
            })?;

            Identifier::create(member.id, phone_hash.clone(), true, &ctx.deps().db_pool)
                .await
                .map_err(|e| {
                    error!("Failed to create admin identifier: {}", e);
                    anyhow::anyhow!("Failed to create admin identifier: {}", e)
                })?;

            info!("Admin member and identifier created successfully for {}", phone_number);
        } else {
            info!("Identifier not registered: {}", phone_number);
            return Ok(AuthEvent::PhoneNotRegistered { phone_number });
        }
    }

    // 3. TEST IDENTIFIER BYPASS: Skip actual OTP sending for test identifiers
    #[cfg(debug_assertions)]
    if ctx.deps().test_identifier_enabled
        && (phone_number == "+1234567890" || phone_number == "test@example.com")
    {
        info!("Test identifier: Skipping actual OTP send for {}", phone_number);
        return Ok(AuthEvent::OTPSent { phone_number });
    }

    // 4. Send OTP via Twilio (supports phone numbers and emails)
    ctx.deps()
        .twilio
        .send_otp(&phone_number)
        .await
        .map_err(|e| {
            error!("Failed to send OTP: {}", e);
            anyhow::anyhow!("Failed to send OTP: {}", e)
        })?;

    info!("OTP sent successfully to {}", phone_number);
    Ok(AuthEvent::OTPSent { phone_number })
}
