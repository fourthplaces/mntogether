//! Unified AI Lint Command
//!
//! Provides a single `ai lint` command with category support:
//! - `ai lint tc` - Test coverage (TST*)
//! - `ai lint sec` - Security (WF*, INF*, DKR*, AUTH*, ENV*)
//! - `ai lint migrations` - Migration safety (AI-powered)
//! - `ai lint all` - All categories (default)

pub mod engine;
pub mod rules;

use anyhow::Result;
use console::style;
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

use crate::cmd_builder::CmdBuilder;
use crate::context::AppContext;
use crate::utils::cmd_exists;

use engine::{check_file, format_rules_help, parse_ignored_rules, Severity};
use rules::{
    check_cleanup, check_migrations, check_test_coverage_ai, get_rule_categories, get_rules,
    get_security_rules, sec::is_sensitive_path, LintCategory,
};

// =============================================================================
// Main Entry Point
// =============================================================================

/// Run the unified AI lint command
///
/// # Arguments
/// * `ctx` - Application context
/// * `category` - Lint category (tc, sec, migrations, all)
/// * `fix` - Whether to use Claude to fix violations
/// * `show_rules` - Show available rules and exit
/// * `base_branch` - Base branch for diff comparison
/// * `files` - Specific files to lint (uses git diff if empty)
pub fn ai_lint(
    ctx: &AppContext,
    category: Option<&str>,
    fix: bool,
    show_rules: bool,
    base_branch: Option<&str>,
    files: Option<Vec<String>>,
) -> Result<()> {
    let category = category
        .and_then(LintCategory::from_str)
        .unwrap_or(LintCategory::All);

    let base = base_branch.unwrap_or("main");

    // Show rules help if requested
    if show_rules {
        print_rules_help(category);
        return Ok(());
    }

    // Get files to analyze
    let files_to_check = match files {
        Some(f) if !f.is_empty() => f,
        _ => get_changed_files(ctx, base)?,
    };

    ctx.print_header(&format!(
        "AI Lint - {} ({}..HEAD)",
        category.display_name(),
        base
    ));
    println!();

    // Run the appropriate linter(s)
    let is_safe = match category {
        LintCategory::TestCoverage => lint_test_coverage(ctx, base, &files_to_check)?,
        LintCategory::Security => lint_security(ctx, &files_to_check)?,
        LintCategory::Migrations => {
            let migration_files: Vec<_> = files_to_check
                .iter()
                .filter(|f| f.contains("migrations/") && f.ends_with(".sql"))
                .cloned()
                .collect();
            check_migrations(ctx, &migration_files)?
        }
        LintCategory::Cleanup => {
            // Cleanup check doesn't depend on changed files - it scans for all deprecated markers
            // and checks against completed workflows
            check_cleanup(ctx, false)?
        }
        LintCategory::All => {
            let mut all_safe = true;

            // Security lint
            println!("{}", style("=== Security ===").bold());
            if !lint_security(ctx, &files_to_check)? {
                all_safe = false;
            }

            // Test coverage lint
            println!();
            println!("{}", style("=== Test Coverage ===").bold());
            if !lint_test_coverage(ctx, base, &files_to_check)? {
                all_safe = false;
            }

            // Migration lint
            let migration_files: Vec<_> = files_to_check
                .iter()
                .filter(|f| f.contains("migrations/") && f.ends_with(".sql"))
                .cloned()
                .collect();
            if !migration_files.is_empty() {
                println!();
                println!("{}", style("=== Migrations ===").bold());
                if !check_migrations(ctx, &migration_files)? {
                    all_safe = false;
                }
            }

            // Cleanup check
            println!();
            println!("{}", style("=== Migration Cleanup ===").bold());
            if !check_cleanup(ctx, false)? {
                all_safe = false;
            }

            all_safe
        }
    };

    // If fix requested and there are issues, invoke Claude
    if fix && !is_safe {
        println!();
        run_claude_fix(ctx, category)?;
    } else if !is_safe {
        std::process::exit(1);
    }

    Ok(())
}

// =============================================================================
// Security Linting
// =============================================================================

