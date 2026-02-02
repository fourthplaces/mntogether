use anyhow::Result;
use async_trait::async_trait;
use seesaw_core::{Effect, EffectContext};

use super::actions;
use super::events::AuthEvent;
use crate::domains::chatrooms::ChatRequestState;
use crate::domains::posts::effects::ServerDeps;

/// Auth effect - thin dispatcher to actions
pub struct AuthEffect;

#[async_trait]
impl Effect<AuthEvent, ServerDeps, ChatRequestState> for AuthEffect {
    type Event = AuthEvent;

    async fn handle(
        &mut self,
        event: AuthEvent,
        ctx: EffectContext<ServerDeps, ChatRequestState>,
    ) -> Result<Option<AuthEvent>> {
        match event {
            AuthEvent::SendOTPRequested { phone_number } => {
                actions::send_otp(phone_number, &ctx).await.map(Some)
            }
            AuthEvent::VerifyOTPRequested { phone_number, code } => {
                actions::verify_otp(phone_number, code, &ctx).await.map(Some)
            }
            // Fact events - terminal, no follow-up event needed
            AuthEvent::OTPSent { .. }
            | AuthEvent::OTPVerified { .. }
            | AuthEvent::OTPFailed { .. }
            | AuthEvent::PhoneNotRegistered { .. } => Ok(None),
        }
    }
}
