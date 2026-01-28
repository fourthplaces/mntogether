// Auth domain - handles authentication via OTP (phone number)
//
// Responsibilities:
// - Phone-based OTP authentication via Twilio
// - Session/JWT token management
// - Phone number hashing for privacy

pub mod commands;
pub mod edges;
pub mod effects;
pub mod events;
pub mod jwt;
pub mod machines;
pub mod models;

pub use commands::AuthCommand;
pub use effects::AuthEffect;
pub use events::AuthEvent;
pub use jwt::{Claims, JwtService};
pub use machines::AuthMachine;
