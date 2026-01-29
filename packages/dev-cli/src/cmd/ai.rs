//! AI Fix Command - Run commands and fix errors with Claude
//!
//! This module handles the `ai fix` command which runs external commands
//! and uses Claude to fix any errors that occur.
//!
//! For lint functionality, see the `ai_lint` module.

use anyhow::{anyhow, Result};
use console::style;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

use crate::cmd_builder::CmdBuilder;
use crate::context::AppContext;
use crate::utils::cmd_exists;

// =============================================================================
// AI Fix Command
// =============================================================================

/// Run commands and use AI to fix any errors.
///
/// Valid commands:
/// - "fmt" or "format" - Run formatters with --fix
/// - "test" or "tests" - Run tests
/// - "docker [service]" - Build and run docker
/// - (empty or "all") - Run all steps: fmt, test, docker
pub fn ai_fix(ctx: &AppContext, command: &[String]) -> Result<()> {
    // Check for Claude CLI first
    if !cmd_exists("claude") {
        return Err(anyhow!(
            "Claude CLI not found. Install from: https://docs.anthropic.com/en/docs/claude-code"
        ));
    }

    let cmd = command.join(" ").to_lowercase();
    let cmd = cmd.trim();

    ctx.print_header("AI Fix");

    // Collect all errors from the steps we run
    let mut all_errors = String::new();

    match cmd {
        "" | "all" => {
            // Run all steps in order
            println!("{}", style("Running all auto-fix steps...").dim());
            println!();

            // Step 1: Format
            println!("{}", style("=== Step 1: Format ===").bold());
            if let Some(errors) = run_fix_step(ctx, "fmt")? {
                all_errors.push_str(&errors);
            }

            // Step 2: Lint (standard linters, not ai-lint)
            println!();
            println!("{}", style("=== Step 2: Lint ===").bold());
            if let Some(errors) = run_fix_step(ctx, "lint")? {
                all_errors.push_str(&errors);
            }

            // Step 3: Test
            println!();
            println!("{}", style("=== Step 3: Tests ===").bold());
            if let Some(errors) = run_fix_step(ctx, "test")? {
                all_errors.push_str(&errors);
            }
        }
        "fmt" | "format" => {
            if let Some(errors) = run_fix_step(ctx, "fmt")? {
                all_errors.push_str(&errors);
            }
        }
        "lint" => {
            if let Some(errors) = run_fix_step(ctx, "lint")? {
                all_errors.push_str(&errors);
            }
        }
        // AI lint categories - run lint then Claude fix loop
        "lint sec" | "lint security" => {
            return run_claude_fix(ctx, "lint sec");
        }
        "lint tc" | "lint test-coverage" | "lint testcoverage" => {
            return run_claude_fix(ctx, "lint tc");
        }
        "lint migrations" | "lint migrate" => {
            return run_claude_fix(ctx, "lint migrations");
        }
        "test" | "tests" => {
            // Let Claude handle the recursive fix loop
            return run_claude_fix(ctx, "test");
        }
        cmd if cmd == "docker" || cmd.starts_with("docker ") => {
            // Extract optional service name: "docker api" -> Some("api")
            let service = cmd
                .strip_prefix("docker")
                .map(|s| s.trim())
                .filter(|s| !s.is_empty());
            if let Some(errors) = run_docker_step(ctx, service)? {
                all_errors.push_str(&errors);
            }
        }
        _ => {
            // Custom command - just run it and capture output
            println!("Running custom command: {}", style(cmd).cyan());
            let output = Command::new("sh")
                .args(["-c", cmd])
                .current_dir(&ctx.repo)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()?;

            if !output.status.success() {
                all_errors.push_str(&format!("=== {} ===\n", cmd));
                all_errors.push_str(&String::from_utf8_lossy(&output.stderr));
                all_errors.push_str(&String::from_utf8_lossy(&output.stdout));
            }
        }
    }

    // If we have errors, pass them to Claude
    if all_errors.is_empty() {
        println!();
        ctx.print_success("All checks passed! No issues to fix.");
        return Ok(());
    }

    println!();
    ctx.print_header("Using Claude to fix remaining issues");
    println!("{}", style("Launching Claude to fix the issues...").dim());
    println!();

    let prompt = format!(
        "Fix the following errors in this codebase. \
        Read each file mentioned, understand the error, and fix it. \
        After fixing, briefly explain what you changed.\n\n{}",
        all_errors
    );

    let code = CmdBuilder::new("claude")
        .arg(&prompt)
        .cwd(&ctx.repo)
        .inherit_io()
        .run()?;

    if code != 0 {
        return Err(anyhow!("AI fix failed"));
    }

    ctx.print_success("AI fix complete. Run the checks again to verify.");
    Ok(())
}

