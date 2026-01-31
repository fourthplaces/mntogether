// Emergency Resource Aggregator - API Core
//
// This crate provides the backend API for matching volunteers with organization needs.
// Architecture follows domain-driven design with event sourcing via seesaw-rs.

pub mod common;
pub mod config;
pub mod data_migrations;
pub mod domains;
pub mod kernel;
pub mod server;

pub use config::*;
