use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};
use tracing::{debug, error, info};

use super::commands::AuthCommand;
use super::events::AuthEvent;
use super::models::{hash_phone_number, Identifier};
use crate::domains::listings::effects::ServerDeps;

/// Auth effect - handles OTP sending and verification
///
/// Supports both phone numbers and email addresses as identifiers.
/// Twilio can send OTP codes to either.
pub struct AuthEffect;

#[async_trait]
impl Effect<AuthCommand, ServerDeps> for AuthEffect {
    type Event = AuthEvent;

    async fn execute(&self, cmd: AuthCommand, ctx: EffectContext<ServerDeps>) -> Result<AuthEvent> {
        // Production safety check - test identifier should never be enabled in production
        if ctx.deps().test_identifier_enabled && !cfg!(debug_assertions) {
            error!("⚠️  SECURITY WARNING: TEST_IDENTIFIER_ENABLED is true in production build! This is a security risk.");
        }

        match cmd {
            AuthCommand::SendOTP { phone_number } => {
                // Note: phone_number field can be either phone or email (Twilio supports both)
                debug!("Sending OTP to identifier: {}", phone_number);

                // 1. Check if identifier is registered
                let phone_hash = hash_phone_number(&phone_number);
                let mut identifier =
                    Identifier::find_by_phone_hash(&phone_hash, &ctx.deps().db_pool).await?;

                // 2. Auto-create identifier for admin emails if not registered
                if identifier.is_none() {
                    use super::models::is_admin_identifier;
                    use crate::domains::member::models::Member;
                    use uuid::Uuid;

                    let is_admin =
                        is_admin_identifier(&phone_number, &ctx.deps().admin_identifiers);

                    if is_admin {
                        info!(
                            "Auto-creating admin member and identifier for: {}",
                            phone_number
                        );

                        // Create member record first (required for foreign key)
                        let member = Member {
                            id: Uuid::new_v4(),
                            expo_push_token: format!("admin:{}", phone_number), // Placeholder
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

                        // Create identifier record
                        identifier = Some(
                            Identifier::create(
                                member.id,
                                phone_hash.clone(),
                                true,
                                &ctx.deps().db_pool,
                            )
                            .await
                            .map_err(|e| {
                                error!("Failed to create admin identifier: {}", e);
                                anyhow::anyhow!("Failed to create admin identifier: {}", e)
                            })?,
                        );

                        info!(
                            "Admin member and identifier created successfully for {}",
                            phone_number
                        );
                    } else {
                        info!("Identifier not registered: {}", phone_number);
                        return Ok(AuthEvent::PhoneNotRegistered { phone_number });
                    }
                }

                // 2. Send OTP via Twilio (supports phone numbers and emails)
                // TEST IDENTIFIER BYPASS: Skip actual OTP sending for test identifiers
                #[cfg(debug_assertions)]
                if ctx.deps().test_identifier_enabled
                    && (phone_number == "+1234567890" || phone_number == "test@example.com")
                {
                    info!(
                        "Test identifier: Skipping actual OTP send for {}",
                        phone_number
                    );
                    return Ok(AuthEvent::OTPSent { phone_number });
                }

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

            AuthCommand::VerifyOTP { phone_number, code } => {
                // Note: phone_number field can be either phone or email (Twilio supports both)
                debug!("Verifying OTP for identifier: {}", phone_number);

                // TEST IDENTIFIER BYPASS: Only available in debug builds (development)
                // In release builds, this code is completely removed at compile time
                #[cfg(debug_assertions)]
                if ctx.deps().test_identifier_enabled
                    && code == "123456"
                    && (phone_number == "+1234567890" || phone_number == "test@example.com")
                {
                    info!("Test identifier bypass activated for {}", phone_number);

                    // Get or create member info for test identifier
                    let phone_hash = hash_phone_number(&phone_number);
                    let identifier =
                        match Identifier::find_by_phone_hash(&phone_hash, &ctx.deps().db_pool)
                            .await?
                        {
                            Some(id) => id,
                            None => {
                                use super::models::is_admin_identifier;
                                use crate::domains::member::models::Member;
                                use uuid::Uuid;

                                info!("Auto-creating test member and identifier: {}", phone_number);
                                let is_admin = is_admin_identifier(
                                    &phone_number,
                                    &ctx.deps().admin_identifiers,
                                );

                                // Create member record first
                                let member = Member {
                                    id: Uuid::new_v4(),
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

                                let member =
                                    member.insert(&ctx.deps().db_pool).await.map_err(|e| {
                                        anyhow::anyhow!("Failed to create test member: {}", e)
                                    })?;

                                Identifier::create(
                                    member.id,
                                    phone_hash,
                                    is_admin,
                                    &ctx.deps().db_pool,
                                )
                                .await
                                .map_err(|e| {
                                    anyhow::anyhow!("Failed to create test identifier: {}", e)
                                })?
                            }
                        };

                    return Ok(AuthEvent::OTPVerified {
                        member_id: identifier.member_id,
                        phone_number,
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
                match ctx
                    .deps()
                    .twilio
                    .verify_otp(&phone_number, &code)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
                {
                    Ok(_) => {
                        // 2. Get member info
                        let phone_hash = hash_phone_number(&phone_number);
                        let identifier =
                            Identifier::find_by_phone_hash(&phone_hash, &ctx.deps().db_pool)
                                .await?
                                .ok_or_else(|| {
                                    anyhow::anyhow!("Identifier not found after verification")
                                })?;

                        info!(
                            "OTP verified successfully for member {}",
                            identifier.member_id
                        );
                        Ok(AuthEvent::OTPVerified {
                            member_id: identifier.member_id,
                            phone_number,
                            is_admin: identifier.is_admin,
                        })
                    }
                    Err(e) => {
                        error!("OTP verification failed: {}", e);
                        Ok(AuthEvent::OTPFailed {
                            phone_number,
                            reason: e.to_string(),
                        })
                    }
                }
            }
        }
    }
}
