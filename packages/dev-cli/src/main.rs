use anyhow::{Context, Result};
use colored::Colorize;
use console::Term;
use dialoguer::{theme::ColorfulTheme, Select};
use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() -> Result<()> {
    let term = Term::stdout();

    // Print banner
    print_banner(&term)?;

    // Check if this is first run (no node_modules, no cargo build)
    let project_root = get_project_root()?;
    let needs_setup = check_needs_setup(&project_root)?;

    if needs_setup {
        println!("{}", "🚀 First time setup detected!".bright_green().bold());
        println!();
        run_initial_setup(&project_root)?;
    }

    // Main interactive loop
    loop {
        println!();
        let options = vec![
            "📱 Start mobile (Expo)",
            "🐳 Docker start",
            "🔄 Docker restart",
            "🔨 Docker rebuild",
            "📋 Follow docker logs",
            "🛑 Exit",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("What would you like to do?")
            .items(&options)
            .default(0)
            .interact_on(&term)?;

        match selection {
            0 => start_mobile(&project_root)?,
            1 => docker_start(&project_root)?,
            2 => docker_restart(&project_root)?,
            3 => docker_rebuild(&project_root)?,
            4 => docker_logs(&project_root)?,
            5 => {
                println!("{}", "👋 Goodbye!".bright_blue());
                break;
            }
            _ => unreachable!(),
        }
    }

    Ok(())
}

fn print_banner(term: &Term) -> Result<()> {
    term.clear_screen()?;
    println!(
        "{}",
        "╔════════════════════════════════════════╗".bright_cyan()
    );
    println!(
        "{}",
        "║   Minnesota Digital Aid Dev CLI      ║".bright_cyan()
    );
    println!(
        "{}",
        "╚════════════════════════════════════════╝".bright_cyan()
    );
    println!();
    Ok(())
}

fn get_project_root() -> Result<PathBuf> {
    // Get the directory containing Cargo.toml at workspace root
    let current_exe = env::current_exe()?;
    let mut path = current_exe
        .parent()
        .context("Failed to get parent directory")?
        .to_path_buf();

    // Navigate up to find workspace root (contains Cargo.toml with [workspace])
    loop {
        let cargo_toml = path.join("Cargo.toml");
        if cargo_toml.exists() {
            let contents = std::fs::read_to_string(&cargo_toml)?;
            if contents.contains("[workspace]") {
                return Ok(path);
            }
        }

        if !path.pop() {
            break;
        }
    }

    // Fallback to current directory
    env::current_dir().context("Failed to get current directory")
}

fn check_needs_setup(project_root: &PathBuf) -> Result<bool> {
    // Check if node_modules exists in app package
    let app_node_modules = project_root.join("packages/app/node_modules");

    Ok(!app_node_modules.exists())
}

fn run_initial_setup(project_root: &PathBuf) -> Result<()> {
    println!("{}", "Checking dependencies...".bright_yellow());

    // Check for required tools
    check_dependency("cargo", "Rust")?;
    check_dependency("docker", "Docker")?;
    check_dependency("node", "Node.js")?;
    check_dependency("npm", "npm")?;

    println!();
    println!("{}", "Installing dependencies...".bright_yellow());
    println!();

    // Install Expo CLI globally if not present
    if which::which("expo").is_err() {
        println!("{}", "📦 Installing Expo CLI...".bright_blue());
        run_command("npm", &["install", "-g", "expo-cli"], project_root)?;
    }

    // Install app dependencies
    println!("{}", "📦 Installing app dependencies...".bright_blue());
    let app_dir = project_root.join("packages/app");
    run_command("npm", &["install"], &app_dir)?;

    // Build Rust workspace
    println!("{}", "🦀 Building Rust workspace...".bright_blue());
    run_command("cargo", &["build"], project_root)?;

    println!();
    println!("{}", "✅ Setup complete!".bright_green().bold());

    Ok(())
}

fn check_dependency(cmd: &str, name: &str) -> Result<()> {
    match which::which(cmd) {
        Ok(_) => {
            println!("  {} {}", "✓".bright_green(), name);
            Ok(())
        }
        Err(_) => {
            println!("  {} {} is not installed", "✗".bright_red(), name);
            Err(anyhow::anyhow!(
                "{} is required but not found. Please install it first.",
                name
            ))
        }
    }
}

fn run_command(cmd: &str, args: &[&str], cwd: &PathBuf) -> Result<()> {
    let status = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .status()
        .context(format!("Failed to execute: {} {:?}", cmd, args))?;

    if !status.success() {
        return Err(anyhow::anyhow!("Command failed: {} {:?}", cmd, args));
    }

    Ok(())
}

fn start_mobile(project_root: &PathBuf) -> Result<()> {
    println!("{}", "📱 Starting Expo...".bright_blue().bold());
    println!("{}", "   Press Ctrl+C to stop".dimmed());
    println!();

    let app_dir = project_root.join("packages/app");

    let status = Command::new("npm")
        .args(&["start"])
        .current_dir(&app_dir)
        .status()
        .context("Failed to start Expo")?;

    if !status.success() {
        println!("{}", "❌ Failed to start Expo".bright_red());
    }

    Ok(())
}

fn docker_start(project_root: &PathBuf) -> Result<()> {
    println!("{}", "🐳 Starting Docker services...".bright_blue().bold());

    let server_dir = project_root.join("packages/server");

    let status = Command::new("docker")
        .args(&["compose", "up", "-d"])
        .current_dir(&server_dir)
        .status()
        .context("Failed to start Docker")?;

    if status.success() {
        println!("{}", "✅ Docker services started".bright_green());
        println!();
        println!("Services available at:");
        println!("  {} http://localhost:8080", "API:".bright_yellow());
        println!("  {} localhost:5432", "PostgreSQL:".bright_yellow());
        println!("  {} localhost:6379", "Redis:".bright_yellow());
    } else {
        println!("{}", "❌ Failed to start Docker services".bright_red());
    }

    Ok(())
}

fn docker_restart(project_root: &PathBuf) -> Result<()> {
    println!(
        "{}",
        "🔄 Restarting Docker services...".bright_blue().bold()
    );

    let server_dir = project_root.join("packages/server");

    let status = Command::new("docker")
        .args(&["compose", "restart"])
        .current_dir(&server_dir)
        .status()
        .context("Failed to restart Docker")?;

    if status.success() {
        println!("{}", "✅ Docker services restarted".bright_green());
    } else {
        println!("{}", "❌ Failed to restart Docker services".bright_red());
    }

    Ok(())
}

fn docker_rebuild(project_root: &PathBuf) -> Result<()> {
    println!(
        "{}",
        "🔨 Rebuilding Docker services...".bright_blue().bold()
    );
    println!("{}", "   This may take a few minutes...".dimmed());

    let server_dir = project_root.join("packages/server");

    let status = Command::new("docker")
        .args(&["compose", "up", "-d", "--build"])
        .current_dir(&server_dir)
        .status()
        .context("Failed to rebuild Docker")?;

    if status.success() {
        println!(
            "{}",
            "✅ Docker services rebuilt and started".bright_green()
        );
    } else {
        println!("{}", "❌ Failed to rebuild Docker services".bright_red());
    }

    Ok(())
}

fn docker_logs(project_root: &PathBuf) -> Result<()> {
    println!("{}", "📋 Following Docker logs...".bright_blue().bold());
    println!("{}", "   Press Ctrl+C to stop".dimmed());
    println!();

    let server_dir = project_root.join("packages/server");

    let status = Command::new("docker")
        .args(&["compose", "logs", "-f"])
        .current_dir(&server_dir)
        .status()
        .context("Failed to follow Docker logs")?;

    if !status.success() {
        println!("{}", "❌ Failed to follow Docker logs".bright_red());
    }

    Ok(())
}
