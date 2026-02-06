//! Auth domain actions - business logic functions
//!
//! Actions return events directly. GraphQL mutations call actions via `process()`
//! and the returned event is dispatched through the engine.

mod send_otp;
mod verify_otp;

pub use send_otp::{send_otp, NotAuthorizedError};
pub use verify_otp::{verify_otp, VerificationFailedError};
