//! Migration Cleanup Check
//!
//! Detects deprecated code that should be deleted after migrations complete.
//! Scans for MIGRATION_DEPRECATED markers and checks workflow completion status.

use anyhow::Result;
use console::style;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;

use crate::context::AppContext;

/// Result of checking for deprecated code
pub struct CleanupCheckResult {
    /// Migrations with deprecated code that should be cleaned up
    pub needs_cleanup: Vec<MigrationCleanup>,
    /// Migrations with deprecated code still in progress
    pub in_progress: Vec<String>,
    /// Total markers found
    pub total_markers: usize,
}

/// Information about a migration that needs cleanup
pub struct MigrationCleanup {
    pub name: String,
    pub locations: Vec<DeprecatedLocation>,
}

/// Location of deprecated code
#[derive(Clone)]
pub struct DeprecatedLocation {
    pub file: String,
    pub line: usize,
    pub context: String,
}

/// Check for deprecated code that should be cleaned up after migrations complete
///
/// Returns Ok(true) if no cleanup needed, Ok(false) if cleanup required
pub fn check_cleanup(ctx: &AppContext, commit_mode: bool) -> Result<bool> {
    ctx.print_header("Migration Cleanup Check");
    println!();

    // Step 1: Find all MIGRATION_DEPRECATED markers
    println!(
        "  {} Scanning for deprecated code markers...",
        style("◌").cyan()
    );

    let markers = find_deprecated_markers(ctx)?;

    if markers.is_empty() {
        println!(
            "  {} No MIGRATION_DEPRECATED markers found",
            style("✓").green()
        );
        return Ok(true);
    }

    let migration_names: HashSet<_> = markers.keys().cloned().collect();
    println!(
        "  Found {} markers for {} migration(s)",
        markers.values().map(|v| v.len()).sum::<usize>(),
        migration_names.len()
    );

    // Step 2: Check workflow completion status
    println!();
    println!(
        "  {} Checking workflow completion status...",
        style("◌").cyan()
    );

    let completed_migrations = get_completed_migrations(ctx)?;

    if completed_migrations.is_empty() {
        println!("  {} No completed migrations in database", style("~").dim());
        println!(
            "  {} (DATABASE_URL not set or no workflows table)",
            style("~").dim()
        );
    } else {
        println!(
            "  Completed migrations: {}",
            completed_migrations
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // Step 3: Identify migrations needing cleanup
    let mut needs_cleanup = Vec::new();
    let mut in_progress = Vec::new();

    for migration_name in &migration_names {
        if completed_migrations.contains(migration_name) {
            needs_cleanup.push(MigrationCleanup {
                name: migration_name.clone(),
                locations: markers.get(migration_name).cloned().unwrap_or_default(),
            });
        } else {
            in_progress.push(migration_name.clone());
        }
    }

    // Report results
    println!();

    if !in_progress.is_empty() {
        println!(
            "  {} Migrations in progress: {}",
            style("~").dim(),
            in_progress.join(", ")
        );
    }

    if needs_cleanup.is_empty() {
        println!(
            "  {} No cleanup needed - deprecated code is for active migrations",
            style("✓").green()
        );
        return Ok(true);
    }

    // We have deprecated code for completed migrations
    println!();
    println!("{}", style("=== CLEANUP REQUIRED ===").red().bold());
    println!();
    println!(
        "The following migrations are {} but still have deprecated code:",
        style("COMPLETED").green()
    );
    println!();

    for cleanup in &needs_cleanup {
        println!("  {} {}", style("✗").red(), style(&cleanup.name).yellow());
        for loc in &cleanup.locations {
            println!("    {}:{}", loc.file, loc.line);
            if !loc.context.is_empty() {
                println!("      {}", style(&loc.context).dim());
            }
        }
        println!();
    }

    println!("{}", style("Action required:").yellow());
    println!("  1. Review the marked code for safe deletion");
    println!("  2. Remove the deprecated code paths");
    println!("  3. Remove feature flags (if any)");
    println!("  4. Update deprecated_code.yaml to remove the migration entry");
    println!();
    println!("  Or use Claude to help clean up:");
    println!("    {}", style("./dev.sh ai lint cleanup --fix").cyan());
    println!();

    if commit_mode {
        println!("{}", style("CI check FAILED.").red().bold());
        Ok(false)
    } else {
        println!(
            "{}",
            style("Dry-run mode - would fail CI with --commit flag.").yellow()
        );
        Ok(true)
    }
}

/// Find all MIGRATION_DEPRECATED markers in the codebase
fn find_deprecated_markers(ctx: &AppContext) -> Result<HashMap<String, Vec<DeprecatedLocation>>> {
    let src_dir = ctx.repo.join("packages/api-core/src");

    if !src_dir.exists() {
        return Ok(HashMap::new());
    }

    let output = Command::new("grep")
        .args([
            "-rn",
            "--include=*.rs",
            "MIGRATION_DEPRECATED:",
            src_dir.to_str().unwrap_or("."),
        ])
        .current_dir(&ctx.repo)
        .output()?;

    let mut markers: HashMap<String, Vec<DeprecatedLocation>> = HashMap::new();

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        // Format: file:line:content
        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() < 3 {
            continue;
        }

        let file = parts[0];
        let line_num: usize = parts[1].parse().unwrap_or(0);
        let content = parts[2];

        // Extract migration name from "MIGRATION_DEPRECATED: name"
        if let Some(start) = content.find("MIGRATION_DEPRECATED:") {
            let after_marker = &content[start + "MIGRATION_DEPRECATED:".len()..];
            let name = after_marker
                .trim()
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches(|c| c == '"' || c == '\'' || c == ',')
                .to_string();

            if !name.is_empty() {
                // Make path relative to repo root
                let relative_file = Path::new(file)
                    .strip_prefix(&ctx.repo)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| file.to_string());

                markers.entry(name).or_default().push(DeprecatedLocation {
                    file: relative_file,
                    line: line_num,
                    context: content.trim().to_string(),
                });
            }
        }
    }

    Ok(markers)
}

/// Get completed migrations from the database
fn get_completed_migrations(_ctx: &AppContext) -> Result<HashSet<String>> {
    let database_url = std::env::var("DATABASE_URL").ok();

    if database_url.is_none() {
        return Ok(HashSet::new());
    }

    let url = database_url.unwrap();

    // Try to query the workflows table
    let output = Command::new("psql")
        .args([
            &url,
            "-t",
            "-A",
            "-c",
            "SELECT name FROM workflows WHERE workflow_type = 'migration' AND phase = 'completed'",
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let names: HashSet<String> = String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            Ok(names)
        }
        _ => {
            // Table might not exist or other error, return empty
            Ok(HashSet::new())
        }
    }
}

/// List all deprecated code markers (for --list-deprecated flag)
pub fn list_deprecated_markers(ctx: &AppContext) -> Result<()> {
    ctx.print_header("Deprecated Code Markers");
    println!();

    let markers = find_deprecated_markers(ctx)?;

    if markers.is_empty() {
        println!(
            "  {} No MIGRATION_DEPRECATED markers found",
            style("✓").green()
        );
        return Ok(());
    }

    let total: usize = markers.values().map(|v| v.len()).sum();

    for (migration, locations) in &markers {
        println!(
            "  {} ({} location(s))",
            style(migration).yellow(),
            locations.len()
        );
        for loc in locations {
            println!("    {}:{}", loc.file, loc.line);
        }
        println!();
    }

    println!(
        "  Total: {} markers for {} migration(s)",
        total,
        markers.len()
    );

    Ok(())
}
