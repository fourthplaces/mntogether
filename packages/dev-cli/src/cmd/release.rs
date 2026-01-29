//! Release management with unified v* tags

use anyhow::{anyhow, Context, Result};
use console::style;
use dialoguer::Select;

use crate::cmd::ai_lint::rules::{check_migrations, check_test_coverage_ai};
use crate::cmd_builder::CmdBuilder;
use crate::config::Config;
use crate::context::AppContext;
use crate::utils::cmd_exists;

// =============================================================================
// Version
// =============================================================================

#[derive(Debug, Clone)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub prerelease: Option<String>,
}

impl Version {
    pub fn parse(s: &str) -> Result<Self> {
        // Strip v prefix (e.g., "v1.0.0" -> "1.0.0")
        let version_part = s.trim_start_matches('v');

        let (version_part, prerelease) = if let Some(idx) = version_part.find('-') {
            (
                &version_part[..idx],
                Some(version_part[idx + 1..].to_string()),
            )
        } else {
            (version_part, None)
        };

        let parts: Vec<&str> = version_part.split('.').collect();
        if parts.len() != 3 {
            return Err(anyhow!("Invalid version format: {}", s));
        }

        Ok(Version {
            major: parts[0].parse().context("Invalid major version")?,
            minor: parts[1].parse().context("Invalid minor version")?,
            patch: parts[2].parse().context("Invalid patch version")?,
            prerelease,
        })
    }

    pub fn to_tag(&self) -> String {
        let base = format!("v{}.{}.{}", self.major, self.minor, self.patch);
        if let Some(ref pre) = self.prerelease {
            format!("{}-{}", base, pre)
        } else {
            base
        }
    }

    pub fn bump_major(&self) -> Self {
        Version {
            major: self.major + 1,
            minor: 0,
            patch: 0,
            prerelease: None,
        }
    }

    pub fn bump_minor(&self) -> Self {
        Version {
            major: self.major,
            minor: self.minor + 1,
            patch: 0,
            prerelease: None,
        }
    }

    pub fn bump_patch(&self) -> Self {
        Version {
            major: self.major,
            minor: self.minor,
            patch: self.patch + 1,
            prerelease: None,
        }
    }

