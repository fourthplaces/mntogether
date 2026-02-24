// Emergency Resource Aggregator - API Core
//
// This crate provides the backend API for matching volunteers with organization needs.
// Architecture follows domain-driven design with durable execution via Restate.
//
// Restate services, objects, and workflows are organized per-domain in domains/*/restate/

pub mod common;
pub mod config;
pub mod data_migrations;
pub mod domains;
pub mod kernel;

pub use config::*;
