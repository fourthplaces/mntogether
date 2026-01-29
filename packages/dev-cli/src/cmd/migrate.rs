//! Data Migration CLI
//!
//! Manages cursor-based, resumable data migrations that run against the workflow system.
//! For SQL schema migrations, see `cmd/db.rs`.
//!
//! ## Commands
//!
//! - `list` - Show registered migrations
//! - `estimate` - Count items needing migration
//! - `run` - Dry-run (validate without mutations)
//! - `start` - Execute migration (commits changes)
//! - `status` - Check progress
//! - `pause` - Pause running migration
//! - `resume` - Resume paused migration
//! - `verify` - Verify migration integrity
//! - `complete` - Mark as complete

use anyhow::{anyhow, Result};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::cmd_builder::CmdBuilder;
use crate::config::Config;
use crate::context::AppContext;
use crate::utils::cmd_exists;
use crate::MigrateAction;

// =============================================================================
// Entry Point
// =============================================================================

/// Handle data migration commands
pub fn data_migrate(
    ctx: &AppContext,
    config: Option<&Config>,
    action: MigrateAction,
) -> Result<()> {
    match action {
        MigrateAction::List => list_migrations(ctx, config),
        MigrateAction::Estimate { name, env } => estimate_migration(ctx, config, &name, &env),
        MigrateAction::Run {
            name,
            env,
            batch_size,
        } => run_migration(ctx, config, &name, &env, batch_size, true),
        MigrateAction::Start {
            name,
            env,
            error_budget,
            batch_size,
        } => start_migration(ctx, config, &name, &env, error_budget, batch_size),
        MigrateAction::Status { name, env } => show_status(ctx, config, &name, &env),
        MigrateAction::Pause { name, env } => pause_migration(ctx, config, &name, &env),
        MigrateAction::Resume { name, env } => resume_migration(ctx, config, &name, &env),
        MigrateAction::Verify { name, env } => verify_migration(ctx, config, &name, &env),
        MigrateAction::Complete { name, env } => complete_migration(ctx, config, &name, &env),
    }
}

// =============================================================================
// Workflow Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStatus {
    pub id: String,
    pub name: String,
    pub phase: String,
    pub total_items: Option<i64>,
    pub completed_items: i64,
    pub failed_items: i64,
    pub skipped_items: i64,
    pub dry_run: bool,
    pub started_at: Option<String>,
    pub paused_at: Option<String>,
    pub completed_at: Option<String>,
}

impl WorkflowStatus {
    fn progress_pct(&self) -> f64 {
        match self.total_items {
            Some(total) if total > 0 => {
                (self.completed_items + self.failed_items + self.skipped_items) as f64
                    / total as f64
                    * 100.0
            }
            _ => 0.0,
        }
    }

    fn error_rate(&self) -> f64 {
        let processed = self.completed_items + self.failed_items + self.skipped_items;
        if processed > 0 {
            self.failed_items as f64 / processed as f64
        } else {
            0.0
        }
    }
}

// =============================================================================
// Commands
// =============================================================================

/// List all registered data migrations
fn list_migrations(ctx: &AppContext, config: Option<&Config>) -> Result<()> {
    ctx.print_header("Registered Data Migrations");
    println!();

    // Read migrations from data_migrations directory
    let api_core_path = config
        .map(|c| {
            c.get_package("api-core")
                .map(|p| p.path.clone())
                .unwrap_or_else(|| ctx.repo.join("packages/api-core"))
        })
        .unwrap_or_else(|| ctx.repo.join("packages/api-core"));

    let migrations_dir = api_core_path.join("src/data_migrations");

    if !migrations_dir.exists() {
        println!("  {} No data_migrations directory found", style("~").dim());
        return Ok(());
    }

    let mut migrations: Vec<String> = std::fs::read_dir(&migrations_dir)?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with(".rs") && name != "mod.rs" {
                Some(name.trim_end_matches(".rs").to_string())
            } else {
                None
            }
        })
        .collect();

    migrations.sort();

    if migrations.is_empty() {
        println!("  {} No migrations registered", style("~").dim());
        println!();
        println!("  To create a migration:");
        println!("    1. Create src/data_migrations/my_migration.rs");
        println!("    2. Implement the DataMigration trait");
        println!("    3. Register in src/data_migrations/mod.rs");
        return Ok(());
    }

    for name in &migrations {
        println!("  {} {}", style("•").cyan(), name);
    }

    println!();
    println!(
        "  {} migration(s) registered",
        style(migrations.len()).bold()
    );
    println!();
    println!(
        "  Use {} to check status",
        style("./dev.sh migrate status <name> --env <env>").cyan()
    );

    Ok(())
}

