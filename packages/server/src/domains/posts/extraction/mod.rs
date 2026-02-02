//! Post extraction submodule
//!
//! Handles AI extraction of posts from crawled page content.
//! This submodule listens to CrawlEvent::PagesReadyForExtraction from the crawling domain.

pub mod commands;
pub mod effects;
pub mod events;
pub mod machines;

pub use commands::PostExtractionCommand;
pub use effects::PostExtractionEffect;
pub use events::PostExtractionEvent;
pub use machines::PostExtractionMachine;
