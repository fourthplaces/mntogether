pub mod data;
pub mod loader;
pub mod models;

// Re-export commonly used types
pub use data::TagData;
pub use models::{Tag, TagKind, Taggable, TaggableType, TaxonomyCrosswalk};