/// Estimate items needing migration
fn estimate_migration(
    ctx: &AppContext,
    config: Option<&Config>,
    name: &str,
    env: &str,
) -> Result<()> {
    ctx.print_header(&format!("Estimate: {} ({})", name, env));
    println!();

    let spinner = create_spinner("Counting items...");

    let result = run_api_command(ctx, config, env, &format!("migrate estimate {}", name))?;

    spinner.finish_and_clear();

    // Parse result
    let count: i64 = result.trim().parse().unwrap_or(0);

    println!("  {} items need migration", style(count).bold().cyan());

    if count == 0 {
        println!();
        println!("  {} Migration may already be complete", style("ℹ").blue());
        println!(
            "    Run {} to verify",
            style(format!("./dev.sh migrate verify {} --env {}", name, env)).cyan()
        );
    }

    Ok(())
}

/// Run migration in dry-run mode
fn run_migration(
    ctx: &AppContext,
    config: Option<&Config>,
    name: &str,
    env: &str,
    batch_size: i64,
    dry_run: bool,
) -> Result<()> {
    let mode = if dry_run { "Dry-run" } else { "Run" };
    ctx.print_header(&format!("{}: {} ({})", mode, name, env));
    println!();

    // First estimate
    let spinner = create_spinner("Estimating...");
    let estimate_result = run_api_command(ctx, config, env, &format!("migrate estimate {}", name))?;
    spinner.finish_and_clear();

    let total: i64 = estimate_result.trim().parse().unwrap_or(0);

    if total == 0 {
        println!("  {} No items to migrate", style("✓").green());
        return Ok(());
    }

    println!("  Found {} items", style(total).bold());
    println!();

    // Create or get workflow
    let spinner = create_spinner("Initializing workflow...");
    let _workflow = run_api_command(
        ctx,
        config,
        env,
        &format!(
            "migrate init {} --dry-run={} --batch-size={}",
            name, dry_run, batch_size
        ),
    )?;
    spinner.finish_and_clear();

    // Create progress bar
    let pb = create_progress_bar(total as u64, dry_run);

    // Process batches
    let mut processed: i64 = 0;
    let mut failed: i64 = 0;

    loop {
        let result = run_api_command(
            ctx,
            config,
            env,
            &format!("migrate batch {} --batch-size={}", name, batch_size),
        )?;

        // Parse batch result: "processed:N,failed:M,done:bool"
        let parts: Vec<&str> = result.trim().split(',').collect();
        let batch_processed: i64 = parts
            .get(0)
            .and_then(|s| s.strip_prefix("processed:"))
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let batch_failed: i64 = parts
            .get(1)
            .and_then(|s| s.strip_prefix("failed:"))
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let done = parts.get(2).map(|s| s.contains("true")).unwrap_or(false);

        processed += batch_processed;
        failed += batch_failed;

        pb.set_position(processed as u64);

        if done || batch_processed == 0 {
            break;
        }
    }

    pb.finish_and_clear();

    // Summary
    println!();
    if dry_run {
        println!(
            "  {} Dry-run complete: {} would be migrated, {} would fail",
            style("✓").green(),
            style(processed - failed).bold(),
            style(failed).yellow()
        );
    } else {
        println!(
            "  {} Migration complete: {} migrated, {} failed",
            style("✓").green(),
            style(processed - failed).bold(),
            style(failed).yellow()
        );
    }

    if failed > 0 {
        println!();
        println!(
            "  {} Review failed items with: {}",
            style("!").yellow(),
            style(format!("./dev.sh migrate status {} --env {}", name, env)).cyan()
        );
    }

    Ok(())
}

