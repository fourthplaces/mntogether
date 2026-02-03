//! Organization domain actions - business logic functions
//!
//! Actions are async functions called directly from GraphQL mutations via `process()`.
//! They do the work and return final data types.

mod queries;

pub use queries::get_organizations_paginated;
