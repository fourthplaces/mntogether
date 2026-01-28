// Organization domain - handles need discovery from websites
//
// Responsibilities:
// - Scraping organization websites (via Firecrawl)
// - AI extraction of volunteer needs (via rig.rs + GPT-4o)
// - Content hash-based deduplication
// - Human-in-the-loop approval workflow
// - Sync tracking (new/changed/disappeared needs)

pub mod commands;
pub mod data;
pub mod edges;
pub mod effects;
pub mod events;
pub mod machines;
pub mod models;

pub use edges::*;
pub use models::*;
