pub mod data;
pub mod models;

// Re-export commonly used types
pub use data::TagData;
pub use models::{Tag, Taggable, TagKind, TaggableType};
