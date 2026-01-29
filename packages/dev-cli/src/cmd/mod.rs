//! Command implementations

pub mod ai;
pub mod ai_assistant;
pub mod ai_lint;
pub mod benchmark;
pub mod ci;
pub mod cmd;
pub mod coverage;
pub mod db;
pub mod deploy;
pub mod docker;
pub mod ecs;
pub mod env;
pub mod houston;
pub mod jobs;
pub mod local;
pub mod migrate;
pub mod mobile;
pub mod quality;
pub mod release;
pub mod status;
pub mod test;
pub mod todos;
pub mod tunnel;
pub mod watch;

use anyhow::Result;
use console::style;

use crate::config::Config;
use crate::context::AppContext;
use crate::utils::cmd_exists;
use crate::utils::open_in_browser;

/// Print doctor/health check information
pub fn print_doctor(ctx: &AppContext) {
    let checks: &[(&str, &str)] = &[
        ("git", "git"),
        ("docker", "docker"),
        ("docker compose", "docker"),
        ("cargo", "cargo"),
        ("rustc", "rustc"),
        ("sqlx", "sqlx"),
        ("cargo-watch", "cargo-watch"),
        ("cargo-llvm-cov", "cargo-llvm-cov"),
        ("npm", "npm"),
        ("yarn", "yarn"),
        ("ngrok", "ngrok"),
        ("esc", "esc"),
        ("ffmpeg", "ffmpeg"),
    ];

    ctx.print_header("Doctor");
    if !ctx.quiet {
        println!("Repo: {}", ctx.repo.display());
        println!();
    }

    for (label, bin) in checks {
        let ok = cmd_exists(bin);
        let status = if ok {
            style("OK").green()
        } else {
            style("MISSING").yellow()
        };
        println!("{:<16} {}", label, status);
    }

    if cfg!(target_os = "macos") {
        let ok = cmd_exists("xcodebuild");
        let status = if ok {
            style("OK").green()
        } else {
            style("MISSING").yellow()
        };
        println!("{:<16} {}", "xcodebuild", status);
    }
}

/// Open a URL by key from config
pub fn open_url(ctx: &AppContext, config: &Config, key: &str) -> Result<()> {
    let entry = config.global.urls.get(key).ok_or_else(|| {
        anyhow::anyhow!(
            "URL '{}' not found in config. Define it in [urls.{}]",
            key,
            key
        )
    })?;

    ctx.print_header(&format!("Opening {}: {}", entry.label, entry.url));
    open_in_browser(&entry.url)?;
    ctx.print_success(&format!("{} opened in browser.", entry.label));
    Ok(())
}

/// Interactive menu to select and open a URL from config
pub fn open_url_menu(ctx: &AppContext, config: &Config) -> Result<()> {
    use dialoguer::Select;

    if config.global.urls.is_empty() {
        println!("No URLs defined in config.");
        println!("Add URLs to .dev/config.toml:");
        println!();
        println!("  [urls.my-tool]");
        println!("  label = \"My Tool\"");
        println!("  url = \"http://localhost:8080\"");
        return Ok(());
    }

    // Collect entries sorted by key
    let mut entries: Vec<_> = config.global.urls.all().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));

    let labels: Vec<&str> = entries.iter().map(|(_, e)| e.label.as_str()).collect();

    let choice = Select::with_theme(&ctx.theme())
        .with_prompt("Select URL to open")
        .items(&labels)
        .default(0)
        .interact()?;

    let (key, _) = entries[choice];
    open_url(ctx, config, key)
}
