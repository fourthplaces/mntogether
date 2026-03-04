// Root Editorial — API Core
//
// This crate provides the backend API server (Axum HTTP/JSON).
// Architecture follows domain-driven design: domains/*/models/ for persistence,
// domains/*/activities/ for business logic, api/routes/ for HTTP handlers.

pub mod api;
pub mod common;
pub mod config;
pub mod data_migrations;
pub mod domains;
pub mod kernel;

pub use config::*;
