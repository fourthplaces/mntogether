use seesaw::Machine;

use super::commands::AuthCommand;
use super::events::AuthEvent;

/// Auth machine - converts request events into commands
pub struct AuthMachine;

impl AuthMachine {
    pub fn new() -> Self {
        Self
    }
}

impl Machine for AuthMachine {
    type Event = AuthEvent;
    type Command = AuthCommand;

    fn decide(&mut self, event: &AuthEvent) -> Option<AuthCommand> {
        match event {
            // Convert request events to commands
            AuthEvent::SendOTPRequested { phone_number } => Some(AuthCommand::SendOTP {
                phone_number: phone_number.clone(),
            }),
            AuthEvent::VerifyOTPRequested { phone_number, code } => Some(AuthCommand::VerifyOTP {
                phone_number: phone_number.clone(),
                code: code.clone(),
            }),
            // Fact events don't produce commands
            AuthEvent::OTPSent { .. }
            | AuthEvent::OTPVerified { .. }
            | AuthEvent::OTPFailed { .. }
            | AuthEvent::PhoneNotRegistered { .. } => None,
        }
    }
}

impl Default for AuthMachine {
    fn default() -> Self {
        Self::new()
    }
}
