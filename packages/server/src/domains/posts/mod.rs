pub mod actions;
pub mod data;
pub mod effects;
pub mod events;
pub mod models;

// Re-export data types (GraphQL types)
pub use data::post::{PostData, ServicePostData, TagData};
pub use data::types::{
    BusinessInfo, ContactInfo, ContactInfoInput, EditPostInput, PostConnection, PostStatusData,
    PostType, ScrapeJobResult, SubmitPostInput, SubmitResourceLinkInput, SubmitResourceLinkResult,
};

// Re-export events
pub use events::PostEvent;

// Re-export models (domain models)
pub use models::post::Post;

// Re-export effects
pub use effects::post_composite_effect;
