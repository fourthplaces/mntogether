//! Auth domain - handles authentication via OTP (phone number)
//!
//! Architecture (Restate workflows):
//!   API → Restate workflow → workflow orchestrates activities
//!
//! Responsibilities:
//! - Phone-based OTP authentication via Twilio
//! - Session/JWT token management
//! - Phone number hashing for privacy

pub mod activities;
pub mod jwt;
pub mod models;
pub mod types;
pub mod restate;

pub use jwt::{Claims, JwtService};
pub use types::{OtpSent, OtpVerified};
pub use restate::*;
