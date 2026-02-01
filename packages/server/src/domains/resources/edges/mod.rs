//! GraphQL edge resolvers for resources

pub mod mutation;
pub mod query;

pub use mutation::{
    approve_resource, delete_resource, edit_and_approve_resource, edit_resource,
    generate_missing_embeddings, reject_resource, GenerateEmbeddingsResult,
};
pub use query::*;
