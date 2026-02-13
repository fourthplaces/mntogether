//! Scripts and utilities for common tasks

use anyhow::{Context, Result};
use clap::Subcommand;
use console::style;
use devkit_core::AppContext;
use dialoguer::FuzzySelect;
use std::process::Command;

/// Available scripts/binaries
const SCRIPTS: &[(&str, &str, &str)] = &[
    (
        "seed",
        "seed_organizations",
        "Seed organizations from JSON data",
    ),
    (
        "embeddings",
        "generate_embeddings",
        "Generate embeddings for organizations",
    ),
];

#[derive(Subcommand)]
pub enum ScriptsCommand {
    /// List available scripts
    List,

    /// Seed organizations from JSON data
    Seed,

    /// Generate embeddings for organizations
    Embeddings,

    /// Run a script by name
    Run {
        /// Script name
        name: Option<String>,
    },
}

pub fn run(ctx: &AppContext, cmd: ScriptsCommand) -> Result<()> {
    match cmd {
        ScriptsCommand::List => run_list(ctx),
        ScriptsCommand::Seed => run_script(ctx, "seed_organizations"),
        ScriptsCommand::Embeddings => run_script(ctx, "generate_embeddings"),
        ScriptsCommand::Run { name } => {
            let name = match name {
                Some(n) => resolve_script_name(&n),
                None => select_script(ctx)?,
            };
            run_script(ctx, &name)
        }
    }
}

fn resolve_script_name(name: &str) -> String {
    // Allow short names
    for (short, full, _) in SCRIPTS {
        if *short == name || *full == name {
            return full.to_string();
        }
    }
    name.to_string()
}

fn select_script(ctx: &AppContext) -> Result<String> {
    if ctx.quiet {
        anyhow::bail!("Script selection requires interactive mode");
    }

    let items: Vec<String> = SCRIPTS
        .iter()
        .map(|(short, _, desc)| format!("{} - {}", short, desc))
        .collect();

    let selection = FuzzySelect::with_theme(&ctx.theme())
        .with_prompt("Select script to run")
        .items(&items)
        .default(0)
        .interact()?;

    Ok(SCRIPTS[selection].1.to_string())
}

fn run_list(ctx: &AppContext) -> Result<()> {
    ctx.print_header("Available scripts");
    println!();

    for (short, full, desc) in SCRIPTS {
        println!(
            "  {} ({}) - {}",
            style(short).cyan().bold(),
            style(full).dim(),
            desc
        );
    }

    println!();
    println!("Run with: {} scripts run <name>", style("./dev.sh").green());
    println!("      or: {} scripts <name>", style("./dev.sh").green());

    Ok(())
}

fn run_script(ctx: &AppContext, bin_name: &str) -> Result<()> {
    ctx.print_header(&format!("Running {}", bin_name));

    // First, build the binary
    let build_status = Command::new("cargo")
        .args([
            "build",
            "--release",
            "-p",
            "server",
            "--bin",
            bin_name,
        ])
        .current_dir(&ctx.repo)
        .status()
        .context("Failed to build script")?;

    if !build_status.success() {
        anyhow::bail!("Failed to build {}", bin_name);
    }

    // Run the binary
    let bin_path = ctx.repo.join("target/release").join(bin_name);

    let status = Command::new(&bin_path)
        .current_dir(&ctx.repo)
        .status()
        .context(format!("Failed to run {}", bin_name))?;

    if status.success() {
        ctx.print_success(&format!("{} completed successfully", bin_name));
    } else {
        anyhow::bail!("{} failed with exit code: {:?}", bin_name, status.code());
    }

    Ok(())
}
