//! Providers domain - service provider directory
//!
//! Architecture (seesaw 0.6.0 direct-call pattern):
//!   GraphQL → process(action) → emit(FactEvent) → Effect watches facts → calls handlers
//!
//! Components:
//! - actions: Entry-point business logic called directly from GraphQL via process()
//! - effects: Cascade handlers for event-driven cleanup
//! - events: Fact events emitted by actions
//! - models: Database models
//! - data: GraphQL data types
//!
//! Cascade flow:
//!   ProviderDeleted → cleanup contacts and tags

pub mod activities;
pub mod data;
pub mod effects;
pub mod events;
pub mod models;

// Re-export commonly used types
pub use data::{ProviderData, ProviderStatusData, SubmitProviderInput, UpdateProviderInput};
pub use events::ProviderEvent;
pub use models::{CreateProvider, Provider, ProviderStatus, UpdateProvider};
