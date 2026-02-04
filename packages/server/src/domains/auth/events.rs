use uuid::Uuid;

/// Auth events - facts about authentication state changes
///
/// NOTE: Failed/error events have been removed (OTPFailed, PhoneNotRegistered).
/// Errors go in Result::Err, not in events. Events are for successful state changes.
#[derive(Debug, Clone)]
pub enum AuthEvent {
    /// OTP was sent successfully
    OTPSent { phone_number: String },

    /// OTP was verified successfully
    OTPVerified {
        member_id: Uuid,
        phone_number: String,
        is_admin: bool,
    },
}