/// Start migration (commit mode) with safety confirmations
fn start_migration(
    ctx: &AppContext,
    config: Option<&Config>,
    name: &str,
    env: &str,
    error_budget: f64,
    batch_size: i64,
) -> Result<()> {
    ctx.print_header(&format!("Start Migration: {} ({})", name, env));
    println!();

    // Safety confirmation for prod
    if env == "prod" && !ctx.quiet {
        println!(
            "{}",
            style("⚠️  WARNING: This will modify PRODUCTION data")
                .red()
                .bold()
        );
        println!();
        println!("  Migration: {}", style(name).yellow());
        println!(
            "  Error budget: {}%",
            style(format!("{:.1}", error_budget * 100.0)).yellow()
        );
        println!();

        if !ctx.confirm("Are you ABSOLUTELY sure you want to continue?", false)? {
            println!("Migration cancelled.");
            return Ok(());
        }

        // Double confirmation
        println!();
        let confirm_text: String = dialoguer::Input::with_theme(&ctx.theme())
            .with_prompt(&format!("Type 'migrate {}' to confirm", name))
            .interact_text()?;

        if confirm_text.trim() != format!("migrate {}", name) {
            println!("Confirmation failed. Migration cancelled.");
            return Ok(());
        }
        println!();
    }

    // First check if there's an existing workflow
    let status = get_workflow_status(ctx, config, name, env)?;

    if let Some(wf) = status {
        match wf.phase.as_str() {
            "running" => {
                println!("  {} Migration already running", style("ℹ").blue());
                println!("    Progress: {:.1}%", wf.progress_pct());
                println!();
                println!(
                    "  Use {} to monitor",
                    style(format!("./dev.sh migrate status {} --env {}", name, env)).cyan()
                );
                return Ok(());
            }
            "paused" => {
                println!("  {} Migration is paused", style("ℹ").blue());
                println!("    Progress: {:.1}%", wf.progress_pct());
                println!();
                if ctx.confirm("Resume the existing migration?", true)? {
                    return resume_migration(ctx, config, name, env);
                }
                return Ok(());
            }
            "completed" => {
                println!("  {} Migration already completed", style("✓").green());
                return Ok(());
            }
            _ => {}
        }
    }

    // Run the actual migration
    run_migration(ctx, config, name, env, batch_size, false)?;

    // If we got here, prompt for verify
    println!();
    if ctx.confirm("Run verification?", true)? {
        verify_migration(ctx, config, name, env)?;
    }

    Ok(())
}

/// Show migration status
fn show_status(ctx: &AppContext, config: Option<&Config>, name: &str, env: &str) -> Result<()> {
    ctx.print_header(&format!("Status: {} ({})", name, env));
    println!();

    let status = get_workflow_status(ctx, config, name, env)?;

    match status {
        None => {
            println!("  {} No workflow found for '{}'", style("~").dim(), name);
            println!();
            println!("  Start a migration with:");
            println!(
                "    {}",
                style(format!("./dev.sh migrate start {} --env {}", name, env)).cyan()
            );
        }
        Some(wf) => {
            // Phase indicator
            let phase_style = match wf.phase.as_str() {
                "running" => style(&wf.phase).green().bold(),
                "paused" => style(&wf.phase).yellow().bold(),
                "completed" => style(&wf.phase).cyan().bold(),
                "failed" => style(&wf.phase).red().bold(),
                _ => style(&wf.phase).dim(),
            };

            println!("  Phase: {}", phase_style);

            if let Some(total) = wf.total_items {
                println!();
                println!("  Progress:");

                // Progress bar
                let processed = wf.completed_items + wf.failed_items + wf.skipped_items;
                let pct = if total > 0 {
                    processed * 100 / total
                } else {
                    0
                };
                let bar_width = 40;
                let filled = (pct as usize * bar_width) / 100;
                let empty = bar_width - filled;

                println!(
                    "    [{}{}] {:.1}%",
                    style("█".repeat(filled)).green(),
                    style("░".repeat(empty)).dim(),
                    wf.progress_pct()
                );

                println!();
                println!("    Total:     {}", total);
                println!(
                    "    Completed: {} {}",
                    wf.completed_items,
                    style("✓").green()
                );
                println!("    Skipped:   {} {}", wf.skipped_items, style("~").dim());
                println!(
                    "    Failed:    {} {}",
                    wf.failed_items,
                    if wf.failed_items > 0 {
                        style("✗").red()
                    } else {
                        style("✓").green()
                    }
                );

                if wf.failed_items > 0 {
                    println!();
                    println!("    Error rate: {:.2}%", wf.error_rate() * 100.0);
                }
            }

            if wf.dry_run {
                println!();
                println!(
                    "  {} This was a dry-run (no data modified)",
                    style("ℹ").blue()
                );
            }

            // Timestamps
            println!();
            if let Some(started) = &wf.started_at {
                println!("  Started: {}", started);
            }
            if let Some(paused) = &wf.paused_at {
                println!("  Paused: {}", paused);
            }
            if let Some(completed) = &wf.completed_at {
                println!("  Completed: {}", completed);
            }

            // Next steps
            println!();
            match wf.phase.as_str() {
                "running" => {
                    println!("  {}", style("Migration in progress...").dim());
                }
                "paused" => {
                    println!(
                        "  Resume with: {}",
                        style(format!("./dev.sh migrate resume {} --env {}", name, env)).cyan()
                    );
                }
                "verifying" | "completed" => {
                    println!(
                        "  Verify with: {}",
                        style(format!("./dev.sh migrate verify {} --env {}", name, env)).cyan()
                    );
                }
                _ => {}
            }
        }
    }

    Ok(())
}

