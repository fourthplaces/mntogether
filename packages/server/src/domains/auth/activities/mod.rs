//! Auth domain actions - business logic functions
//!
//! Actions return events directly. Restate workflows call actions via activities
//! and orchestrate the execution flow.

mod send_otp;
mod verify_otp;

pub use send_otp::{send_otp, NotAuthorizedError};
pub use verify_otp::{verify_otp, VerificationFailedError};
