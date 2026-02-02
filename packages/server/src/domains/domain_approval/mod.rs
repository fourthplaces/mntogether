//! Domain Approval - website assessment workflow for approval decisions
//!
//! Architecture (seesaw 0.3.0):
//!   Request Event → Effect → Fact Event → Internal Edge → Request Event → ...
//!
//! Components:
//! - events: Request events (user intent) and fact events (what happened)
//! - effects: Thin dispatcher that routes request events to handlers
//! - edges/internal: React to fact events, emit new request events
//! - edges/mutation: GraphQL mutations that emit request events
//! - edges/query: GraphQL queries (read-only)

pub mod actions;
pub mod data;
pub mod edges;
pub mod effects;
pub mod events;

pub use data::*;
pub use edges::{mutation::*, query::*};
pub use effects::DomainApprovalEffect;
pub use events::DomainApprovalEvent;