/// Lint sensitive files for security issues
fn lint_security(ctx: &AppContext, files: &[String]) -> Result<bool> {
    let rules = get_security_rules();

    // Filter to only sensitive files
    let sensitive_files: Vec<&String> = files.iter().filter(|f| is_sensitive_path(f)).collect();

    if sensitive_files.is_empty() {
        println!("  {} No sensitive files to lint", style("~").dim());
        return Ok(true);
    }

    // Load allowlist
    let allowlist_path = ctx.repo.join("scripts/ai-lint-allowlist.txt");
    let allowlist: Vec<String> = if allowlist_path.exists() {
        std::fs::read_to_string(&allowlist_path)?
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
            .map(|s| s.to_string())
            .collect()
    } else {
        Vec::new()
    };

    let mut all_safe = true;
    let mut total_errors = 0;
    let mut total_warnings = 0;

    for file_path in sensitive_files {
        let filename = Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path);

        // Check allowlist
        if allowlist.iter().any(|a| file_path.contains(a)) {
            println!("  {} [{}]: In allowlist", style("~").dim(), filename);
            continue;
        }

        let full_path = ctx.repo.join(file_path);
        if !full_path.exists() {
            continue;
        }

        let content = match std::fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(_) => continue, // Skip binary files
        };

        if content.is_empty() {
            continue;
        }

        // Parse inline ignore directives
        let (ignored_rules, skip_file) = parse_ignored_rules(&content, "ai-lint-ignore");

        if skip_file {
            println!(
                "  {} [{}]: Skipped (@ai-lint-ignore-file)",
                style("~").dim(),
                filename
            );
            continue;
        }

        // Run rule checks
        let violations = check_file(file_path, &content, &rules, &ignored_rules);

        if violations.is_empty() {
            println!("  {} [{}]: OK", style("✓").green(), filename);
        } else {
            let errors: Vec<_> = violations
                .iter()
                .filter(|v| v.severity == Severity::Error)
                .collect();
            let warnings: Vec<_> = violations
                .iter()
                .filter(|v| v.severity == Severity::Warning)
                .collect();

            total_errors += errors.len();
            total_warnings += warnings.len();

            if !errors.is_empty() {
                all_safe = false;
            }

            println!(
                "  {} [{}]: {} error(s), {} warning(s)",
                if errors.is_empty() {
                    style("⚠").yellow()
                } else {
                    style("✗").red()
                },
                filename,
                errors.len(),
                warnings.len()
            );

            for v in &violations {
                let severity_str = match v.severity {
                    Severity::Error => style("error").red().bold(),
                    Severity::Warning => style("warn").yellow(),
                };
                println!(
                    "    {}:{} {} [{}]: {}",
                    filename,
                    v.line,
                    severity_str,
                    style(v.rule_id).cyan(),
                    v.message
                );
            }
        }
    }

    println!();
    if all_safe && total_warnings == 0 {
        println!("  {} All sensitive files passed", style("✓").green());
    } else if all_safe {
        println!(
            "  {} Passed with {} warning(s)",
            style("⚠").yellow(),
            total_warnings
        );
    } else {
        println!(
            "  {} {} error(s), {} warning(s)",
            style("✗").red().bold(),
            total_errors,
            total_warnings
        );
    }

    Ok(all_safe)
}

// =============================================================================
// Test Coverage Linting (AI-Powered)
// =============================================================================

/// Lint for test coverage on changed code using AI analysis
fn lint_test_coverage(ctx: &AppContext, base_branch: &str, files: &[String]) -> Result<bool> {
    // Use AI-powered analysis for business flow coverage
    check_test_coverage_ai(ctx, files, base_branch)
}

// =============================================================================
// Helpers
// =============================================================================