/// Pause a running migration
fn pause_migration(ctx: &AppContext, config: Option<&Config>, name: &str, env: &str) -> Result<()> {
    ctx.print_header(&format!("Pause: {} ({})", name, env));
    println!();

    let spinner = create_spinner("Pausing migration...");

    run_api_command(ctx, config, env, &format!("migrate pause {}", name))?;

    spinner.finish_and_clear();

    println!("  {} Migration paused", style("✓").green());
    println!();
    println!(
        "  Resume with: {}",
        style(format!("./dev.sh migrate resume {} --env {}", name, env)).cyan()
    );

    Ok(())
}

/// Resume a paused migration
fn resume_migration(
    ctx: &AppContext,
    config: Option<&Config>,
    name: &str,
    env: &str,
) -> Result<()> {
    ctx.print_header(&format!("Resume: {} ({})", name, env));
    println!();

    let spinner = create_spinner("Resuming migration...");

    run_api_command(ctx, config, env, &format!("migrate resume {}", name))?;

    spinner.finish_and_clear();

    println!("  {} Migration resumed", style("✓").green());

    // Show status
    println!();
    show_status(ctx, config, name, env)?;

    Ok(())
}

/// Verify migration integrity
fn verify_migration(
    ctx: &AppContext,
    config: Option<&Config>,
    name: &str,
    env: &str,
) -> Result<()> {
    ctx.print_header(&format!("Verify: {} ({})", name, env));
    println!();

    let spinner = create_spinner("Running verification...");

    let result = run_api_command(ctx, config, env, &format!("migrate verify {}", name))?;

    spinner.finish_and_clear();

    // Parse result: "passed" | "incomplete:N" | "failed:reason"
    let result = result.trim();

    if result == "passed" {
        println!("  {} Verification passed", style("✓").green());
        println!();
        println!(
            "  Mark complete with: {}",
            style(format!("./dev.sh migrate complete {} --env {}", name, env)).cyan()
        );
    } else if let Some(remaining) = result.strip_prefix("incomplete:") {
        let count: i64 = remaining.parse().unwrap_or(0);
        println!("  {} Verification incomplete", style("⚠").yellow());
        println!("    {} items still need migration", style(count).bold());
    } else if let Some(reason) = result.strip_prefix("failed:") {
        println!("  {} Verification failed", style("✗").red());
        println!("    {}", reason);
    } else {
        println!("  {} Unknown result: {}", style("?").yellow(), result);
    }

    Ok(())
}

