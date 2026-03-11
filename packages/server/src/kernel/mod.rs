//! Kernel module - server infrastructure and dependencies.

pub mod deps;
pub mod pii;
pub mod sse;
pub mod storage;
pub mod stream_hub;
pub mod test_dependencies;
pub mod traits;

// Other exports
pub use deps::{ServerDeps, TwilioAdapter};
pub use pii::{create_pii_detector, NoopPiiDetector, RegexPiiDetector};
pub use stream_hub::StreamHub;
pub use test_dependencies::TestDependencies;
pub use traits::*;
