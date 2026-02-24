//! Auth domain data types
//!
//! Simple, serializable types returned by auth activities.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::impl_restate_serde;

/// Result of sending an OTP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtpSent {
    pub phone_number: String,
    pub success: bool,
}

impl_restate_serde!(OtpSent);

/// Result of verifying an OTP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtpVerified {
    pub member_id: Uuid,
    pub phone_number: String,
    pub is_admin: bool,
    pub token: String,
}

impl_restate_serde!(OtpVerified);
