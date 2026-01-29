//! Code quality commands (format, lint, check)

use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::cmd_builder::CmdBuilder;
use crate::config::Config;
use crate::context::AppContext;
use crate::utils::cmd_exists;

/// Find the expo/app package from config
fn find_app_package(ctx: &AppContext) -> Option<PathBuf> {
    let config = Config::load(&ctx.repo).ok()?;
    config
        .packages
        .values()
        .find(|pkg| pkg.release_type.as_deref() == Some("expo"))
        .map(|pkg| pkg.path.clone())
}

/// Run all formatters
/// If `capture_errors` is true, returns captured error output instead of failing
pub fn run_fmt(ctx: &AppContext, fix: bool, capture_errors: bool) -> Result<Option<String>> {
    ctx.print_header("Running formatters");

    let mut had_errors = false;
    let mut error_output = String::new();

    // Rust formatting
    if cmd_exists("cargo") {
        if !ctx.quiet {
            println!("[fmt] Running cargo fmt...");
        }
        let mut args = vec!["fmt", "--all"];
        if !fix {
            args.push("--check");
        }

        if capture_errors {
            let output = Command::new("cargo")
                .args(&args)
                .current_dir(&ctx.repo)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()?;

            if !output.status.success() {
                ctx.print_warning("cargo fmt had issues");
                error_output.push_str("=== cargo fmt ===\n");
                error_output.push_str(&String::from_utf8_lossy(&output.stderr));
                error_output.push_str(&String::from_utf8_lossy(&output.stdout));
                had_errors = true;
            }
        } else {
            let code = CmdBuilder::new("cargo").args(args).cwd(&ctx.repo).run()?;
            if code != 0 {
                if fix {
                    ctx.print_warning("cargo fmt had issues");
                } else {
                    ctx.print_warning("cargo fmt check failed (run with --fix to auto-fix)");
                }
                had_errors = true;
            }
        }
    }

    // TypeScript/JavaScript formatting with prettier
    if let Some(app_dir) = find_app_package(ctx) {
        if app_dir.exists() && cmd_exists("npx") {
            let app_rel = app_dir.strip_prefix(&ctx.repo).unwrap_or(&app_dir);
            if !ctx.quiet {
                println!("[fmt] Running prettier on {}...", app_rel.display());
            }
            let mut args = vec!["prettier"];
            if fix {
                args.push("--write");
            } else {
                args.push("--check");
            }
            args.push("src/**/*.{ts,tsx,js,jsx,json,css}");

            if capture_errors {
                let output = Command::new("npx")
                    .args(&args)
                    .current_dir(&app_dir)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()?;

                if !output.status.success() {
                    ctx.print_warning("prettier had issues");
                    error_output.push_str("\n=== prettier ===\n");
                    error_output.push_str(&String::from_utf8_lossy(&output.stderr));
                    error_output.push_str(&String::from_utf8_lossy(&output.stdout));
                    had_errors = true;
                }
            } else {
                let code = CmdBuilder::new("npx").args(args).cwd(&app_dir).run()?;
                if code != 0 {
                    if fix {
                        ctx.print_warning("prettier had issues");
                    } else {
                        ctx.print_warning("prettier check failed (run with --fix to auto-fix)");
                    }
                    had_errors = true;
                }
            }
        }
    }

    if had_errors {
        if capture_errors {
            return Ok(Some(error_output));
        }
        if fix {
            ctx.print_success("Formatting complete (with warnings).");
        } else {
            return Err(anyhow!("Formatting check failed"));
        }
    } else {
        ctx.print_success("Formatting complete.");
    }

    Ok(None)
}

