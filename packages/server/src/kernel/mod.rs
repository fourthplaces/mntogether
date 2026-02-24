//! Kernel module - server infrastructure and dependencies.

pub mod ai_tools;
pub mod deps;
pub mod llm_request;
pub mod pii;
pub mod sse;
pub mod stream_hub;
pub mod tag;
pub mod test_dependencies;
pub mod traits;

// Re-export AI client types
pub use ai_client::openai::StructuredOutput;
pub use ai_client::Claude;
pub use ai_client::OpenAi;

/// GPT-5 Mini — cost-effective frontier model for extraction, dedup, sync, PII.
pub const GPT_5_MINI: &str = "gpt-5-mini";

/// Claude Sonnet 4.5 — best instruction-following for writer pass.
pub const CLAUDE_SONNET: &str = "claude-sonnet-4-5-20250929";

/// GPT-5 — full frontier model for highest-accuracy tasks.
pub const GPT_5: &str = "gpt-5";

// Other exports
pub use deps::{ServerDeps, TwilioAdapter};
pub use llm_request::CompletionExt;
pub use pii::{create_pii_detector, HybridPiiDetector, NoopPiiDetector, RegexPiiDetector};
pub use stream_hub::StreamHub;
pub use test_dependencies::TestDependencies;
pub use traits::*;

// AI Tools for agentic workflows
pub use ai_tools::SearchPostsTool;
