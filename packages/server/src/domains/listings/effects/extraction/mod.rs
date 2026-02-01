//! Two-pass extraction module for creating listings from websites
//!
//! Pass 1 (summarize): Extract key information from each page
//! Pass 2 (synthesize): Combine all summaries into complete listings

pub mod summarize;
pub mod synthesize;
pub mod types;

pub use summarize::*;
pub use synthesize::*;
pub use types::*;
