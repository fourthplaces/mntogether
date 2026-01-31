//! Data migration commands for surgical database transformations
//!
//! These commands orchestrate the migrate_cli binary which implements
//! the actual DataMigration trait logic.

use anyhow::{Context, Result};
use clap::Subcommand;
use console::style;
use devkit_core::AppContext;
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::time::Duration;

#[derive(Subcommand)]
pub enum MigrateCommand {
    /// List all migrations with their status and pending counts
    List,

    /// Show status of all migrations (pending items, completion %)
    StatusAll,

    /// Dry-run all pending migrations
    RunAll,

    /// Execute all pending migrations
    StartAll,

    /// Estimate the number of items to migrate
    Estimate {
        /// Migration name (optional - will prompt for selection)
        name: Option<String>,
    },

    /// Dry-run a specific migration (validates without mutations)
    Run {
        /// Migration name (optional - will prompt for selection)
        name: Option<String>,
    },

    /// Execute a specific migration
    Start {
        /// Migration name (optional - will prompt for selection)
        name: Option<String>,
    },

    /// Check migration progress
    Status {
        /// Migration name (optional - will prompt for selection)
        name: Option<String>,
    },

    /// Pause a running migration
    Pause {
        /// Migration name (optional - will prompt for selection)
        name: Option<String>,
    },

    /// Resume a paused migration
    Resume {
        /// Migration name (optional - will prompt for selection)
        name: Option<String>,
    },

    /// Verify migration integrity
    Verify {
        /// Migration name (optional - will prompt for selection)
        name: Option<String>,
    },

    /// Mark migration as complete
    Complete {
        /// Migration name (optional - will prompt for selection)
        name: Option<String>,
    },
}

pub fn run(ctx: &AppContext, cmd: MigrateCommand) -> Result<()> {
    match cmd {
        MigrateCommand::List => run_list(ctx),
        MigrateCommand::StatusAll => run_status_all(ctx),
        MigrateCommand::RunAll => run_all(ctx, true),
        MigrateCommand::StartAll => run_all(ctx, false),
        MigrateCommand::Estimate { name } => {
            let name = resolve_migration_name(ctx, name)?;
            run_estimate(ctx, &name)
        }
        MigrateCommand::Run { name } => {
            let name = resolve_migration_name(ctx, name)?;
            run_dry_run(ctx, &name)
        }
        MigrateCommand::Start { name } => {
            let name = resolve_migration_name(ctx, name)?;
            run_start(ctx, &name)
        }
        MigrateCommand::Status { name } => {
            let name = resolve_migration_name(ctx, name)?;
            run_status(ctx, &name)
        }
        MigrateCommand::Pause { name } => {
            let name = resolve_migration_name(ctx, name)?;
            run_pause(ctx, &name)
        }
        MigrateCommand::Resume { name } => {
            let name = resolve_migration_name(ctx, name)?;
            run_resume(ctx, &name)
        }
        MigrateCommand::Verify { name } => {
            let name = resolve_migration_name(ctx, name)?;
            run_verify(ctx, &name)
        }
        MigrateCommand::Complete { name } => {
            let name = resolve_migration_name(ctx, name)?;
            run_complete(ctx, &name)
        }
    }
}

/// If name is provided, return it. Otherwise, fetch the list of migrations
/// and prompt the user to select one.
fn resolve_migration_name(ctx: &AppContext, name: Option<String>) -> Result<String> {
    if let Some(n) = name {
        return Ok(n);
    }

    // Fetch available migrations
    let resp = run_migrate_cli(ctx, &["list"])?;
    let migrations = resp.migrations.unwrap_or_default();

    if migrations.is_empty() {
        anyhow::bail!("No migrations registered");
    }

    // Build selection items with name and description
    let items: Vec<String> = migrations
        .iter()
        .map(|m| {
            if let Some(desc) = &m.description {
                format!("{} - {}", m.name, desc)
            } else {
                m.name.clone()
            }
        })
        .collect();

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select migration")
        .items(&items)
        .default(0)
        .interact()
        .context("Failed to show migration selector")?;

    Ok(migrations[selection].name.clone())
}

/// Response from migrate_cli commands
#[derive(Debug, Deserialize)]
struct MigrateResponse {
    success: bool,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    count: Option<i64>,
    #[serde(default)]
    migrations: Option<Vec<MigrationInfo>>,
    #[serde(default)]
    status: Option<WorkflowStatus>,
}