/// Get files changed between base branch and HEAD
fn get_changed_files(ctx: &AppContext, base_branch: &str) -> Result<Vec<String>> {
    // Check if base branch exists (try local first, then origin/)
    let branch_check = Command::new("git")
        .args(["rev-parse", "--verify", base_branch])
        .current_dir(&ctx.repo)
        .output()?;

    let compare_ref = if branch_check.status.success() {
        base_branch.to_string()
    } else {
        // Try origin/base_branch (common in CI where local branch doesn't exist)
        let origin_branch = format!("origin/{}", base_branch);
        let origin_check = Command::new("git")
            .args(["rev-parse", "--verify", &origin_branch])
            .current_dir(&ctx.repo)
            .output()?;

        if origin_check.status.success() {
            origin_branch
        } else {
            "HEAD~1".to_string()
        }
    };

    let output = Command::new("git")
        .args(["diff", "--name-only", &format!("{}..HEAD", compare_ref)])
        .current_dir(&ctx.repo)
        .output()?;

    let mut all_files: HashSet<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect();

    // Include uncommitted changes
    let uncommitted = Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .current_dir(&ctx.repo)
        .output()?;

    for line in String::from_utf8_lossy(&uncommitted.stdout).lines() {
        all_files.insert(line.to_string());
    }

    // Include staged changes
    let staged = Command::new("git")
        .args(["diff", "--name-only", "--cached"])
        .current_dir(&ctx.repo)
        .output()?;

    for line in String::from_utf8_lossy(&staged.stdout).lines() {
        all_files.insert(line.to_string());
    }

    Ok(all_files.into_iter().collect())
}

/// Print rules help for a category
fn print_rules_help(category: LintCategory) {
    let rules = get_rules(category);
    let categories = get_rule_categories(category);
    let prefix = category.ignore_prefix();

    if rules.is_empty() && category == LintCategory::Migrations {
        println!("Migration linting is AI-powered and has no pattern-based rules.");
        println!("It analyzes SQL migrations for backward compatibility issues.");
        return;
    }

    if rules.is_empty() && category == LintCategory::Cleanup {
        println!("Migration cleanup detects deprecated code that should be deleted.");
        println!();
        println!("It scans for MIGRATION_DEPRECATED markers and checks if the");
        println!("associated workflow migration has completed.");
        println!();
        println!("Marker format: // MIGRATION_DEPRECATED: migration_name");
        println!();
        println!("Usage:");
        println!("  ./dev.sh ai lint cleanup           # Check for cleanup needed");
        println!("  ./dev.sh ai lint cleanup --fix     # Use Claude to clean up");
        return;
    }

    println!("{}", format_rules_help(&rules, &categories, prefix));
}

/// Invoke Claude to fix lint issues
fn run_claude_fix(ctx: &AppContext, category: LintCategory) -> Result<()> {
    if !cmd_exists("claude") {
        return Err(anyhow::anyhow!(
            "Claude CLI not found. Install from: https://docs.anthropic.com/en/docs/claude-code"
        ));
    }

    let command = match category {
        LintCategory::TestCoverage => "./dev.sh ai lint tc",
        LintCategory::Security => "./dev.sh ai lint sec",
        LintCategory::Migrations => "./dev.sh ai lint migrations",
        LintCategory::Cleanup => "./dev.sh ai lint cleanup",
        LintCategory::All => "./dev.sh ai lint",
    };

    let ignore_hint = match category {
        LintCategory::Security => "\n\nIf you cannot fix an issue, add @ai-lint-ignore comment.",
        LintCategory::TestCoverage => {
            "\n\nWhen adding tests:\n\
            - Follow existing test patterns\n\
            - Add tests for happy path, error cases, and edge cases\n\
            - If you cannot add tests, use @ai-test-ignore TST100 comment"
        }
        LintCategory::Cleanup => {
            "\n\nWhen cleaning up deprecated code:\n\
            - Read the MIGRATION_DEPRECATED markers to understand what should be removed\n\
            - Verify the migration workflow is completed before deleting code\n\
            - Remove old code paths, feature flags, and the migration registration\n\
            - Update deprecated_code.yaml to remove the migration entry\n\
            - See docs/ai/tasks/cleanup-migration.md for detailed instructions"
        }
        _ => "",
    };

    let prompt = format!(
        "Run `{command}` and fix any errors. Keep running and fixing until all issues are resolved.\n\n\
        IMPORTANT:\n\
        - Find the ROOT CAUSE of errors, not just symptoms\n\
        - Fix ALL errors before re-running\n\
        - Keep iterating until the command succeeds{ignore_hint}"
    );

    ctx.print_header(&format!("AI Fix: {}", category.display_name()));
    println!("{}", style("Launching Claude to fix issues...").dim());
    println!();

    let code = CmdBuilder::new("claude")
        .arg(&prompt)
        .cwd(&ctx.repo)
        .inherit_io()
        .run()?;

    if code != 0 {
        return Err(anyhow::anyhow!("Claude fix failed"));
    }

    Ok(())
}
