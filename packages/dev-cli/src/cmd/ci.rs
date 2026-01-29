//! CI/CD commands using GitHub Actions (gh CLI)

use anyhow::{anyhow, Result};
use console::style;
use dialoguer::Select;

use crate::cmd_builder::CmdBuilder;
use crate::config::Config;
use crate::context::AppContext;
use crate::utils::cmd_exists;

/// Ensure gh CLI is available and authenticated
fn ensure_gh() -> Result<()> {
    if !cmd_exists("gh") {
        return Err(anyhow!(
            "GitHub CLI (gh) not found. Install from: https://cli.github.com/"
        ));
    }
    Ok(())
}

/// Show current CI status for the repository
pub fn ci_status(ctx: &AppContext) -> Result<()> {
    ci_status_with_config(ctx, None)
}

/// Show current CI status for the repository with config
pub fn ci_status_with_config(ctx: &AppContext, config: Option<&Config>) -> Result<()> {
    ensure_gh()?;

    ctx.print_header("CI/CD Status");

    // Get current branch
    let branch_output = CmdBuilder::new("git")
        .args(["branch", "--show-current"])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()?;
    let current_branch = branch_output.stdout_string().trim().to_string();

    println!("Branch: {}", style(&current_branch).cyan());
    println!();

    // Show recent runs for this branch
    println!("{}", style("Recent runs on this branch:").bold());
    let code = CmdBuilder::new("gh")
        .args(["run", "list", "--branch", &current_branch, "--limit", "5"])
        .cwd(&ctx.repo)
        .run()?;

    if code != 0 {
        return Err(anyhow!("Failed to fetch CI status"));
    }

    println!();

    // Show PR checks if on a feature branch (not a protected branch)
    let protected_branches: Vec<&str> = config
        .map(|c| {
            c.global
                .git
                .protected_branches
                .iter()
                .map(|s| s.as_str())
                .collect()
        })
        .unwrap_or_else(|| vec!["main", "master", "dev"]);

    if !protected_branches.contains(&current_branch.as_str()) {
        println!("{}", style("PR checks:").bold());
        let _ = CmdBuilder::new("gh")
            .args(["pr", "checks"])
            .cwd(&ctx.repo)
            .run();
    }

    Ok(())
}

/// List recent workflow runs
pub fn ci_runs(ctx: &AppContext, limit: u32, workflow: Option<&str>) -> Result<()> {
    ensure_gh()?;

    ctx.print_header("Recent Workflow Runs");

    let mut args = vec![
        "run".to_string(),
        "list".to_string(),
        "--limit".to_string(),
        limit.to_string(),
    ];

    if let Some(wf) = workflow {
        args.push("--workflow".to_string());
        args.push(wf.to_string());
    }

    let code = CmdBuilder::new("gh").args(&args).cwd(&ctx.repo).run()?;

    if code != 0 {
        return Err(anyhow!("Failed to list workflow runs"));
    }

    Ok(())
}

/// View logs for a specific workflow run
pub fn ci_logs(ctx: &AppContext, run_id: Option<&str>) -> Result<()> {
    ensure_gh()?;

    let run = match run_id {
        Some(id) => id.to_string(),
        None => {
            // Interactive selection
            let output = CmdBuilder::new("gh")
                .args([
                    "run",
                    "list",
                    "--limit",
                    "10",
                    "--json",
                    "databaseId,displayTitle,status,conclusion,headBranch,createdAt",
                ])
                .cwd(&ctx.repo)
                .capture_stdout()
                .run_capture()?;

            let runs: Vec<serde_json::Value> =
                serde_json::from_str(&output.stdout_string()).unwrap_or_default();

            if runs.is_empty() {
                return Err(anyhow!("No workflow runs found"));
            }

            let choices: Vec<String> = runs
                .iter()
                .map(|r| {
                    let title = r["displayTitle"].as_str().unwrap_or("Unknown");
                    let branch = r["headBranch"].as_str().unwrap_or("?");
                    let status = r["status"].as_str().unwrap_or("?");
                    let conclusion = r["conclusion"].as_str().unwrap_or("");
                    let state = if conclusion.is_empty() {
                        status.to_string()
                    } else {
                        conclusion.to_string()
                    };
                    format!("[{}] {} ({})", state, title, branch)
                })
                .collect();

            let selection = Select::with_theme(&ctx.theme())
                .with_prompt("Select a workflow run")
                .items(&choices)
                .default(0)
                .interact()?;

            runs[selection]["databaseId"]
                .as_i64()
                .map(|id| id.to_string())
                .ok_or_else(|| anyhow!("Failed to get run ID"))?
        }
    };

    ctx.print_header(&format!("Logs for run {}", run));

    let code = CmdBuilder::new("gh")
        .args(["run", "view", &run, "--log"])
        .cwd(&ctx.repo)
        .run()?;

    if code != 0 {
        return Err(anyhow!("Failed to fetch logs"));
    }

    Ok(())
}

