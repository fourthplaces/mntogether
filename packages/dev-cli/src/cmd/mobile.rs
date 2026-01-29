//! Mobile development commands

#![allow(dead_code)]

use anyhow::{anyhow, Result};
use console::style;
use dialoguer::{MultiSelect, Select};
use std::path::Path;

use crate::cmd_builder::CmdBuilder;
use crate::compose::run_docker_compose;
use crate::config::Config;
use crate::context::AppContext;
use crate::services;
use crate::utils::cmd_exists;

/// EAS build profiles
const EAS_PROFILES: &[(&str, &str)] = &[
    ("development", "Development client (simulator/internal)"),
    ("preview", "Preview build (internal distribution)"),
    ("production", "Production build (app store)"),
];

/// Platforms for EAS build
const EAS_PLATFORMS: &[(&str, &str)] = &[
    ("all", "iOS and Android"),
    ("ios", "iOS only"),
    ("android", "Android only"),
];

fn ensure_xcode_cli_tools(ctx: &AppContext) -> Result<()> {
    if !cfg!(target_os = "macos") {
        return Ok(());
    }
    if cmd_exists("xcodebuild") {
        return Ok(());
    }

    ctx.print_warning("Xcode not detected (xcodebuild missing). iOS builds require Xcode.");

    if ctx.confirm(
        "Run `xcode-select --install` now? (may open a GUI prompt)",
        true,
    )? {
        let code = CmdBuilder::new("xcode-select")
            .arg("--install")
            .cwd(&ctx.repo)
            .run()?;
        if code != 0 {
            return Err(anyhow!("xcode-select exited with code {code}"));
        }
    }
    Ok(())
}

/// Wait for the API server to be available
fn wait_for_api_server(ctx: &AppContext, url: &str, timeout_secs: u64) -> Result<bool> {
    use std::time::{Duration, Instant};

    let timeout = Duration::from_secs(timeout_secs);
    let start = Instant::now();
    let check_interval = Duration::from_secs(2);

    ctx.print_info(&format!("Waiting for API server at {}...", url));
    ctx.print_info("(This may take a few minutes on cold start while Rust compiles)");

    let mut last_message_time = 0u64;

    while start.elapsed() < timeout {
        // Use curl to check if the server is responding
        let result = CmdBuilder::new("curl")
            .args(["-s", "-o", "/dev/null", "-w", "%{http_code}", url])
            .capture_stdout()
            .capture_stderr()
            .run_capture();

        if let Ok(output) = result {
            let status = output.stdout_string().trim().to_string();
            if status.starts_with('2') || status.starts_with('3') || status == "400" {
                let elapsed = start.elapsed().as_secs();
                ctx.print_success(&format!("API server is ready! (took {}s)", elapsed));
                return Ok(true);
            }
        }

        // Show progress with helpful messages
        let elapsed = start.elapsed().as_secs();
        if elapsed >= last_message_time + 30 {
            last_message_time = elapsed;
            let minutes = elapsed / 60;
            let seconds = elapsed % 60;
            let time_str = if minutes > 0 {
                format!("{}m {}s", minutes, seconds)
            } else {
                format!("{}s", seconds)
            };

            let message = match elapsed {
                0..=59 => format!("Still waiting for API server... ({})", time_str),
                60..=119 => format!(
                    "API server still compiling... ({}) - Rust builds can take a while on first run",
                    time_str
                ),
                120..=179 => format!(
                    "Hang tight, compilation in progress... ({}) - This is normal for cold starts",
                    time_str
                ),
                _ => format!(
                    "Still waiting... ({}) - Check docker logs if this seems too long",
                    time_str
                ),
            };
            ctx.print_info(&message);
        }

        std::thread::sleep(check_interval);
    }

    let minutes = timeout_secs / 60;
    ctx.print_warning(&format!(
        "API server did not respond within {} minute timeout. You may need to check docker logs.",
        minutes
    ));
    Ok(false)
}

/// Get API URL from config or default
fn get_api_url(ctx: &AppContext, config: Option<&Config>, path: &str) -> String {
    config
        .map(|c| {
            let default_port = c.global.services.get_port("api", 8080);
            let port =
                services::get_service_port(&ctx.repo, "api", default_port).unwrap_or(default_port);
            format!("http://localhost:{}{}", port, path)
        })
        .unwrap_or_else(|| format!("http://localhost:8080{}", path))
}

/// Run pre-run scripts for mobile development
/// This includes GraphQL codegen and any custom scripts
fn run_mobile_pre_run(ctx: &AppContext, mobile_dir: &Path) -> Result<()> {
    run_mobile_pre_run_with_config(ctx, None, mobile_dir)
}

