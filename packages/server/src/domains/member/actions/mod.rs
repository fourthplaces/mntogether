//! Member domain actions - business logic functions
//!
//! Actions contain all business logic and can be called from:
//! - Effects (handling request events)
//! - Other contexts as needed
//!
//! Each action takes typed parameters and returns a MemberEvent (fact event).

mod generate_embedding;
mod register_member;
mod update_status;

pub use generate_embedding::generate_embedding;
pub use register_member::register_member;
pub use update_status::update_member_status;
