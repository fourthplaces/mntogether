//! MN Together - Dioxus Fullstack Web Application
//!
//! This is a fullstack SSR web application built with Dioxus.
//! It connects to the existing GraphQL API for data.
//!
//! ## Running
//!
//! Development (with hot reload):
//! ```bash
//! dx serve --features web,server
//! ```
//!
//! Production build:
//! ```bash
//! dx build --release --features web,server
//! ```

#![allow(non_snake_case)]

mod app;
mod auth;
mod components;
mod graphql;
mod pages;
mod routes;
mod state;
mod types;

use dioxus::prelude::*;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Launch the Dioxus app
    // In fullstack mode, this handles both server and client
    dioxus::launch(app::App);
}
