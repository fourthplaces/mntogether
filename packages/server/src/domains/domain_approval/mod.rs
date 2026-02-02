//! Domain Approval - website assessment workflow for approval decisions
//!
//! Architecture (seesaw 0.6.0 direct-call pattern):
//!   GraphQL → process(action) → emit(FactEvent) → Effect watches facts → calls handlers
//!
//! Components:
//! - actions: Entry-point business logic called directly from GraphQL via process()
//! - effects: Event handlers that respond to fact events

pub mod actions;
pub mod data;
pub mod effects;
pub mod events;

pub use data::*;
pub use effects::domain_approval_effect;
pub use events::DomainApprovalEvent;
