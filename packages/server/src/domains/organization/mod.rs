// Organization domain - handles organization and source data
//
// Responsibilities:
// - Organization model and data
// - Organization sources (website sources for scraping)
// - Source operations (add/remove scrape URLs)
//
// Note: Scraping, AI extraction, and approval workflow moved to listings domain

pub mod data;
pub mod edges;
pub mod effects;
pub mod models;
pub mod utils;

pub use edges::*;
pub use models::*;
