//! Project-specific development CLI
//!
//! Customize this file to add your own commands and workflows.

use anyhow::Result;
use clap::{Parser, Subcommand};
use devkit_core::AppContext;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "dev")]
#[command(about = "Development environment CLI")]
#[command(version)]
struct Cli {
    /// Run in quiet mode (non-interactive)
    #[arg(short, long, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start development environment
    Start,

    /// Stop all services
    Stop,

    /// Show environment status
    Status,

    /// Check system prerequisites
    Doctor,

    /// Run package-defined commands
    ///
    /// Commands are defined in package dev.toml files.
    /// Example: dev cmd build, dev cmd test
    Cmd {
        /// Command name (e.g., build, test, lint)
        command: Option<String>,

        /// Run in parallel where possible
        #[arg(long)]
        parallel: bool,

        /// Only run for specific packages
        #[arg(short, long)]
        package: Vec<String>,

        /// List all available commands
        #[arg(long)]
        list: bool,
    },
}

fn main() -> ExitCode {
    // Load environment variables
    let _ = dotenvy::dotenv();

    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let ctx = AppContext::new(cli.quiet)?;

    match cli.command {
        Some(Commands::Start) => cmd_start(&ctx),
        Some(Commands::Stop) => cmd_stop(&ctx),
        Some(Commands::Status) => cmd_status(&ctx),
        Some(Commands::Doctor) => cmd_doctor(&ctx),
        Some(Commands::Cmd {
            command,
            parallel,
            package,
            list,
        }) => cmd_run(&ctx, command, parallel, package, list),
        None => interactive_menu(&ctx),
    }
}

fn cmd_start(ctx: &AppContext) -> Result<()> {
    ctx.print_header("Starting development environment");
    ctx.print_info("TODO: Implement start command");
    ctx.print_info("Suggestions:");
    ctx.print_info("  - Start Docker containers");
    ctx.print_info("  - Pull environment variables");
    ctx.print_info("  - Run database migrations");
    Ok(())
}

fn cmd_stop(ctx: &AppContext) -> Result<()> {
    ctx.print_header("Stopping development environment");
    ctx.print_info("TODO: Implement stop command");
    Ok(())
}

fn cmd_status(ctx: &AppContext) -> Result<()> {
    ctx.print_header("Development Environment Status");
    println!();
    println!("Repository: {}", ctx.repo.display());
    println!("Project: {}", ctx.config.global.project.name);
    println!();
    ctx.print_success("✓ Configuration loaded");
    Ok(())
}

fn cmd_doctor(ctx: &AppContext) -> Result<()> {
    ctx.print_header("System Health Check");
    println!();

    let tools = vec![
        ("git", devkit_core::utils::cmd_exists("git")),
        ("cargo", devkit_core::utils::cmd_exists("cargo")),
        ("docker", devkit_core::utils::docker_available()),
    ];

    for (tool, available) in tools {
        if available {
            ctx.print_success(&format!("✓ {}", tool));
        } else {
            ctx.print_warning(&format!("✗ {} (not found)", tool));
        }
    }

    println!();
    ctx.print_success("Health check complete");
    Ok(())
}

fn cmd_run(
    ctx: &AppContext,
    command: Option<String>,
    parallel: bool,
    packages: Vec<String>,
    list: bool,
) -> Result<()> {
    use devkit_tasks::{list_commands, print_results, run_cmd, CmdOptions};

    if list {
        let commands = list_commands(&ctx.config);
        if commands.is_empty() {
            println!("No commands defined.");
            println!();
            println!("Add commands to package dev.toml files:");
            println!();
            println!("  [cmd]");
            println!("  build = \"cargo build\"");
            println!("  test = \"cargo test\"");
            return Ok(());
        }

        println!("Available commands:");
        println!();
        for (cmd, pkgs) in commands {
            println!("  {} ({})", cmd, pkgs.join(", "));
        }
        return Ok(());
    }

    let cmd_name = match command {
        Some(c) => c,
        None => {
            ctx.print_warning("No command specified. Use --list to see available commands.");
            return Ok(());
        }
    };

    let opts = CmdOptions {
        parallel,
        variant: None,
        packages,
        capture: false,
    };

    let results = run_cmd(ctx, &cmd_name, &opts)?;
    print_results(ctx, &results);

    if results.iter().any(|r| !r.success) {
        return Err(anyhow::anyhow!("Some commands failed"));
    }

    Ok(())
}

fn interactive_menu(ctx: &AppContext) -> Result<()> {
    use dialoguer::Select;

    let items = vec![
        "Start development environment",
        "Stop services",
        "Run commands (cmd)",
        "Status",
        "Doctor",
        "Exit",
    ];

    loop {
        println!();
        let choice = Select::with_theme(&ctx.theme())
            .with_prompt("What would you like to do?")
            .items(&items)
            .default(0)
            .interact()?;

        match choice {
            0 => cmd_start(ctx)?,
            1 => cmd_stop(ctx)?,
            2 => {
                // Show available commands
                cmd_run(ctx, None, false, vec![], true)?;
            }
            3 => cmd_status(ctx)?,
            4 => cmd_doctor(ctx)?,
            _ => break,
        }
    }

    Ok(())
}
