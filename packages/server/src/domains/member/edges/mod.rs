//! Member domain edges
//!
//! Edges are the entry points to the event-driven system.
//! - External edges: GraphQL mutations/queries that emit request events
//! - Internal edges: React to fact events and emit new request events

pub mod internal;
pub mod mutation;
pub mod query;

pub use internal::*;
pub use mutation::*;
pub use query::*;