/// Run pre-run scripts for mobile development with config
fn run_mobile_pre_run_with_config(
    ctx: &AppContext,
    config: Option<&Config>,
    mobile_dir: &Path,
) -> Result<()> {
    ctx.print_header("Running mobile pre-run scripts...");

    // 1. Wait for API server to be available before generating GraphQL code
    let api_url = get_api_url(ctx, config, "/graphql");
    let api_ready = wait_for_api_server(ctx, &api_url, 60 * 5)?;

    // 2. Generate GraphQL code
    if api_ready {
        ctx.print_info("Generating GraphQL code...");
        let code = CmdBuilder::new("npm")
            .args(["run", "graphql:build"])
            .cwd(mobile_dir)
            .run()?;
        if code != 0 {
            ctx.print_warning("GraphQL codegen failed. Continuing anyway...");
        } else {
            ctx.print_success("GraphQL code generated successfully.");
        }
    } else {
        ctx.print_warning("Skipping GraphQL codegen (API server not available).");
    }

    // 2. Run custom pre-run script if it exists
    let pre_run_script = mobile_dir.join("scripts").join("pre-run.js");
    if pre_run_script.exists() {
        ctx.print_info("Running custom pre-run script...");
        let code = CmdBuilder::new("node")
            .arg("scripts/pre-run.js")
            .cwd(mobile_dir)
            .run()?;
        if code != 0 {
            return Err(anyhow!("pre-run.js exited with code {code}"));
        }
        ctx.print_success("Custom pre-run script completed.");
    }

    // 3. Run shell pre-run script if it exists
    let pre_run_sh = mobile_dir.join("scripts").join("pre-run.sh");
    if pre_run_sh.exists() {
        ctx.print_info("Running shell pre-run script...");
        let code = CmdBuilder::new("bash")
            .arg("scripts/pre-run.sh")
            .cwd(mobile_dir)
            .run()?;
        if code != 0 {
            return Err(anyhow!("pre-run.sh exited with code {code}"));
        }
        ctx.print_success("Shell pre-run script completed.");
    }

    Ok(())
}

/// Regenerate GraphQL types for mobile app
pub fn run_mobile_codegen(ctx: &AppContext) -> Result<()> {
    run_mobile_codegen_with_config(ctx, None)
}

/// Regenerate GraphQL types for mobile app with config
pub fn run_mobile_codegen_with_config(ctx: &AppContext, config: Option<&Config>) -> Result<()> {
    ctx.print_header("Regenerating mobile GraphQL types");

    let mobile_dir = ctx.repo.join("packages").join("app");
    if !mobile_dir.exists() {
        return Err(anyhow!("Expected mobile app at {}", mobile_dir.display()));
    }

    // Wait for API server to be available
    let api_url = get_api_url(ctx, config, "/graphql");
    let api_ready = wait_for_api_server(ctx, &api_url, 60 * 5)?;

    if !api_ready {
        return Err(anyhow!(
            "API server not available. Make sure docker containers are running."
        ));
    }

    // Run GraphQL codegen
    ctx.print_info("Running graphql-codegen...");
    let code = CmdBuilder::new("npm")
        .args(["run", "graphql:build"])
        .cwd(&mobile_dir)
        .run()?;

    if code != 0 {
        return Err(anyhow!("GraphQL codegen failed with code {code}"));
    }

    ctx.print_success("GraphQL types regenerated successfully.");
    Ok(())
}

