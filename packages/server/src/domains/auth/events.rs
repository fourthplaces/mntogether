use uuid::Uuid;

/// Auth events - facts about authentication state changes
#[derive(Debug, Clone)]
pub enum AuthEvent {
    // Fact events (emitted by actions)
    OTPSent {
        phone_number: String,
    },
    OTPVerified {
        member_id: Uuid,
        phone_number: String,
        is_admin: bool,
    },
    OTPFailed {
        phone_number: String,
        reason: String,
    },
    PhoneNotRegistered {
        phone_number: String,
    },
}
