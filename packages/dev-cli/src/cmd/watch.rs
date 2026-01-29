//! Watch mode for development

use anyhow::{anyhow, Result};
use dialoguer::Select;
use std::path::PathBuf;

use crate::cmd_builder::CmdBuilder;
use crate::config::Config;
use crate::context::AppContext;
use crate::utils::cmd_exists;

/// Find the server package from config
fn find_server_package(config: &Config) -> Option<(String, PathBuf)> {
    config
        .packages
        .values()
        .find(|pkg| pkg.release_type.as_deref() == Some("server"))
        .map(|pkg| (pkg.name.clone(), pkg.path.clone()))
}

/// Find the expo/app package from config
fn find_app_package(config: &Config) -> Option<PathBuf> {
    config
        .packages
        .values()
        .find(|pkg| pkg.release_type.as_deref() == Some("expo"))
        .map(|pkg| pkg.path.clone())
}

/// Start watch mode for the API (auto-rebuild on changes)
pub fn watch_api(ctx: &AppContext, config: Option<&Config>) -> Result<()> {
    if !cmd_exists("cargo-watch") {
        return Err(anyhow!(
            "cargo-watch not found. Install with: cargo install cargo-watch"
        ));
    }

    // Get server package from config
    let (server_name, server_path) = match config {
        Some(cfg) => {
            find_server_package(cfg).ok_or_else(|| anyhow!("No server package found in config"))?
        }
        None => {
            let cfg = Config::load(&ctx.repo)?;
            find_server_package(&cfg).ok_or_else(|| anyhow!("No server package found in config"))?
        }
    };

    let server_rel = server_path.strip_prefix(&ctx.repo).unwrap_or(&server_path);
    let src_watch = format!("{}/src", server_rel.display());
    let cargo_watch = format!("{}/Cargo.toml", server_rel.display());
    let run_cmd = format!("run --package {}", server_name);

    ctx.print_header("Starting API in watch mode");
    println!("Watching for changes in {}...", server_rel.display());
    println!("Press Ctrl+C to stop.");
    println!();

    // Use cargo-watch to rebuild and restart on changes
    CmdBuilder::new("cargo")
        .args([
            "watch",
            "-w",
            &src_watch,
            "-w",
            &cargo_watch,
            "-x",
            &run_cmd,
        ])
        .cwd(&ctx.repo)
        .run()?;

    Ok(())
}

/// Start watch mode for the app (hot reload)
pub fn watch_app(ctx: &AppContext, config: Option<&Config>) -> Result<()> {
    ctx.print_header("Starting App in watch mode");
    println!("Starting Expo with hot reload...");
    println!("Press Ctrl+C to stop.");
    println!();

    let app_dir = match config {
        Some(cfg) => {
            find_app_package(cfg).ok_or_else(|| anyhow!("No expo/app package found in config"))?
        }
        None => {
            let cfg = Config::load(&ctx.repo)?;
            find_app_package(&cfg).ok_or_else(|| anyhow!("No expo/app package found in config"))?
        }
    };

    CmdBuilder::new("yarn")
        .args(["start"])
        .cwd(&app_dir)
        .run()?;

    Ok(())
}

/// Watch everything (API + containers)
pub fn watch_all(ctx: &AppContext, config: Option<&Config>) -> Result<()> {
    ctx.print_header("Starting full development watch mode");

    // Ensure containers are running
    println!("Ensuring containers are running...");
    super::docker::docker_compose_up(ctx, &[], false)?;
    println!();

    // Start API in watch mode
    watch_api(ctx, config)
}

/// Interactive watch menu
pub fn watch_menu(ctx: &AppContext, config: Option<&Config>) -> Result<()> {
    let items = vec![
        "API (Rust) - auto-rebuild on changes",
        "App (Expo) - hot reload",
        "Full stack - containers + API watch",
        "Back",
    ];

    let choice = Select::with_theme(&ctx.theme())
        .with_prompt("What do you want to watch?")
        .items(&items)
        .default(0)
        .interact()?;

    match choice {
        0 => watch_api(ctx, config),
        1 => watch_app(ctx, config),
        2 => watch_all(ctx, config),
        _ => Ok(()),
    }
}
