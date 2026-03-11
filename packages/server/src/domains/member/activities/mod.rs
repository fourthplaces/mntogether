//! Member domain actions - business logic functions
//!
//! Actions return events directly. HTTP handlers call actions via activities
//! and orchestrate the execution flow.

mod queries;
mod register_member;
mod update_status;

pub use queries::get_members_paginated;
pub use register_member::register_member;
pub use update_status::update_member_status;
