//! Agent activities — AI response generation and curator pipeline.
//!
//! - responses: greeting/reply generation for assistant agents
//! - discover/extract/enrich/monitor: curator pipeline steps
//! - evaluate_filter: AI pre-filter for website candidates

mod responses;
pub mod discover;
pub mod enrich;
pub mod evaluate_filter;
pub mod extract;
pub mod monitor;

pub use responses::{
    generate_greeting, generate_reply, get_container_agent_config,
};
