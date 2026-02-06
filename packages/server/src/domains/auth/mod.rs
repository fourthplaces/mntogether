//! Auth domain - handles authentication via OTP (phone number)
//!
//! Architecture (Restate workflows):
//!   GraphQL → workflow_client.invoke(Workflow) → workflow orchestrates activities
//!
//! Responsibilities:
//! - Phone-based OTP authentication via Twilio
//! - Session/JWT token management
//! - Phone number hashing for privacy

pub mod activities;
pub mod events; // Still used by activities (TODO: Remove after activities refactored)
pub mod jwt;
pub mod models;
pub mod workflows;

pub use events::AuthEvent; // Still exported for compatibility
pub use jwt::{Claims, JwtService};
pub use workflows::*;
