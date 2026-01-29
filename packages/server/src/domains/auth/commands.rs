use seesaw_core::{Command, ExecutionMode};

/// Auth commands - actions to perform
#[derive(Debug, Clone)]
pub enum AuthCommand {
    SendOTP { phone_number: String },
    VerifyOTP { phone_number: String, code: String },
}

impl Command for AuthCommand {
    fn execution_mode(&self) -> ExecutionMode {
        // Auth operations are synchronous (Twilio API calls)
        ExecutionMode::Inline
    }
}
