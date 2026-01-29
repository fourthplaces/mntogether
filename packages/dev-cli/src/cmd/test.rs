//! Test running commands

use anyhow::{anyhow, Result};
use dialoguer::Select;
use std::process::{Command, Stdio};

use crate::cmd_builder::CmdBuilder;
use crate::context::AppContext;
use crate::utils::list_rust_packages;

/// Check if the test command is cargo-based
fn is_cargo(command: &str) -> bool {
    command.starts_with("cargo ")
}

/// Check if the test command is nextest-based
fn is_nextest(command: &str) -> bool {
    command.contains("nextest")
}

/// Parse command into executable and arguments
fn parse_command(command: &str) -> (&str, Vec<&str>) {
    let parts: Vec<&str> = command.split_whitespace().collect();
    let (exe, args) = parts.split_first().unwrap_or((&"echo", &[]));
    (*exe, args.to_vec())
}

/// Run tests
/// If `capture_errors` is true, returns captured error output instead of failing
pub fn run_tests(
    ctx: &AppContext,
    package: Option<&str>,
    test_filter: Option<&str>,
    capture_errors: bool,
) -> Result<Option<String>> {
    let test_command = &ctx.config.global.test.command;
    let (exe, base_args) = parse_command(test_command);

    // Build args: start with base command args
    let mut args: Vec<String> = base_args.iter().map(|s| s.to_string()).collect();

    // Add package/filter args only for cargo-based commands
    if is_cargo(test_command) {
        if let Some(pkg) = package {
            args.push("-p".to_string());
            args.push(pkg.to_string());
        }

        if let Some(filter) = test_filter {
            if is_nextest(test_command) {
                args.push("-E".to_string());
                args.push(format!("test({})", filter));
            } else {
                args.push("--".to_string());
                args.push(filter.to_string());
            }
        }
    }

    ctx.print_header(&format!("Running tests: {} {}", exe, args.join(" ")));

    if capture_errors {
        // Capture output while displaying it to the user
        let output = Command::new(exe)
            .args(&args)
            .current_dir(&ctx.repo)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Print output so user sees it
        if !stdout.is_empty() {
            print!("{}", stdout);
        }
        if !stderr.is_empty() {
            eprint!("{}", stderr);
        }

        if !output.status.success() {
            let mut error_output = format!("=== {} ===\n", test_command);
            error_output.push_str(&stderr);
            error_output.push_str(&stdout);
            return Ok(Some(error_output));
        }
    } else {
        let code = CmdBuilder::new(exe)
            .args(&args)
            .cwd(&ctx.repo)
            .inherit_io()
            .run()?;

        if code != 0 {
            return Err(anyhow!("{} exited with code {code}", test_command));
        }
    }

    Ok(None)
}

/// Watch tests for changes
pub fn watch_tests(ctx: &AppContext) -> Result<()> {
    let watch_command = ctx
        .config
        .global
        .test
        .watch_command
        .as_ref()
        .ok_or_else(|| anyhow!("No watch_command configured in [test] section"))?;

    let (exe, base_args) = parse_command(watch_command);
    let args: Vec<String> = base_args.iter().map(|s| s.to_string()).collect();

    ctx.print_header(&format!("Watching tests: {}", watch_command));
    ctx.print_warning("Press Ctrl+C to stop watching.");

    let code = CmdBuilder::new(exe)
        .args(&args)
        .cwd(&ctx.repo)
        .inherit_io()
        .run()?;

    // 130 = SIGINT (Ctrl+C), which is expected
    if code != 0 && code != 130 {
        return Err(anyhow!("{} exited with code {code}", watch_command));
    }
    Ok(())
}

/// Interactive menu for running tests
pub fn test_menu(ctx: &AppContext) -> Result<()> {
    ctx.print_header("Run tests");

    let test_command = &ctx.config.global.test.command;
    let has_watch = ctx.config.global.test.watch_command.is_some();
    let is_cargo_cmd = is_cargo(test_command);

    // Package selection only for cargo-based commands
    let selected_package = if is_cargo_cmd {
        let packages = list_rust_packages(&ctx.repo)?;

        let mut pkg_items: Vec<String> = vec!["[All packages]".to_string()];
        pkg_items.extend(packages.clone());

        let pkg_choice = if ctx.quiet {
            0
        } else {
            Select::with_theme(&ctx.theme())
                .with_prompt("Which package to test?")
                .items(&pkg_items)
                .default(0)
                .interact()?
        };

        if pkg_choice == 0 {
            None
        } else {
            Some(packages[pkg_choice - 1].clone())
        }
    } else {
        None
    };

    // Test filter only for cargo-based commands
    let test_filter = if is_cargo_cmd && !ctx.quiet {
        let filter: String = dialoguer::Input::with_theme(&ctx.theme())
            .with_prompt("Test name filter (leave empty for all)")
            .allow_empty(true)
            .interact_text()?;
        if filter.trim().is_empty() {
            None
        } else {
            Some(filter.trim().to_string())
        }
    } else {
        None
    };

    // Watch mode only if watch_command is configured
    let watch = if has_watch && !ctx.quiet {
        ctx.confirm("Watch for changes and re-run tests?", false)?
    } else {
        false
    };

    if watch {
        watch_tests(ctx)
    } else {
        run_tests(
            ctx,
            selected_package.as_deref(),
            test_filter.as_deref(),
            false,
        )?;
        Ok(())
    }
}