#[derive(Debug, Deserialize)]
struct MigrationInfo {
    name: String,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WorkflowStatus {
    phase: String,
    total_items: i64,
    completed_items: i64,
    failed_items: i64,
    skipped_items: i64,
    #[serde(default)]
    error_rate: Option<f64>,
}

/// Progress update streamed during batch execution
#[derive(Debug, Deserialize)]
struct ProgressUpdate {
    #[serde(rename = "type")]
    update_type: String,
    #[serde(default)]
    completed: Option<i64>,
    #[serde(default)]
    total: Option<i64>,
    #[serde(default)]
    result: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

fn build_migrate_cli_path(ctx: &AppContext) -> std::path::PathBuf {
    ctx.repo.join("target/release/migrate_cli")
}

fn ensure_migrate_cli_built(ctx: &AppContext) -> Result<()> {
    let bin_path = build_migrate_cli_path(ctx);
    if bin_path.exists() {
        return Ok(());
    }

    ctx.print_info("Building migrate_cli...");
    let status = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--manifest-path",
            ctx.repo.join("packages/server/Cargo.toml").to_str().unwrap(),
            "--bin",
            "migrate_cli",
        ])
        .status()
        .context("Failed to build migrate_cli")?;

    if !status.success() {
        anyhow::bail!("Failed to build migrate_cli");
    }

    Ok(())
}

fn run_migrate_cli(ctx: &AppContext, args: &[&str]) -> Result<MigrateResponse> {
    ensure_migrate_cli_built(ctx)?;

    let bin_path = build_migrate_cli_path(ctx);
    let output = Command::new(&bin_path)
        .args(args)
        .output()
        .context("Failed to execute migrate_cli")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("migrate_cli failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).context("Failed to parse migrate_cli output")
}

fn run_list(ctx: &AppContext) -> Result<()> {
    ctx.print_header("Data migrations status");

    let resp = run_migrate_cli(ctx, &["list"])?;

    if let Some(migrations) = resp.migrations {
        if migrations.is_empty() {
            ctx.print_info("No migrations registered.");
            return Ok(());
        }

        println!();
        for m in &migrations {
            // Get estimate for each migration
            let estimate = run_migrate_cli(ctx, &["estimate", &m.name])
                .ok()
                .and_then(|r| r.count);

            let desc = m.description.clone().unwrap_or_default();
            let count_str = match estimate {
                Some(0) => style("✓ complete").green().to_string(),
                Some(n) => style(format!("{} pending", n)).yellow().to_string(),
                None => style("? unknown").dim().to_string(),
            };

            println!("  {} {} - {}", count_str, style(&m.name).cyan(), desc);
        }
        println!();
    }

    Ok(())
}

fn run_status_all(ctx: &AppContext) -> Result<()> {
    ctx.print_header("All migrations status");

    let resp = run_migrate_cli(ctx, &["list"])?;
    let migrations = resp.migrations.unwrap_or_default();

    if migrations.is_empty() {
        ctx.print_info("No migrations registered.");
        return Ok(());
    }

    println!();
    let mut total_pending = 0i64;
    let mut complete_count = 0;

    for m in &migrations {
        let estimate = run_migrate_cli(ctx, &["estimate", &m.name])
            .ok()
            .and_then(|r| r.count)
            .unwrap_or(0);

        let status_str = if estimate == 0 {
            complete_count += 1;
            style("✓ complete".to_string()).green()
        } else {
            total_pending += estimate;
            style(format!("{} pending", estimate)).yellow()
        };

        println!("  {:>12}  {}", status_str, style(&m.name).cyan());
    }

    println!();
    println!(
        "  Summary: {}/{} complete, {} total items pending",
        style(complete_count).green(),
        migrations.len(),
        style(total_pending).yellow()
    );
    println!();

    Ok(())
}

fn run_all(ctx: &AppContext, dry_run: bool) -> Result<()> {
    let mode = if dry_run { "Dry-run" } else { "Running" };
    ctx.print_header(&format!("{} all pending migrations", mode));

    let resp = run_migrate_cli(ctx, &["list"])?;
    let migrations = resp.migrations.unwrap_or_default();

    if migrations.is_empty() {
        ctx.print_info("No migrations registered.");
        return Ok(());
    }

    // Find migrations with pending work
    let mut pending_migrations = Vec::new();
    for m in &migrations {
        let estimate = run_migrate_cli(ctx, &["estimate", &m.name])
            .ok()
            .and_then(|r| r.count)
            .unwrap_or(0);

        if estimate > 0 {
            pending_migrations.push((m.name.clone(), estimate));
        }
    }

    if pending_migrations.is_empty() {
        ctx.print_success("All migrations are complete!");
        return Ok(());
    }

    println!();
    println!(
        "  Found {} migrations with pending work:",
        style(pending_migrations.len()).yellow()
    );
    for (name, count) in &pending_migrations {
        println!("    {} ({} items)", style(name).cyan(), count);
    }
    println!();

    // Run each pending migration
    for (name, _count) in &pending_migrations {
        if dry_run {
            ctx.print_info(&format!("Dry-run: {}", name));
            run_with_progress(ctx, name, true)?;
        } else {
            ctx.print_info(&format!("Running: {}", name));
            run_with_progress(ctx, name, false)?;
        }
        println!();
    }

    ctx.print_success(&format!(
        "{} {} migrations",
        if dry_run { "Validated" } else { "Completed" },
        pending_migrations.len()
    ));

    Ok(())
}

fn run_estimate(ctx: &AppContext, name: &str) -> Result<()> {
    ctx.print_header(&format!("Estimating migration: {}", name));

    let resp = run_migrate_cli(ctx, &["estimate", name])?;

    if let Some(count) = resp.count {
        println!();
        println!("  Items to migrate: {}", style(count).yellow().bold());
        println!();
    }

    Ok(())
}

fn run_dry_run(ctx: &AppContext, name: &str) -> Result<()> {
    ctx.print_header(&format!("Dry-run migration: {}", name));

    run_with_progress(ctx, name, true)
}

fn run_start(ctx: &AppContext, name: &str) -> Result<()> {
    ctx.print_header(&format!("Starting migration: {}", name));

    run_with_progress(ctx, name, false)
}

fn run_with_progress(ctx: &AppContext, name: &str, dry_run: bool) -> Result<()> {
    ensure_migrate_cli_built(ctx)?;

    let bin_path = build_migrate_cli_path(ctx);
    let mut cmd = Command::new(&bin_path);
    cmd.args(["batch", name]);
    if dry_run {
        cmd.arg("--dry-run");
    }
    cmd.stdout(Stdio::piped());

    let mut child = cmd.spawn().context("Failed to spawn migrate_cli")?;
    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);

    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.enable_steady_tick(Duration::from_millis(100));

    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }

        if let Ok(update) = serde_json::from_str::<ProgressUpdate>(&line) {
            match update.update_type.as_str() {
                "init" => {
                    if let Some(total) = update.total {
                        pb.set_length(total as u64);
                    }
                }
                "progress" => {
                    if let Some(completed) = update.completed {
                        pb.set_position(completed as u64);
                    }
                }
                "complete" => {
                    pb.finish_with_message("Done");
                    if let Some(msg) = update.message {
                        ctx.print_success(&msg);
                    }
                }
                "error" => {
                    pb.abandon_with_message("Failed");
                    if let Some(msg) = update.message {
                        ctx.print_warning(&msg);
                    }
                }
                _ => {}
            }
        }
    }

    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("Migration failed");
    }

    Ok(())
}

