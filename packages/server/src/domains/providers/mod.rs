//! Providers domain - service provider directory
//!
//! Architecture (seesaw 0.6.0 direct-call pattern):
//!   GraphQL → process(action) → emit(FactEvent) → Effect watches facts → calls handlers
//!
//! Components:
//! - actions: Entry-point business logic called directly from GraphQL via process()
//! - models: Database models
//! - data: GraphQL data types

pub mod actions;
pub mod data;
pub mod models;

// Re-export commonly used types
pub use data::{ProviderData, ProviderStatusData, SubmitProviderInput, UpdateProviderInput};
pub use models::{CreateProvider, Provider, ProviderStatus, UpdateProvider};