/// Run a single fix step and return any errors (None if step passed)
fn run_fix_step(ctx: &AppContext, step: &str) -> Result<Option<String>> {
    use super::quality::{run_fmt, run_lint};
    use super::test::run_tests;

    match step {
        "fmt" => run_fmt(ctx, true, true),
        "lint" => run_lint(ctx, true, true),
        "test" => run_tests(ctx, None, None, true),
        _ => Ok(None),
    }
}

/// Let Claude handle the fix loop - it will run the command, fix issues, and repeat
fn run_claude_fix(ctx: &AppContext, step: &str) -> Result<()> {
    let (command, extra_instructions) = match step {
        "test" => ("./dev.sh test", ""),
        "lint sec" => (
            "./dev.sh ai lint sec",
            "\n- If you cannot fix an issue, add @ai-lint-ignore comment",
        ),
        "lint tc" => (
            "./dev.sh ai lint tc",
            "\n\nThe output shows exactly what needs tests:\n\
            - Each gap lists the SOURCE FILE, FUNCTION NAME, and LINE NUMBER\n\
            - For Rust: add tests to `#[cfg(test)] mod tests` at the bottom of the SAME file\n\
            - TEST SCENARIOS list the specific test function names to implement\n\n\
            For each gap:\n\
            1. Read the source file to understand the function\n\
            2. Find or create the `#[cfg(test)]` module at the bottom of the file\n\
            3. Add `#[test]` functions for each listed scenario\n\
            4. Run the lint again to verify coverage\n\n\
            If truly untestable, add @ai-test-ignore TST100 comment above the function",
        ),
        "lint migrations" => ("./dev.sh ai lint migrations", ""),
        _ => return Err(anyhow!("Unknown step: {}", step)),
    };

    let prompt = format!(
        "Run `{command}` and fix any errors. Keep running and fixing until all issues are resolved.\n\n\
        IMPORTANT:\n\
        - Find the ROOT CAUSE of errors, not just symptoms\n\
        - The error location may differ from the fix location\n\
        - Fix ALL errors before re-running\n\
        - Keep iterating until the command succeeds{extra_instructions}"
    );

    ctx.print_header(&format!("AI Fix: {}", step));
    println!("{}", style("Launching Claude to fix issues...").dim());
    println!();

    let code = CmdBuilder::new("claude")
        .arg(&prompt)
        .cwd(&ctx.repo)
        .inherit_io()
        .run()?;

    if code != 0 {
        return Err(anyhow!("Claude fix failed"));
    }

    Ok(())
}

// =============================================================================
// Docker Fix
// =============================================================================

/// Run a command with real-time streaming output, capturing for error reporting
fn run_streaming_command(ctx: &AppContext, program: &str, args: &[&str]) -> Result<(bool, String)> {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(&ctx.repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut captured = String::new();

    // Stream stdout in real-time
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().flatten() {
            println!("{}", line);
            captured.push_str(&line);
            captured.push('\n');
        }
    }

    // Stream stderr in real-time
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        for line in reader.lines().flatten() {
            eprintln!("{}", line);
            captured.push_str(&line);
            captured.push('\n');
        }
    }

    let status = child.wait()?;
    Ok((status.success(), captured))
}

/// Parse docker build output to identify which service/Dockerfile failed
fn parse_failed_service(output: &str) -> Option<String> {
    for line in output.lines() {
        // BuildKit error format: "ERROR [service stage]"
        if line.contains("ERROR [") {
            if let Some(start) = line.find("ERROR [") {
                let rest = &line[start + 7..];
                if let Some(end) = rest.find(' ') {
                    return Some(rest[..end].to_string());
                } else if let Some(end) = rest.find(']') {
                    return Some(rest[..end].to_string());
                }
            }
        }
        // Alternative format: "failed to solve: service:"
        if line.contains("failed to solve:") {
            if let Some(start) = line.find("failed to solve:") {
                let rest = &line[start + 16..].trim();
                if let Some(end) = rest.find(':') {
                    return Some(rest[..end].to_string());
                }
            }
        }
    }
    None
}