/// Watch a workflow run in progress
pub fn ci_watch(ctx: &AppContext, run_id: Option<&str>) -> Result<()> {
    ensure_gh()?;

    let run = match run_id {
        Some(id) => id.to_string(),
        None => {
            // Find the most recent in-progress run
            let output = CmdBuilder::new("gh")
                .args([
                    "run",
                    "list",
                    "--status",
                    "in_progress",
                    "--limit",
                    "1",
                    "--json",
                    "databaseId",
                ])
                .cwd(&ctx.repo)
                .capture_stdout()
                .run_capture()?;

            let runs: Vec<serde_json::Value> =
                serde_json::from_str(&output.stdout_string()).unwrap_or_default();

            if runs.is_empty() {
                // No in-progress runs, watch the most recent
                let output = CmdBuilder::new("gh")
                    .args(["run", "list", "--limit", "1", "--json", "databaseId"])
                    .cwd(&ctx.repo)
                    .capture_stdout()
                    .run_capture()?;

                let runs: Vec<serde_json::Value> =
                    serde_json::from_str(&output.stdout_string()).unwrap_or_default();

                runs.first()
                    .and_then(|r| r["databaseId"].as_i64())
                    .map(|id| id.to_string())
                    .ok_or_else(|| anyhow!("No workflow runs found"))?
            } else {
                runs[0]["databaseId"]
                    .as_i64()
                    .map(|id| id.to_string())
                    .ok_or_else(|| anyhow!("Failed to get run ID"))?
            }
        }
    };

    ctx.print_header(&format!("Watching run {}", run));
    println!("Press Ctrl+C to stop watching");
    println!();

    let code = CmdBuilder::new("gh")
        .args(["run", "watch", &run])
        .cwd(&ctx.repo)
        .run()?;

    if code != 0 {
        return Err(anyhow!("Watch failed"));
    }

    Ok(())
}

/// Trigger a workflow manually
pub fn ci_trigger(ctx: &AppContext, workflow: Option<&str>, branch: Option<&str>) -> Result<()> {
    ensure_gh()?;

    let wf = match workflow {
        Some(w) => w.to_string(),
        None => {
            // List available workflows
            let output = CmdBuilder::new("gh")
                .args(["workflow", "list", "--json", "name,state"])
                .cwd(&ctx.repo)
                .capture_stdout()
                .run_capture()?;

            let workflows: Vec<serde_json::Value> =
                serde_json::from_str(&output.stdout_string()).unwrap_or_default();

            if workflows.is_empty() {
                return Err(anyhow!("No workflows found"));
            }

            let choices: Vec<String> = workflows
                .iter()
                .map(|w| {
                    let name = w["name"].as_str().unwrap_or("Unknown");
                    let state = w["state"].as_str().unwrap_or("?");
                    format!("{} ({})", name, state)
                })
                .collect();

            let selection = Select::with_theme(&ctx.theme())
                .with_prompt("Select a workflow to trigger")
                .items(&choices)
                .default(0)
                .interact()?;

            workflows[selection]["name"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| anyhow!("Failed to get workflow name"))?
        }
    };

    // Get branch
    let target_branch = match branch {
        Some(b) => b.to_string(),
        None => {
            let output = CmdBuilder::new("git")
                .args(["branch", "--show-current"])
                .cwd(&ctx.repo)
                .capture_stdout()
                .run_capture()?;
            output.stdout_string().trim().to_string()
        }
    };

    ctx.print_header(&format!("Triggering workflow: {}", wf));
    println!("Branch: {}", style(&target_branch).cyan());

    if !ctx.confirm("Trigger this workflow?", true)? {
        println!("Cancelled.");
        return Ok(());
    }

    let code = CmdBuilder::new("gh")
        .args(["workflow", "run", &wf, "--ref", &target_branch])
        .cwd(&ctx.repo)
        .run()?;

    if code != 0 {
        return Err(anyhow!("Failed to trigger workflow"));
    }

    ctx.print_success(&format!(
        "Workflow '{}' triggered on branch '{}'",
        wf, target_branch
    ));
    println!();
    println!("Run `./dev.sh ci watch` to follow the progress.");

    Ok(())
}