/// Start mobile development
pub fn start_mobile(ctx: &AppContext, platform: Option<&str>) -> Result<()> {
    ctx.print_header("Mobile development");

    ensure_xcode_cli_tools(ctx)?;

    if cmd_exists("docker") || cmd_exists("docker-compose") {
        ctx.print_header("Starting docker containers in background (docker compose up -d)...");
        let code = run_docker_compose(&ctx.repo, &["up", "-d"])?;
        if code != 0 {
            return Err(anyhow!("docker compose exited with code {code}"));
        }
    } else {
        ctx.print_warning("Docker Compose not found; skipping `docker compose up -d`.");
    }

    let mobile_dir = ctx.repo.join("packages").join("app");
    if !mobile_dir.exists() {
        return Err(anyhow!("Expected mobile app at {}", mobile_dir.display()));
    }

    if !cmd_exists("yarn") {
        return Err(anyhow!(
            "yarn not found. Install yarn to run the mobile app commands."
        ));
    }

    let needs_install = !mobile_dir.join("node_modules").exists();
    if needs_install && ctx.confirm("Mobile dependencies missing. Run `yarn install` now?", true)? {
        let code = CmdBuilder::new("yarn")
            .arg("install")
            .cwd(&mobile_dir)
            .run()?;
        if code != 0 {
            return Err(anyhow!("yarn install exited with code {code}"));
        }
    }

    // Run pre-run scripts (GraphQL codegen, etc.)
    run_mobile_pre_run(ctx, &mobile_dir)?;

    let platform = if let Some(p) = platform {
        p.to_string()
    } else if ctx.quiet {
        "ios".to_string()
    } else {
        let items = vec![
            "Expo (npx expo start)",
            "Web (npx expo start --web)",
            "iOS (yarn ios)",
            "Android (yarn android)",
        ];
        let choice = Select::with_theme(&ctx.theme())
            .with_prompt("What do you want to run?")
            .items(&items)
            .default(0)
            .interact()?;
        match choice {
            0 => "expo".to_string(),
            1 => "web".to_string(),
            2 => "ios".to_string(),
            3 => "android".to_string(),
            _ => return Ok(()),
        }
    };

    if !cmd_exists("npx") {
        return Err(anyhow!(
            "npx not found. Install Node/npm to run Expo commands."
        ));
    }

    let code = if platform == "expo" {
        // Just launch Expo dev server
        CmdBuilder::new("npx")
            .args(["expo", "start"])
            .cwd(&mobile_dir)
            .run()?
    } else if platform == "web" {
        // Launch Expo with web
        CmdBuilder::new("npx")
            .args(["expo", "start", "--web"])
            .cwd(&mobile_dir)
            .run()?
    } else {
        // Use `expo run:ios/android` which handles building the dev client
        // if it's not already installed
        CmdBuilder::new("npx")
            .args(["expo", &format!("run:{platform}")])
            .cwd(&mobile_dir)
            .run()?
    };

    if code != 0 {
        return Err(anyhow!("mobile command exited with code {code}"));
    }
    Ok(())
}

// =============================================================================
// EAS Build
// =============================================================================

/// Get available app directories with eas.json
pub fn get_eas_apps(ctx: &AppContext) -> Vec<(String, std::path::PathBuf)> {
    let mut apps = Vec::new();

    // Check packages directory for apps with eas.json
    let packages_dir = ctx.repo.join("packages");
    if let Ok(entries) = std::fs::read_dir(&packages_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("eas.json").exists() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    apps.push((name.to_string(), path));
                }
            }
        }
    }

    // Check root for eas.json (monorepo root app)
    if ctx.repo.join("eas.json").exists() {
        apps.push(("root".to_string(), ctx.repo.clone()));
    }

    apps
}

/// Interactive EAS build
pub fn eas_build_interactive(ctx: &AppContext) -> Result<()> {
    ctx.print_header("EAS Build");

    ensure_eas_cli(ctx)?;

    let apps = get_eas_apps(ctx);
    if apps.is_empty() {
        return Err(anyhow!("No apps with eas.json found"));
    }

    // Select app (if multiple)
    let app_path = if apps.len() == 1 {
        apps[0].1.clone()
    } else {
        let app_labels: Vec<String> = apps.iter().map(|(name, _)| name.clone()).collect();
        let choice = Select::with_theme(&ctx.theme())
            .with_prompt("Select app")
            .items(&app_labels)
            .default(0)
            .interact()?;
        apps[choice].1.clone()
    };

    // Select profile
    let profile_labels: Vec<String> = EAS_PROFILES
        .iter()
        .map(|(name, desc)| format!("{:<12} {}", name, desc))
        .collect();

    let profile_choice = Select::with_theme(&ctx.theme())
        .with_prompt("Build profile")
        .items(&profile_labels)
        .default(0)
        .interact()?;

    let profile = EAS_PROFILES[profile_choice].0;

    // Select platform
    let platform_labels: Vec<String> = EAS_PLATFORMS
        .iter()
        .map(|(name, desc)| format!("{:<10} {}", name, desc))
        .collect();

    let platform_choice = Select::with_theme(&ctx.theme())
        .with_prompt("Platform")
        .items(&platform_labels)
        .default(0)
        .interact()?;

    let platform = EAS_PLATFORMS[platform_choice].0;

    run_eas_build(ctx, &app_path, profile, platform, None)
}

/// Run EAS build with specific options
pub fn run_eas_build(
    ctx: &AppContext,
    app_path: &Path,
    profile: &str,
    platform: &str,
    message: Option<&str>,
) -> Result<()> {
    ensure_eas_cli(ctx)?;

    ctx.print_header(&format!("EAS Build: {} ({})", profile, platform));

    let mut args = vec!["build".to_string()];

    // Profile
    args.push("--profile".to_string());
    args.push(profile.to_string());

    // Platform
    if platform != "all" {
        args.push("--platform".to_string());
        args.push(platform.to_string());
    }

    // Message (for tracking releases)
    if let Some(msg) = message {
        args.push("--message".to_string());
        args.push(msg.to_string());
    }

    // Non-interactive for CI
    args.push("--non-interactive".to_string());

    println!();
    println!("Running: eas {}", args.join(" "));
    println!("Directory: {}", app_path.display());
    println!();

    let code = CmdBuilder::new("eas").args(args).cwd(app_path).run()?;

    if code != 0 {
        return Err(anyhow!("EAS build failed with code {}", code));
    }

    ctx.print_success("EAS build started!");
    println!();
    println!(
        "Monitor builds at: {}",
        style("https://expo.dev/builds").cyan()
    );

    Ok(())
}

