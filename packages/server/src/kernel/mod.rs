// Kernel - core infrastructure with dependency injection
//
// The ServerKernel holds all server dependencies (database, APIs, services)
// and provides dependency injection through traits for testability.
//
// IMPORTANT: Kernel is for INFRASTRUCTURE only, not business logic.
// Business logic belongs in domain layers.

pub mod ai;
pub mod scheduled_tasks;
pub mod server_kernel;
pub mod test_dependencies;
pub mod traits;

pub use ai::OpenAIClient;
pub use server_kernel::ServerKernel;
pub use test_dependencies::{
    MockAI, MockEmbeddingService, MockPushNotificationService, MockWebScraper, TestDependencies,
};
pub use traits::*;

// Re-export common types for convenience
// These are shared types used across layers (no circular dependency)
pub use crate::common::{ContactInfo, ExtractedNeed};
