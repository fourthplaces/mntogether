//! Shared utility functions

use anyhow::{anyhow, Context, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use which::which;

use crate::cmd_builder::CmdBuilder;
use crate::context::AppContext;

/// Get the repository root path from REPO_ROOT env var or infer from CARGO_MANIFEST_DIR
pub fn repo_root() -> Result<PathBuf> {
    if let Ok(v) = env::var("REPO_ROOT") {
        let p = PathBuf::from(v);
        if p.exists() {
            return Ok(p);
        }
    }
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()));
    Ok(manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .context("failed to infer repo root from CARGO_MANIFEST_DIR")?
        .to_path_buf())
}

/// Check if a command exists in PATH
pub fn cmd_exists(name: &str) -> bool {
    which(name).is_ok()
}

/// Check if docker or docker-compose is available
pub fn docker_available() -> bool {
    cmd_exists("docker") || cmd_exists("docker-compose")
}

/// Ensure docker is available, returning an error if not
pub fn ensure_docker() -> Result<()> {
    if !docker_available() {
        return Err(anyhow!("Docker is required for this operation."));
    }
    Ok(())
}

/// Ensure cargo is available, returning an error if not
pub fn ensure_cargo() -> Result<()> {
    if !cmd_exists("cargo") {
        return Err(anyhow!("cargo not found (install Rust toolchain)"));
    }
    Ok(())
}

/// Ensure a cargo tool is installed, prompting to install if missing.
/// `tool_name` is the binary name to check (e.g., "cargo-watch", "sqlx").
/// The crate name is derived from the tool name, with special cases handled.
pub fn ensure_cargo_tool(
    ctx: &AppContext,
    tool_name: &str,
    description: &str,
    extra_args: &[&str],
) -> Result<()> {
    if cmd_exists(tool_name) {
        return Ok(());
    }

    ctx.print_warning(&format!("{} not found. {}", tool_name, description));

    // Derive crate name from tool name
    let crate_name = match tool_name {
        "sqlx" => "sqlx-cli",
        name => name.trim_start_matches("cargo-"),
    };

    if !ctx.confirm(
        &format!("Install {} via `cargo install` now?", crate_name),
        true,
    )? {
        return Err(anyhow!("{} is required", tool_name));
    }

    ensure_cargo()?;

    let mut cmd = CmdBuilder::new("cargo")
        .args(["install", crate_name, "--locked"])
        .cwd(&ctx.repo);

    for arg in extra_args {
        cmd = cmd.arg(*arg);
    }

    let code = cmd.run()?;

    if code != 0 {
        return Err(anyhow!(
            "cargo install {} exited with code {}",
            crate_name,
            code
        ));
    }
    Ok(())
}

/// Open a URL in the default browser
pub fn open_in_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(url)
            .spawn()
            .with_context(|| format!("failed to open {url} in browser"))?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .with_context(|| format!("failed to open {url} in browser"))?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", url])
            .spawn()
            .with_context(|| format!("failed to open {url} in browser"))?;
    }
    Ok(())
}

/// Get docker compose program and base args
pub fn docker_compose_program() -> Result<(String, Vec<String>)> {
    if cmd_exists("docker") {
        return Ok(("docker".to_string(), vec!["compose".to_string()]));
    }
    if cmd_exists("docker-compose") {
        return Ok(("docker-compose".to_string(), vec![]));
    }
    Err(anyhow!(
        "Docker Compose not found (`docker` or `docker-compose`)"
    ))
}

// =============================================================================
// Rust Package Discovery
// =============================================================================

/// Extract the package name from a Cargo.toml file
fn get_cargo_package_name(cargo_toml_path: &Path) -> Option<String> {
    let content = fs::read_to_string(cargo_toml_path).ok()?;
    // Simple parsing: look for name = "..." in [package] section
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("name") && line.contains('=') {
            // Extract the value between quotes
            if let Some(start) = line.find('"') {
                if let Some(end) = line[start + 1..].find('"') {
                    return Some(line[start + 1..start + 1 + end].to_string());
                }
            }
        }
    }
    None
}

/// List all Rust packages in the repository
pub fn list_rust_packages(repo: &Path) -> Result<Vec<String>> {
    let packages_dir = repo.join("packages");
    let mut packages = Vec::new();

    if packages_dir.exists() {
        for entry in fs::read_dir(&packages_dir)? {
            let entry = entry?;
            let path = entry.path();
            let cargo_toml = path.join("Cargo.toml");
            if path.is_dir() && cargo_toml.exists() {
                // Read actual package name from Cargo.toml
                if let Some(name) = get_cargo_package_name(&cargo_toml) {
                    packages.push(name);
                }
            }
        }
    }

    let dev_cli = repo.join("dev").join("cli");
    let dev_cli_cargo = dev_cli.join("Cargo.toml");
    if dev_cli_cargo.exists() {
        if let Some(name) = get_cargo_package_name(&dev_cli_cargo) {
            packages.push(name);
        } else {
            packages.push("dev-cli".to_string());
        }
    }

    packages.sort();
    Ok(packages)
}
