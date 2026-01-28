use anyhow::Result;
use async_trait::async_trait;
use seesaw::{Effect, EffectContext};
use tracing::{debug, error, info};

use super::commands::AuthCommand;
use super::events::AuthEvent;
use super::models::{hash_phone_number, Identifier};
use crate::domains::organization::effects::ServerDeps;

/// Auth effect - handles OTP sending and verification
pub struct AuthEffect;

#[async_trait]
impl Effect<AuthCommand, ServerDeps> for AuthEffect {
    type Event = AuthEvent;

    async fn execute(&self, cmd: AuthCommand, ctx: EffectContext<ServerDeps>) -> Result<AuthEvent> {
        match cmd {
            AuthCommand::SendOTP { phone_number } => {
                debug!("Sending OTP to {}", phone_number);

                // 1. Check if phone is registered
                let phone_hash = hash_phone_number(&phone_number);
                let identifier =
                    Identifier::find_by_phone_hash(&phone_hash, &ctx.deps().db_pool).await?;

                if identifier.is_none() {
                    info!("Phone number not registered: {}", phone_number);
                    return Ok(AuthEvent::PhoneNotRegistered { phone_number });
                }

                // 2. Send OTP via Twilio
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
                debug!("Verifying OTP for {}", phone_number);

                // 1. Verify OTP with Twilio
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
                                    anyhow::anyhow!("Phone not found after verification")
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
