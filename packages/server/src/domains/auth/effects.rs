use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};

use super::actions;
use super::events::AuthEvent;
use crate::domains::posts::effects::ServerDeps;

/// Auth effect - thin dispatcher to actions
pub struct AuthEffect;

#[async_trait]
impl Effect<AuthEvent, ServerDeps> for AuthEffect {
    type Event = AuthEvent;

    async fn handle(
        &mut self,
        event: AuthEvent,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<AuthEvent> {
        match event {
            AuthEvent::SendOTPRequested { phone_number } => {
                actions::send_otp(phone_number, &ctx).await
            }
            AuthEvent::VerifyOTPRequested { phone_number, code } => {
                actions::verify_otp(phone_number, code, &ctx).await
            }
            // Fact events - these shouldn't reach the effect in 0.3.0
            // but we need to handle them for exhaustive match
            AuthEvent::OTPSent { .. }
            | AuthEvent::OTPVerified { .. }
            | AuthEvent::OTPFailed { .. }
            | AuthEvent::PhoneNotRegistered { .. } => {
                unreachable!("Fact events should not be dispatched to effects")
            }
        }
    }
}
