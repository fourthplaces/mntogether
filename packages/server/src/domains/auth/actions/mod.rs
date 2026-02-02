//! Auth domain actions - business logic functions
//!
//! Actions are async functions called directly from GraphQL mutations via `process()`.

mod send_otp;
mod verify_otp;

pub use send_otp::{send_otp, SendOtpResult};
pub use verify_otp::{verify_otp, VerifyOtpResult};