/// Run EAS build for production release
pub fn run_eas_production_build(
    ctx: &AppContext,
    version_tag: &str,
    platforms: &[&str],
) -> Result<()> {
    ensure_eas_cli(ctx)?;

    let apps = get_eas_apps(ctx);
    if apps.is_empty() {
        ctx.print_warning("No apps with eas.json found. Skipping mobile builds.");
        return Ok(());
    }

    ctx.print_header("EAS Production Build");

    // If multiple apps, let user select which to build
    let apps_to_build: Vec<&(String, std::path::PathBuf)> = if apps.len() == 1 {
        vec![&apps[0]]
    } else {
        let app_labels: Vec<&str> = apps.iter().map(|(name, _)| name.as_str()).collect();
        let defaults = vec![true; apps.len()];

        let selections = MultiSelect::with_theme(&ctx.theme())
            .with_prompt("Select apps to build")
            .items(&app_labels)
            .defaults(&defaults)
            .interact()?;

        if selections.is_empty() {
            println!("No apps selected. Skipping mobile builds.");
            return Ok(());
        }

        selections.iter().map(|&i| &apps[i]).collect()
    };

    let message = format!("Release {}", version_tag);

    for (app_name, app_path) in apps_to_build {
        println!();
        println!("Building {} for production...", style(app_name).cyan());

        for platform in platforms {
            run_eas_build(ctx, app_path, "production", platform, Some(&message))?;
        }
    }

    Ok(())
}

/// Ensure EAS CLI is installed
fn ensure_eas_cli(ctx: &AppContext) -> Result<()> {
    if cmd_exists("eas") {
        return Ok(());
    }

    ctx.print_warning("EAS CLI not found.");

    if ctx.confirm("Install EAS CLI globally (npm install -g eas-cli)?", true)? {
        let code = CmdBuilder::new("npm")
            .args(["install", "-g", "eas-cli"])
            .run()?;

        if code != 0 {
            return Err(anyhow!("Failed to install EAS CLI"));
        }

        ctx.print_success("EAS CLI installed!");
    } else {
        return Err(anyhow!(
            "EAS CLI required. Install with: npm install -g eas-cli"
        ));
    }

    Ok(())
}

/// Submit to app stores
pub fn eas_submit(ctx: &AppContext, platform: Option<&str>) -> Result<()> {
    ensure_eas_cli(ctx)?;

    ctx.print_header("EAS Submit");

    let apps = get_eas_apps(ctx);
    if apps.is_empty() {
        return Err(anyhow!("No apps with eas.json found"));
    }

    // Select app
    let app_path = if apps.len() == 1 {
        apps[0].1.clone()
    } else {
        let app_labels: Vec<String> = apps.iter().map(|(name, _)| name.clone()).collect();
        let choice = Select::with_theme(&ctx.theme())
            .with_prompt("Select app")
            .items(&app_labels)
            .default(0)
            .interact()?;
        apps[choice].1.clone()
    };

    // Select platform if not provided
    let platform = match platform {
        Some(p) => p.to_string(),
        None => {
            let choices = vec!["ios", "android"];
            let choice = Select::with_theme(&ctx.theme())
                .with_prompt("Submit to")
                .items(&choices)
                .default(0)
                .interact()?;
            choices[choice].to_string()
        }
    };

    println!();
    println!("Submitting latest {} build...", platform);

    let code = CmdBuilder::new("eas")
        .args([
            "submit",
            "--platform",
            &platform,
            "--latest",
            "--non-interactive",
        ])
        .cwd(&app_path)
        .run()?;

    if code != 0 {
        return Err(anyhow!("EAS submit failed"));
    }

    ctx.print_success(&format!("Submitted to {} store!", platform));

    Ok(())
}

/// List recent EAS builds
pub fn eas_builds_list(ctx: &AppContext) -> Result<()> {
    ensure_eas_cli(ctx)?;

    ctx.print_header("Recent EAS Builds");

    let apps = get_eas_apps(ctx);
    let app_path = if apps.is_empty() {
        &ctx.repo
    } else {
        &apps[0].1
    };

    CmdBuilder::new("eas")
        .args(["build:list", "--limit", "10"])
        .cwd(app_path)
        .run()?;

    Ok(())
}
