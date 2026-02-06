pub mod actions;
pub mod data;
pub mod effects;
pub mod events;
pub mod loader;
pub mod models;

// Re-export data types (GraphQL types)
pub use crate::domains::tag::TagData;
pub use data::post::{PostData, ServicePostData};
pub use data::types::{
    BusinessInfo, ContactInfoGraphQL, ContactInfoInput, EditPostInput, PostConnection,
    PostStatusData, PostType, ScrapeJobResult, SubmitPostInput, SubmitResourceLinkInput,
    SubmitResourceLinkResult,
};

// Re-export events
pub use events::PostEvent;

// Re-export models (domain models)
pub use models::post::Post;

