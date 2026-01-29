pub mod coordinator;
pub mod machines;
pub mod opportunity_adapter;

pub use coordinator::{CrawlerCoordinator, CoordinatorStats};
pub use machines::{PageLifecycleMachine, ResourceDiscoveryMachine};
pub use opportunity_adapter::{ExtractedOpportunity, OpportunityAdapter};
