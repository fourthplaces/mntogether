//! Interactive menu system with fuzzy search, status indicators, and quick keys
//!
//! This module provides a redesigned interactive CLI experience with:
//! - Workflow-based organization (5 groups instead of 9 categories)
//! - Live status indicators showing Docker, Git, and migration state
//! - Fuzzy search with `/` key
//! - Quick key shortcuts (s/x/l/t/q)
//! - Pinned favorites

mod preferences;
mod render;
mod runner;
mod search;
mod state;
mod types;

pub use runner::run_interactive;
