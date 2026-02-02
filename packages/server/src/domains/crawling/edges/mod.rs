//! Crawling domain edges
//!
//! Edges are the entry points to the event-driven system.
//! - Internal edges: React to fact events and emit new request events

pub mod internal;

pub use internal::*;
