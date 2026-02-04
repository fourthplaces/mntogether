//! Member domain actions - business logic functions
//!
//! Actions are async functions called directly from GraphQL mutations via `process()`.
//! They do the work, emit fact events, and return `ReadResult<T>` for deferred reads.
//!
//! Handlers (which emit events) live in effects, not here.

mod generate_embedding;
mod queries;
mod register_member;
mod update_status;

pub use generate_embedding::{generate_embedding, EmbeddingResult};
pub use queries::get_members_paginated;
pub use register_member::register_member;
pub use update_status::update_member_status;
