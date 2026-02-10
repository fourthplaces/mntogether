//! Source domain - unified content sources (websites, social profiles)

pub mod activities;
pub mod data;
pub mod models;
pub mod restate;

pub use data::SourceData;
pub use models::{
    create_social_source, create_website_source, find_or_create_social_source,
    find_or_create_website_source, get_source_identifier, SocialSource, Source, SourceStatus,
    WebsiteSource,
};