/// Run docker compose build, start containers, and verify they're healthy
fn run_docker_step(ctx: &AppContext, service: Option<&str>) -> Result<Option<String>> {
    let mut all_errors = String::new();

    let service_msg = service
        .map(|s| format!(" (service: {})", s))
        .unwrap_or_default();

    // Step 1: Build Docker images
    println!("[docker] Building Docker images{}...", service_msg);

    let mut build_args = vec!["compose", "build", "--progress=plain"];
    if let Some(svc) = service {
        build_args.push(svc);
    }

    let (success, output) = run_streaming_command(ctx, "docker", &build_args)?;

    if success {
        println!("  {} Docker build{}", style("✓").green(), service_msg);
    } else {
        let failed_svc = parse_failed_service(&output);
        let fail_msg = failed_svc
            .as_ref()
            .map(|s| format!(" (failed: {})", s))
            .unwrap_or_default();

        println!("  {} Docker build failed{}", style("✗").red(), fail_msg);

        all_errors.push_str(&format!(
            "=== Docker build errors{} ===\n\
            docker compose build --progress=plain failed:\n\n\
            {}\n\n",
            fail_msg, output
        ));

        // Don't continue if build failed
        return Ok(Some(all_errors));
    }

    // Step 2: Start containers
    println!("[docker] Starting containers{}...", service_msg);

    let mut up_args = vec!["compose", "up", "-d"];
    if let Some(svc) = service {
        up_args.push(svc);
    }

    let (success, output) = run_streaming_command(ctx, "docker", &up_args)?;

    if success {
        println!("  {} Containers started{}", style("✓").green(), service_msg);
    } else {
        println!(
            "  {} Failed to start containers{}",
            style("✗").red(),
            service_msg
        );

        all_errors.push_str(&format!(
            "=== Docker start errors{} ===\n\
            docker compose up -d failed:\n\n\
            {}\n\n",
            service_msg, output
        ));

        return Ok(Some(all_errors));
    }

    // Step 3: Wait for containers to be healthy
    println!("[docker] Waiting for containers to be healthy...");

    std::thread::sleep(std::time::Duration::from_secs(3));

    let max_retries = 6;
    let mut healthy = false;

    for attempt in 1..=max_retries {
        let output = Command::new("docker")
            .args(["compose", "ps", "--format", "json"])
            .current_dir(&ctx.repo)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut all_running = true;
        let mut unhealthy_containers = Vec::new();

        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(container) = serde_json::from_str::<serde_json::Value>(line) {
                let name = container["Name"].as_str().unwrap_or("unknown");
                let state = container["State"].as_str().unwrap_or("unknown");
                let health = container["Health"].as_str().unwrap_or("");

                if let Some(svc) = service {
                    if !name.contains(svc) {
                        continue;
                    }
                }

                if state != "running" {
                    all_running = false;
                    unhealthy_containers.push(format!("{}: state={}", name, state));
                } else if health == "unhealthy" {
                    all_running = false;
                    unhealthy_containers.push(format!("{}: health=unhealthy", name));
                } else if health == "starting" {
                    all_running = false;
                }
            }
        }

        if all_running {
            healthy = true;
            break;
        }

        if attempt < max_retries {
            println!(
                "  {} Waiting for containers (attempt {}/{}): {}",
                style("◌").cyan(),
                attempt,
                max_retries,
                unhealthy_containers.join(", ")
            );
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }

    if healthy {
        println!(
            "  {} All containers running and healthy",
            style("✓").green()
        );
        Ok(None)
    } else {
        println!("  {} Some containers are not healthy", style("✗").red());

        let ps_output = Command::new("docker")
            .args(["compose", "ps", "-a"])
            .current_dir(&ctx.repo)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        let ps_status = String::from_utf8_lossy(&ps_output.stdout);

        let mut logs_args = vec!["compose", "logs", "--tail=50"];
        if let Some(svc) = service {
            logs_args.push(svc);
        }

        let logs_output = Command::new("docker")
            .args(&logs_args)
            .current_dir(&ctx.repo)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        let logs = String::from_utf8_lossy(&logs_output.stdout);
        let logs_stderr = String::from_utf8_lossy(&logs_output.stderr);

        all_errors.push_str(&format!(
            "=== Docker health check failed{} ===\n\
            Some containers are not running or healthy after 30 seconds.\n\n\
            Container Status:\n{}\n\n\
            Recent Logs:\n{}\n{}\n",
            service_msg, ps_status, logs, logs_stderr
        ));

        Ok(Some(all_errors))
    }
}