/// Mark migration as complete
fn complete_migration(
    ctx: &AppContext,
    config: Option<&Config>,
    name: &str,
    env: &str,
) -> Result<()> {
    ctx.print_header(&format!("Complete: {} ({})", name, env));
    println!();

    // Verify first
    println!("  Running verification first...");
    println!();

    let verify_result = run_api_command(ctx, config, env, &format!("migrate verify {}", name))?;

    if verify_result.trim() != "passed" {
        println!(
            "  {} Cannot mark complete - verification failed",
            style("✗").red()
        );
        println!();
        println!(
            "  Run {} to see details",
            style(format!("./dev.sh migrate verify {} --env {}", name, env)).cyan()
        );
        return Ok(());
    }

    // Confirm
    if env == "prod" && !ctx.quiet {
        println!(
            "  {} This will mark the migration as complete.",
            style("ℹ").blue()
        );
        println!("    After completion, the migration code should be deleted.");
        println!();

        if !ctx.confirm("Mark migration as complete?", true)? {
            println!("Cancelled.");
            return Ok(());
        }
        println!();
    }

    let spinner = create_spinner("Marking complete...");

    run_api_command(ctx, config, env, &format!("migrate complete {}", name))?;

    spinner.finish_and_clear();

    println!("  {} Migration marked as complete", style("✓").green());
    println!();
    println!("{}", style("Next steps:").bold());
    println!("  1. Remove the migration from src/data_migrations/mod.rs");
    println!(
        "  2. Delete the migration file: src/data_migrations/{}.rs",
        name
    );
    println!(
        "  3. Run {} to check for deprecated code",
        style("./dev.sh ai lint cleanup").cyan()
    );

    Ok(())
}

// =============================================================================
// Helpers
// =============================================================================

/// Get workflow status from the API
fn get_workflow_status(
    ctx: &AppContext,
    config: Option<&Config>,
    name: &str,
    env: &str,
) -> Result<Option<WorkflowStatus>> {
    let result = run_api_command(ctx, config, env, &format!("migrate status {}", name));

    match result {
        Ok(output) => {
            let output = output.trim();
            if output.is_empty() || output == "null" || output.starts_with("not_found") {
                Ok(None)
            } else {
                // Parse JSON status
                serde_json::from_str(output).map(Some).map_err(|e| {
                    anyhow!("Failed to parse workflow status: {} - raw: {}", e, output)
                })
            }
        }
        Err(_) => Ok(None),
    }
}

/// Run a command against the API server
fn run_api_command(
    ctx: &AppContext,
    config: Option<&Config>,
    env: &str,
    command: &str,
) -> Result<String> {
    // For local development, run via cargo
    if env == "local" || env == "dev" {
        // api-cli binary is in api-server package, not api-core
        let api_server_path = config
            .map(|c| {
                c.get_package("api-server")
                    .map(|p| p.path.clone())
                    .unwrap_or_else(|| ctx.repo.join("packages/api-server"))
            })
            .unwrap_or_else(|| ctx.repo.join("packages/api-server"));

        // Check if we can connect to local postgres
        let db_url = std::env::var("DATABASE_URL").ok();
        if db_url.is_none() {
            // Try to load from .env.dev
            let env_file = ctx.repo.join(".env.dev");
            if env_file.exists() {
                let content = std::fs::read_to_string(&env_file)?;
                if let Some(line) = content.lines().find(|l| l.starts_with("DATABASE_URL=")) {
                    let url = line
                        .trim_start_matches("DATABASE_URL=")
                        .trim_matches('"')
                        .trim_matches('\'');
                    std::env::set_var("DATABASE_URL", url);
                }
            }
        }

        let output = std::process::Command::new("cargo")
            .args(["run", "--bin", "api-cli", "--", "migrate"])
            .args(command.split_whitespace())
            .current_dir(&api_server_path)
            .env("RUST_LOG", "warn")
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("API command failed: {}", stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        // For remote environments, use ESC to run the command
        if !cmd_exists("esc") {
            return Err(anyhow!(
                "ESC CLI not found. Install from: https://www.pulumi.com/docs/esc-cli/"
            ));
        }

        let esc_env = config
            .map(|c| c.esc_path(env))
            .unwrap_or_else(|| format!("shaya/service/{}", env));

        let output = CmdBuilder::new("esc")
            .args(["run", &esc_env, "--", "api-cli", "migrate"])
            .args(command.split_whitespace().collect::<Vec<_>>())
            .cwd(&ctx.repo)
            .run_capture()?;

        Ok(output.stdout_string())
    }
}

/// Create a spinner for async operations
fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Create a progress bar for batch processing
fn create_progress_bar(total: u64, dry_run: bool) -> ProgressBar {
    let pb = ProgressBar::new(total);
    let template = if dry_run {
        "{spinner:.cyan} [{bar:40.cyan/dim}] {pos}/{len} (dry-run) {msg}"
    } else {
        "{spinner:.green} [{bar:40.green/dim}] {pos}/{len} {msg}"
    };
    pb.set_style(
        ProgressStyle::default_bar()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template(template)
            .unwrap()
            .progress_chars("█▓░"),
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}