fn run_status(ctx: &AppContext, name: &str) -> Result<()> {
    ctx.print_header(&format!("Migration status: {}", name));

    let resp = run_migrate_cli(ctx, &["status", name])?;

    if let Some(status) = resp.status {
        println!();
        println!("  Phase: {}", style(&status.phase).cyan());
        println!(
            "  Progress: {}/{} ({:.1}%)",
            status.completed_items,
            status.total_items,
            if status.total_items > 0 {
                (status.completed_items as f64 / status.total_items as f64) * 100.0
            } else {
                0.0
            }
        );
        println!("  Failed: {}", status.failed_items);
        println!("  Skipped: {}", status.skipped_items);
        if let Some(rate) = status.error_rate {
            println!("  Error rate: {:.2}%", rate * 100.0);
        }
        println!();
    } else if let Some(msg) = resp.message {
        ctx.print_info(&msg);
    }

    Ok(())
}

fn run_pause(ctx: &AppContext, name: &str) -> Result<()> {
    ctx.print_header(&format!("Pausing migration: {}", name));

    let resp = run_migrate_cli(ctx, &["pause", name])?;

    if resp.success {
        ctx.print_success("Migration paused");
    } else if let Some(msg) = resp.message {
        ctx.print_warning(&msg);
    }

    Ok(())
}

fn run_resume(ctx: &AppContext, name: &str) -> Result<()> {
    ctx.print_header(&format!("Resuming migration: {}", name));

    run_with_progress(ctx, name, false)
}

fn run_verify(ctx: &AppContext, name: &str) -> Result<()> {
    ctx.print_header(&format!("Verifying migration: {}", name));

    let resp = run_migrate_cli(ctx, &["verify", name])?;

    if resp.success {
        ctx.print_success("Migration verification passed");
    } else if let Some(msg) = resp.message {
        ctx.print_warning(&msg);
    }

    Ok(())
}

fn run_complete(ctx: &AppContext, name: &str) -> Result<()> {
    ctx.print_header(&format!("Completing migration: {}", name));

    let resp = run_migrate_cli(ctx, &["complete", name])?;

    if resp.success {
        ctx.print_success("Migration marked as complete");
    } else if let Some(msg) = resp.message {
        ctx.print_warning(&msg);
    }

    Ok(())
}