/// Re-run a failed workflow
pub fn ci_rerun(ctx: &AppContext, run_id: Option<&str>, failed_only: bool) -> Result<()> {
    ensure_gh()?;

    let run = match run_id {
        Some(id) => id.to_string(),
        None => {
            // Get the most recent failed run
            let output = CmdBuilder::new("gh")
                .args([
                    "run",
                    "list",
                    "--status",
                    "failure",
                    "--limit",
                    "5",
                    "--json",
                    "databaseId,displayTitle,headBranch,createdAt",
                ])
                .cwd(&ctx.repo)
                .capture_stdout()
                .run_capture()?;

            let runs: Vec<serde_json::Value> =
                serde_json::from_str(&output.stdout_string()).unwrap_or_default();

            if runs.is_empty() {
                return Err(anyhow!("No failed workflow runs found"));
            }

            let choices: Vec<String> = runs
                .iter()
                .map(|r| {
                    let title = r["displayTitle"].as_str().unwrap_or("Unknown");
                    let branch = r["headBranch"].as_str().unwrap_or("?");
                    format!("{} ({})", title, branch)
                })
                .collect();

            let selection = Select::with_theme(&ctx.theme())
                .with_prompt("Select a run to re-run")
                .items(&choices)
                .default(0)
                .interact()?;

            runs[selection]["databaseId"]
                .as_i64()
                .map(|id| id.to_string())
                .ok_or_else(|| anyhow!("Failed to get run ID"))?
        }
    };

    ctx.print_header(&format!("Re-running workflow {}", run));

    let mut args = vec!["run".to_string(), "rerun".to_string(), run.clone()];
    if failed_only {
        args.push("--failed".to_string());
    }

    let code = CmdBuilder::new("gh").args(&args).cwd(&ctx.repo).run()?;

    if code != 0 {
        return Err(anyhow!("Failed to re-run workflow"));
    }

    ctx.print_success("Workflow re-run triggered");
    println!("Run `./dev.sh ci watch` to follow the progress.");

    Ok(())
}

/// Cancel a running workflow
pub fn ci_cancel(ctx: &AppContext, run_id: Option<&str>) -> Result<()> {
    ensure_gh()?;

    let run = match run_id {
        Some(id) => id.to_string(),
        None => {
            // Get in-progress runs
            let output = CmdBuilder::new("gh")
                .args([
                    "run",
                    "list",
                    "--status",
                    "in_progress",
                    "--json",
                    "databaseId,displayTitle,headBranch",
                ])
                .cwd(&ctx.repo)
                .capture_stdout()
                .run_capture()?;

            let runs: Vec<serde_json::Value> =
                serde_json::from_str(&output.stdout_string()).unwrap_or_default();

            if runs.is_empty() {
                return Err(anyhow!("No in-progress workflow runs to cancel"));
            }

            let choices: Vec<String> = runs
                .iter()
                .map(|r| {
                    let title = r["displayTitle"].as_str().unwrap_or("Unknown");
                    let branch = r["headBranch"].as_str().unwrap_or("?");
                    format!("{} ({})", title, branch)
                })
                .collect();

            let selection = Select::with_theme(&ctx.theme())
                .with_prompt("Select a run to cancel")
                .items(&choices)
                .default(0)
                .interact()?;

            runs[selection]["databaseId"]
                .as_i64()
                .map(|id| id.to_string())
                .ok_or_else(|| anyhow!("Failed to get run ID"))?
        }
    };

    if !ctx.confirm(&format!("Cancel workflow run {}?", run), false)? {
        println!("Cancelled.");
        return Ok(());
    }

    let code = CmdBuilder::new("gh")
        .args(["run", "cancel", &run])
        .cwd(&ctx.repo)
        .run()?;

    if code != 0 {
        return Err(anyhow!("Failed to cancel workflow"));
    }

    ctx.print_success("Workflow cancelled");
    Ok(())
}

/// Interactive CI menu
pub fn ci_menu(ctx: &AppContext) -> Result<()> {
    ensure_gh()?;

    let actions = vec![
        "Status (current branch)",
        "List recent runs",
        "View run logs",
        "Watch running workflow",
        "Trigger workflow",
        "Re-run failed workflow",
        "Cancel running workflow",
        "Back",
    ];

    let choice = Select::with_theme(&ctx.theme())
        .with_prompt("CI/CD Actions")
        .items(&actions)
        .default(0)
        .interact()?;

    match choice {
        0 => ci_status(ctx),
        1 => ci_runs(ctx, 10, None),
        2 => ci_logs(ctx, None),
        3 => ci_watch(ctx, None),
        4 => ci_trigger(ctx, None, None),
        5 => ci_rerun(ctx, None, true),
        6 => ci_cancel(ctx, None),
        _ => Ok(()),
    }
}