    pub fn hotfix(&self) -> Self {
        let hotfix_num = if let Some(ref pre) = self.prerelease {
            if pre.starts_with("hotfix.") {
                pre.trim_start_matches("hotfix.").parse().unwrap_or(0) + 1
            } else {
                1
            }
        } else {
            1
        };
        Version {
            major: self.major,
            minor: self.minor,
            patch: self.patch,
            prerelease: Some(format!("hotfix.{}", hotfix_num)),
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(ref pre) = self.prerelease {
            write!(f, "-{}", pre)?;
        }
        Ok(())
    }
}

// =============================================================================
// Version Discovery
// =============================================================================

/// Get the current version from the latest v* tag
pub fn get_current_version(ctx: &AppContext) -> Result<Option<Version>> {
    let result = CmdBuilder::new("git")
        .args(["describe", "--tags", "--abbrev=0", "--match", "v*"])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture();

    match result {
        Ok(output) if output.code == 0 => {
            let tag = output.stdout_string().trim().to_string();
            if tag.is_empty() {
                Ok(None)
            } else {
                Ok(Some(Version::parse(&tag)?))
            }
        }
        _ => Ok(None),
    }
}

/// Get list of recent version tags
pub fn get_recent_versions(ctx: &AppContext, count: u32) -> Result<Vec<String>> {
    let result = CmdBuilder::new("git")
        .args(["tag", "-l", "v*", "--sort=-version:refname"])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()?;

    Ok(result
        .stdout_string()
        .lines()
        .take(count as usize)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

// =============================================================================
// Release Commands
// =============================================================================

/// Release with specified bump type: ./dev.sh release [patch|minor|major|hotfix]
pub fn release(ctx: &AppContext, config: &Config, bump: Option<&str>, dry_run: bool) -> Result<()> {
    let current = get_current_version(ctx)?;

    // Run pre-flight checks
    run_preflight_checks(ctx, config)?;

    // Determine bump type
    let bump_type = match bump {
        Some(b) => b.to_string(),
        None => select_bump_type(ctx, current.as_ref())?,
    };

    let new_version = calculate_new_version(current.as_ref(), &bump_type)?;
    let tag = new_version.to_tag();

    ctx.print_header("Release Summary");
    println!();

    let current_str = current
        .as_ref()
        .map(|v| v.to_tag())
        .unwrap_or_else(|| "none".to_string());

    println!(
        "  {} → {}",
        style(&current_str).dim(),
        style(&tag).green().bold()
    );
    println!();

    // Show what's changed since last release
    if let Some(ref curr) = current {
        show_release_diff(ctx, &curr.to_tag())?;
    }

    if dry_run {
        println!("{}", style("Dry run - no changes made").yellow());
        return Ok(());
    }

    if !ctx.confirm("Proceed with release?", true)? {
        println!("Cancelled.");
        return Ok(());
    }

    // Create and push tag
    create_release(ctx, &tag)?;

    ctx.print_success(&format!("Released {}!", tag));
    println!();
    println!(
        "  {} GitHub Actions will deploy to production",
        style("→").dim()
    );

    Ok(())
}

/// Interactive release selection
pub fn release_interactive(ctx: &AppContext, config: &Config, dry_run: bool) -> Result<()> {
    release(ctx, config, None, dry_run)
}

/// Release with specific bump type
pub fn release_packages(
    ctx: &AppContext,
    config: &Config,
    _targets: &[String], // Ignored - unified versioning
    bump: Option<&str>,
    dry_run: bool,
) -> Result<()> {
    release(ctx, config, bump, dry_run)
}

fn calculate_new_version(current: Option<&Version>, bump: &str) -> Result<Version> {
    let base = current.cloned().unwrap_or(Version {
        major: 0,
        minor: 0,
        patch: 0,
        prerelease: None,
    });

    Ok(match bump {
        "patch" => base.bump_patch(),
        "minor" => base.bump_minor(),
        "major" => base.bump_major(),
        "hotfix" => base.hotfix(),
        _ => return Err(anyhow!("Invalid bump type: {}", bump)),
    })
}

fn select_bump_type(ctx: &AppContext, current: Option<&Version>) -> Result<String> {
    let current_str = current
        .map(|v| v.to_tag())
        .unwrap_or_else(|| "v0.0.0".to_string());

    let current_v = current.cloned().unwrap_or(Version {
        major: 0,
        minor: 0,
        patch: 0,
        prerelease: None,
    });

    let options = vec![
        format!(
            "patch  ({} → {})",
            current_str,
            current_v.bump_patch().to_tag()
        ),
        format!(
            "minor  ({} → {})",
            current_str,
            current_v.bump_minor().to_tag()
        ),
        format!(
            "major  ({} → {})",
            current_str,
            current_v.bump_major().to_tag()
        ),
        format!("hotfix ({} → {})", current_str, current_v.hotfix().to_tag()),
    ];

    let choice = Select::with_theme(&ctx.theme())
        .with_prompt("Bump type")
        .items(&options)
        .default(0)
        .interact()?;

    Ok(match choice {
        0 => "patch",
        1 => "minor",
        2 => "major",
        3 => "hotfix",
        _ => "patch",
    }
    .to_string())
}

fn create_release(ctx: &AppContext, tag: &str) -> Result<()> {
    println!();
    println!("Creating release {}...", style(tag).green());

    // Create annotated tag
    let tag_message = format!("Release {}", tag);
    CmdBuilder::new("git")
        .args(["tag", "-a", tag, "-m", &tag_message])
        .cwd(&ctx.repo)
        .run()?;

    // Push tag
    CmdBuilder::new("git")
        .args(["push", "origin", tag])
        .cwd(&ctx.repo)
        .run()?;

    println!("  {} Tag {} pushed", style("✓").green(), tag);

    Ok(())
}

// =============================================================================
// Pre-flight Checks
// =============================================================================

fn run_preflight_checks(ctx: &AppContext, config: &Config) -> Result<()> {
    println!("Pre-release checks:");
    println!();

    let mut all_passed = true;

    // Check 0: Test coverage for changed files (AI-powered)
    let changed_files = get_changed_source_files(ctx)?;
    if !changed_files.is_empty() {
        match check_test_coverage_ai(ctx, &changed_files, "main") {
            Ok(true) => {} // Already printed success
            Ok(false) => {
                all_passed = false;
            }
            Err(e) => {
                println!("  {} Test coverage check error: {}", style("?").yellow(), e);
            }
        }
    } else {
        println!("  {} No source files changed", style("✓").green());
    }

    // Check 1: Migration safety (AI-powered)
    let migrations = get_changed_migrations(ctx, config)?;
    if !migrations.is_empty() {
        match check_migrations(ctx, &migrations) {
            Ok(true) => {} // Already printed success
            Ok(false) => {
                all_passed = false;
            }
            Err(e) => {
                println!("  {} AI migration check error: {}", style("?").yellow(), e);
            }
        }
    } else {
        println!("  {} No new migrations to check", style("✓").green());
    }

    // Check 2: On main branch
    let branch = CmdBuilder::new("git")
        .args(["branch", "--show-current"])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()?;

    let current_branch = branch.stdout_string().trim().to_string();
    let on_release_branch = current_branch == "main" || current_branch == "master";

    if on_release_branch {
        println!("  {} On {} branch", style("✓").green(), current_branch);
    } else {
        println!(
            "  {} On {} branch (expected main)",
            style("✗").red(),
            current_branch
        );
        all_passed = false;
    }

    // Check 3: Working tree clean
    let status = CmdBuilder::new("git")
        .args(["status", "--porcelain"])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()?;

    if status.stdout_string().trim().is_empty() {
        println!("  {} Working tree clean", style("✓").green());
    } else {
        println!("  {} Uncommitted changes", style("✗").red());
        all_passed = false;
    }

    // Check 4: Up to date with remote
    let _ = CmdBuilder::new("git")
        .args(["fetch", "origin"])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture();

    let behind = CmdBuilder::new("git")
        .args(["rev-list", "--count", "HEAD..@{u}"])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture();

    match behind {
        Ok(output) if output.code == 0 => {
            let count: i32 = output.stdout_string().trim().parse().unwrap_or(0);
            if count == 0 {
                println!("  {} Up to date with remote", style("✓").green());
            } else {
                println!(
                    "  {} Behind remote by {} commit(s)",
                    style("✗").red(),
                    count
                );
                all_passed = false;
            }
        }
        _ => println!("  {} Could not check remote status", style("?").yellow()),
    }

    // Check 5: CI status
    if cmd_exists("gh") {
        let ci_status = CmdBuilder::new("gh")
            .args(["run", "list", "--limit", "1", "--json", "status,conclusion"])
            .cwd(&ctx.repo)
            .capture_stdout()
            .run_capture();

        match ci_status {
            Ok(output) if output.code == 0 => {
                let stdout = output.stdout_string();
                if stdout.contains("\"conclusion\":\"success\"") {
                    println!("  {} CI passed", style("✓").green());
                } else if stdout.contains("\"status\":\"in_progress\"") {
                    println!("  {} CI in progress", style("~").yellow());
                } else {
                    println!("  {} CI not passing", style("✗").red());
                    all_passed = false;
                }
            }
            _ => println!("  {} Could not check CI status", style("?").yellow()),
        }
    }

    println!();

    if !all_passed && !ctx.confirm("Some checks failed. Continue anyway?", false)? {
        return Err(anyhow!("Pre-release checks failed"));
    }

    Ok(())
}

// =============================================================================
// Changed Files Helpers
// =============================================================================

/// Get source files changed since the last release
fn get_changed_source_files(ctx: &AppContext) -> Result<Vec<String>> {
    let current = get_current_version(ctx)?;
    let last_tag = current.map(|v| v.to_tag());

    let compare_ref = match last_tag {
        Some(tag) => format!("{}..HEAD", tag),
        None => "HEAD~10".to_string(), // No previous release, check last 10 commits
    };

    // Get files added or modified since last tag
    let diff_result = CmdBuilder::new("git")
        .args([
            "diff",
            "--name-only",
            "--diff-filter=AM", // Added or Modified
            &compare_ref,
        ])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()?;

    Ok(diff_result
        .stdout_string()
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

/// Get migration files changed since the last release
fn get_changed_migrations(ctx: &AppContext, config: &Config) -> Result<Vec<String>> {
    // Find migrations path from config
    let db_packages = config.database_packages();
    if db_packages.is_empty() {
        return Ok(vec![]);
    }

    let (pkg_name, _) = db_packages.first().unwrap();
    let migrations_path = config
        .migrations_path(pkg_name)
        .ok_or_else(|| anyhow!("No migrations path found for {}", pkg_name))?;

    // Get relative path for git commands
    let migrations_rel = migrations_path
        .strip_prefix(&config.repo_root)
        .unwrap_or(&migrations_path);
    let migrations_glob = format!("{}/*.sql", migrations_rel.display());

    let current = get_current_version(ctx)?;
    let last_tag = current.map(|v| v.to_tag());

    if last_tag.is_none() {
        // No previous release - check all migrations (but limit for sanity)
        let files_result = CmdBuilder::new("git")
            .args(["ls-files", &migrations_glob])
            .cwd(&ctx.repo)
            .capture_stdout()
            .run_capture()?;

        return Ok(files_result
            .stdout_string()
            .lines()
            .take(20) // Limit to avoid overwhelming AI
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect());
    }

    // Get migrations added or modified since last tag
    let diff_result = CmdBuilder::new("git")
        .args([
            "diff",
            "--name-only",
            "--diff-filter=AM", // Added or Modified
            &format!("{}..HEAD", last_tag.unwrap()),
            "--",
            &migrations_glob,
        ])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()?;

    Ok(diff_result
        .stdout_string()
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

// =============================================================================
// Diff Display
// =============================================================================

/// Show what's changed since the last release
fn show_release_diff(ctx: &AppContext, from_tag: &str) -> Result<()> {
    println!();
    ctx.print_header(&format!("Changes since {}", from_tag));
    println!();

    // Get commit summary
    let log_result = CmdBuilder::new("git")
        .args([
            "log",
            &format!("{}..HEAD", from_tag),
            "--oneline",
            "--no-decorate",
        ])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()?;

    let commits = log_result.stdout_string();
    let commit_lines: Vec<&str> = commits.lines().collect();

    if commit_lines.is_empty() {
        println!("  {}", style("No commits since last release").dim());
        return Ok(());
    }

    println!(
        "  {} commit(s) since last release:\n",
        style(commit_lines.len()).cyan().bold()
    );

    // Show commits (limit to 20)
    for (i, line) in commit_lines.iter().take(20).enumerate() {
        println!("    {}", line);
        if i == 19 && commit_lines.len() > 20 {
            println!(
                "    {} ... and {} more",
                style("...").dim(),
                commit_lines.len() - 20
            );
        }
    }

    println!();
    Ok(())
}

// =============================================================================
// Utility Commands
// =============================================================================

/// List releasable packages (kept for compatibility, but simplified)
pub fn list_packages(ctx: &AppContext, _config: &Config) -> Result<()> {
    ctx.print_header("Release Info");

    let current = get_current_version(ctx)?;
    let version_str = current
        .map(|v| v.to_tag())
        .unwrap_or_else(|| "none".to_string());

    println!("  Current version: {}", style(version_str).cyan());
    println!();
    println!("  Unified v* tags are used for all releases.");
    println!(
        "  Run {} to create a release.",
        style("./dev.sh release").green()
    );

    Ok(())
}

/// List recent releases
pub fn list_releases(ctx: &AppContext, _config: &Config, count: u32) -> Result<()> {
    ctx.print_header("Recent Releases");

    let tags = get_recent_versions(ctx, count)?;

    if tags.is_empty() {
        println!("  (no releases)");
    } else {
        for (i, tag) in tags.iter().enumerate() {
            if i == 0 {
                println!("  {} (latest)", style(tag).green());
            } else {
                println!("  {}", tag);
            }
        }
    }

    Ok(())
}

// =============================================================================
// Rollback
// =============================================================================

/// Rollback to a previous version
pub fn rollback(ctx: &AppContext, version: Option<&str>, env: Option<&str>) -> Result<()> {
    // Ensure gh CLI is available
    if !crate::utils::cmd_exists("gh") {
        return Err(anyhow!(
            "GitHub CLI (gh) is required for rollback. Install it with: brew install gh"
        ));
    }

    // Determine environment
    let target_env = match env {
        Some(e) => e.to_string(),
        None => select_environment(ctx)?,
    };

    // Determine version to rollback to
    let target_version = match version {
        Some(v) => {
            // Validate the version exists as a tag
            let tags = get_recent_versions(ctx, 50)?;
            let normalized = if v.starts_with('v') {
                v.to_string()
            } else {
                format!("v{}", v)
            };
            if !tags.contains(&normalized) {
                return Err(anyhow!(
                    "Version {} not found. Run './dev.sh release list' to see available versions.",
                    normalized
                ));
            }
            normalized
        }
        None => select_rollback_version(ctx)?,
    };

    // Get current version for display
    let current = get_current_version(ctx)?;
    let current_str = current
        .map(|v| v.to_tag())
        .unwrap_or_else(|| "unknown".to_string());

    ctx.print_header("Rollback Summary");
    println!();
    println!(
        "  {} {} → {}",
        style("Environment:").dim(),
        style(&target_env).cyan(),
        style(&target_env).cyan()
    );
    println!(
        "  {} {} → {}",
        style("Version:").dim(),
        style(&current_str).red(),
        style(&target_version).green().bold()
    );
    println!();

    if target_version == current_str {
        println!(
            "{}",
            style("Already at this version. Nothing to rollback.").yellow()
        );
        return Ok(());
    }

    if !ctx.confirm("Proceed with rollback?", false)? {
        println!("Cancelled.");
        return Ok(());
    }

    // Trigger the workflow
    trigger_rollback_workflow(ctx, &target_version, &target_env)?;

    ctx.print_success(&format!(
        "Rollback triggered: {} → {}",
        target_env, target_version
    ));
    println!();
    println!(
        "  {} Monitor at: {}",
        style("→").dim(),
        style("https://github.com/fourthplaces/shay/actions").cyan()
    );

    Ok(())
}

fn select_environment(ctx: &AppContext) -> Result<String> {
    let envs = vec!["prod", "dev"];

    let choice = Select::with_theme(&ctx.theme())
        .with_prompt("Environment to rollback")
        .items(&envs)
        .default(0)
        .interact()?;

    Ok(envs[choice].to_string())
}

fn select_rollback_version(ctx: &AppContext) -> Result<String> {
    let versions = get_recent_versions(ctx, 10)?;

    if versions.is_empty() {
        return Err(anyhow!("No previous versions found to rollback to."));
    }

    // Skip the first version (current) for rollback selection
    let rollback_versions: Vec<&String> = if versions.len() > 1 {
        versions.iter().skip(1).collect()
    } else {
        versions.iter().collect()
    };

    if rollback_versions.is_empty() {
        return Err(anyhow!("No previous versions to rollback to."));
    }

    let display_versions: Vec<String> = rollback_versions
        .iter()
        .enumerate()
        .map(|(i, v)| {
            if i == 0 {
                format!("{} (previous)", v)
            } else {
                v.to_string()
            }
        })
        .collect();

    let choice = Select::with_theme(&ctx.theme())
        .with_prompt("Select version to rollback to")
        .items(&display_versions)
        .default(0)
        .interact()?;

    Ok(rollback_versions[choice].clone())
}

fn trigger_rollback_workflow(ctx: &AppContext, version: &str, env: &str) -> Result<()> {
    println!();
    println!(
        "Triggering deploy-api workflow for {} → {}...",
        style(env).cyan(),
        style(version).green()
    );

    // Use gh workflow run to trigger the deploy-api workflow with workflow_dispatch
    let result = CmdBuilder::new("gh")
        .args([
            "workflow",
            "run",
            "deploy-api.yml",
            "-f",
            &format!("ref={}", version),
            "-f",
            &format!("env={}", env),
        ])
        .cwd(&ctx.repo)
        .run();

    match result {
        Ok(_) => {
            println!("  {} Workflow triggered successfully", style("✓").green());
            Ok(())
        }
        Err(e) => Err(anyhow!(
            "Failed to trigger workflow: {}. Make sure you're authenticated with 'gh auth login'.",
            e
        )),
    }
}
