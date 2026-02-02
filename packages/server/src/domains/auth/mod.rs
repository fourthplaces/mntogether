//! Auth domain - handles authentication via OTP (phone number)
//!
//! Architecture (seesaw 0.6.0 direct-call pattern):
//!   GraphQL → process(action) → emit(FactEvent) → Effect watches facts → calls handlers
//!
//! Responsibilities:
//! - Phone-based OTP authentication via Twilio
//! - Session/JWT token management
//! - Phone number hashing for privacy

pub mod actions;
pub mod effects;
pub mod events;
pub mod jwt;
pub mod models;

pub use effects::auth_effect;
pub use events::AuthEvent;
pub use jwt::{Claims, JwtService};