/// Run all linters
/// If `capture_errors` is true, returns captured error output instead of failing
pub fn run_lint(ctx: &AppContext, fix: bool, capture_errors: bool) -> Result<Option<String>> {
    ctx.print_header("Running linters");

    let mut had_errors = false;
    let mut error_output = String::new();

    // Rust linting with clippy
    if cmd_exists("cargo") {
        if !ctx.quiet {
            println!("[lint] Running cargo clippy...");
        }
        let mut args = vec!["clippy", "--all-targets", "--all-features"];
        if fix {
            args.extend(["--fix", "--allow-dirty", "--allow-staged"]);
        }
        args.extend(["--", "-D", "warnings"]);

        if capture_errors {
            // Capture output for AI fixing (including stderr where errors go)
            let output = Command::new("cargo")
                .args(&args)
                .current_dir(&ctx.repo)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()?;

            if !output.status.success() {
                ctx.print_warning("clippy found issues");
                error_output.push_str("=== clippy ===\n");
                error_output.push_str(&String::from_utf8_lossy(&output.stderr));
                error_output.push_str(&String::from_utf8_lossy(&output.stdout));
                had_errors = true;
            }
        } else {
            let code = CmdBuilder::new("cargo").args(args).cwd(&ctx.repo).run()?;
            if code != 0 {
                ctx.print_warning("clippy found issues");
                had_errors = true;
            }
        }
    }

    // TypeScript/JavaScript linting with eslint
    if let Some(app_dir) = find_app_package(ctx) {
        if app_dir.exists() && cmd_exists("npx") {
            let app_rel = app_dir.strip_prefix(&ctx.repo).unwrap_or(&app_dir);
            if !ctx.quiet {
                println!("[lint] Running eslint on {}...", app_rel.display());
            }
            let mut args = vec!["eslint", "src"];
            if fix {
                args.push("--fix");
            }

            if capture_errors {
                // Capture output for AI fixing
                let output = Command::new("npx")
                    .args(&args)
                    .current_dir(&app_dir)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()?;

                if !output.status.success() {
                    ctx.print_warning("eslint found issues");
                    error_output.push_str("\n=== eslint ===\n");
                    error_output.push_str(&String::from_utf8_lossy(&output.stderr));
                    error_output.push_str(&String::from_utf8_lossy(&output.stdout));
                    had_errors = true;
                }
            } else {
                let code = CmdBuilder::new("npx").args(args).cwd(&app_dir).run()?;
                if code != 0 {
                    ctx.print_warning("eslint found issues");
                    had_errors = true;
                }
            }
        }
    }

    if had_errors {
        if capture_errors {
            return Ok(Some(error_output));
        }
        return Err(anyhow!("Linting found issues"));
    }

    ctx.print_success("Linting complete.");
    Ok(None)
}

/// Run pre-commit checks (fmt + lint + type check)
pub fn run_check(ctx: &AppContext) -> Result<()> {
    ctx.print_header("Running pre-commit checks");

    let mut had_errors = false;

    // Step 1: Format check
    if !ctx.quiet {
        println!();
        println!("=== Format Check ===");
    }
    if let Err(e) = run_fmt(ctx, false, false) {
        ctx.print_warning(&format!("Format check failed: {}", e));
        had_errors = true;
    }

    // Step 2: Lint check
    if !ctx.quiet {
        println!();
        println!("=== Lint Check ===");
    }
    if let Err(e) = run_lint(ctx, false, false) {
        ctx.print_warning(&format!("Lint check failed: {}", e));
        had_errors = true;
    }

    // Step 3: Type check using cmd system (handles dependencies automatically)
    if !ctx.quiet {
        println!();
        println!("=== Type Check ===");
    }

    // Use the cmd system which reads from dev.toml and handles deps
    let config = Config::load(&ctx.repo)?;
    let packages_with_typecheck = config.packages_with_cmd("typecheck");

    if packages_with_typecheck.is_empty() {
        if !ctx.quiet {
            println!("[typecheck] No packages define typecheck command");
        }
    } else {
        let opts = super::cmd::CmdOptions {
            parallel: false,
            variant: None,
            packages: vec![],
            capture: false,
        };
        let results = super::cmd::run_cmd(ctx, &config, "typecheck", &opts)?;
        if results.iter().any(|r| !r.success) {
            ctx.print_warning("typecheck failed");
            had_errors = true;
        }
    }

    println!();
    if had_errors {
        return Err(anyhow!("Pre-commit checks failed"));
    }

    ctx.print_success("All pre-commit checks passed!");
    Ok(())
}
