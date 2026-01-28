// HTTP server setup (Axum + GraphQL)
pub mod app;
// pub mod auth; // REMOVED: Migrated to domains/auth (Phase 4)
pub mod graphql;
pub mod middleware;
pub mod routes;
pub mod static_files;

pub use app::*;
pub use graphql::*;
