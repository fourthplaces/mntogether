//! Job queue debugging operations
//!
//! Provides commands for inspecting and managing the job queue on remote environments.

use anyhow::{anyhow, Result};
use console::style;
use dialoguer::{Confirm, Select};
use serde::Deserialize;

use crate::cmd_builder::CmdBuilder;
use crate::context::AppContext;

/// Job status from database
#[derive(Debug, Deserialize)]
struct JobStats {
    status: String,
    count: i64,
}

/// Job record from database
#[derive(Debug, Deserialize)]
struct JobRecord {
    id: String,
    job_type: String,
    status: String,
    reference_id: String,
    retry_count: i32,
    max_retries: i32,
    created_at: String,
    #[serde(default)]
    last_run_at: Option<String>,
    #[serde(default)]
    error_message: Option<String>,
}

/// Get cluster name for an environment
fn get_cluster_name(env: &str) -> String {
    format!("api-cluster-{}", env)
}

/// Get service name for an environment
fn get_service_name(env: &str) -> String {
    format!("api-service-{}", env)
}

/// Run a SQL query via ECS exec and return the result
fn run_sql_query(cluster: &str, service: &str, query: &str) -> Result<String> {
    // First, get a running task
    let task_output = std::process::Command::new("aws")
        .args([
            "ecs",
            "list-tasks",
            "--cluster",
            cluster,
            "--service-name",
            service,
            "--desired-status",
            "RUNNING",
            "--output",
            "json",
        ])
        .output()?;

    if !task_output.status.success() {
        let stderr = String::from_utf8_lossy(&task_output.stderr);
        return Err(anyhow!("Failed to list ECS tasks: {}", stderr));
    }

    #[derive(Deserialize)]
    struct TaskList {
        #[serde(rename = "taskArns")]
        task_arns: Vec<String>,
    }

    let task_list: TaskList = serde_json::from_slice(&task_output.stdout)?;
    let task_arn = task_list
        .task_arns
        .first()
        .ok_or_else(|| anyhow!("No running tasks found"))?;

    let task_id = task_arn.rsplit('/').next().unwrap_or(task_arn);

    // Run the query via ECS exec
    let sql_cmd = format!(
        r#"psql "$DATABASE_URL" -t -A -F'|' -c "{}""#,
        query.replace('"', r#"\""#)
    );

    let output = std::process::Command::new("aws")
        .args([
            "ecs",
            "execute-command",
            "--cluster",
            cluster,
            "--task",
            task_id,
            "--container",
            "api",
            "--command",
            &format!("/bin/sh -c '{}'", sql_cmd),
            "--interactive",
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Query failed: {}", stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Show job queue statistics
pub fn jobs_stats(ctx: &AppContext, env: &str) -> Result<()> {
    ctx.print_header(&format!("Job Queue Stats: {}", env));

    let cluster = get_cluster_name(env);
    let service = get_service_name(env);

    println!("Querying job statistics...");
    println!();

    // Query job counts by status
    let query = r#"
        SELECT
            status::text,
            COUNT(*)::text
        FROM jobs
        GROUP BY status
        ORDER BY status
    "#;

    let result = run_sql_query(&cluster, &service, query)?;

    println!("{}", style("Job counts by status:").cyan().bold());
    println!("{}", style("─".repeat(40)).dim());

    for line in result.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 2 {
            let status = parts[0].trim();
            let count = parts[1].trim();
            let status_styled = match status {
                "pending" => style(status).yellow(),
                "running" => style(status).blue(),
                "succeeded" => style(status).green(),
                "failed" => style(status).red(),
                "dead_letter" => style(status).red().bold(),
                _ => style(status).white(),
            };
            println!("  {:<15} {}", status_styled, count);
        }
    }

    println!();

    // Query recent pending jobs
    let pending_query = r#"
        SELECT
            id::text,
            job_type,
            status::text,
            reference_id::text,
            retry_count,
            max_retries,
            to_char(created_at, 'YYYY-MM-DD HH24:MI:SS'),
            COALESCE(to_char(last_run_at, 'YYYY-MM-DD HH24:MI:SS'), 'never')
        FROM jobs
        WHERE status = 'pending'
        ORDER BY created_at DESC
        LIMIT 10
    "#;

    let pending_result = run_sql_query(&cluster, &service, pending_query)?;

    println!("{}", style("Recent pending jobs:").cyan().bold());
    println!("{}", style("─".repeat(80)).dim());

    let mut has_pending = false;
    for line in pending_result.lines() {
        if line.trim().is_empty() {
            continue;
        }
        has_pending = true;
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 8 {
            let id = &parts[0][..8]; // Short ID
            let job_type = parts[1];
            let retry = parts[4];
            let max_retry = parts[5];
            let created = parts[6];
            let last_run = parts[7];

            println!(
                "  {} {} retries={}/{} created={} last_run={}",
                style(id).dim(),
                style(job_type).cyan(),
                retry,
                max_retry,
                created,
                if last_run == "never" {
                    style(last_run).red().to_string()
                } else {
                    last_run.to_string()
                }
            );
        }
    }

    if !has_pending {
        println!("  {}", style("No pending jobs").dim());
    }

    Ok(())
}

/// Show stuck jobs (pending with no last_run_at)
pub fn jobs_stuck(ctx: &AppContext, env: &str) -> Result<()> {
    ctx.print_header(&format!("Stuck Jobs: {}", env));

    let cluster = get_cluster_name(env);
    let service = get_service_name(env);

    println!("Querying stuck jobs...");
    println!();

    // Query jobs that are pending but have never been picked up
    let query = r#"
        SELECT
            id::text,
            job_type,
            reference_id::text,
            retry_count,
            max_retries,
            to_char(created_at, 'YYYY-MM-DD HH24:MI:SS'),
            COALESCE(error_message, '')
        FROM jobs
        WHERE status = 'pending'
          AND last_run_at IS NULL
          AND created_at < NOW() - INTERVAL '5 minutes'
        ORDER BY created_at ASC
        LIMIT 20
    "#;

    let result = run_sql_query(&cluster, &service, query)?;

    println!(
        "{}",
        style("Jobs pending for >5 min with no execution attempt:")
            .yellow()
            .bold()
    );
    println!("{}", style("─".repeat(80)).dim());

    let mut count = 0;
    for line in result.lines() {
        if line.trim().is_empty() {
            continue;
        }
        count += 1;
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 6 {
            let id = &parts[0][..8];
            let job_type = parts[1];
            let ref_id = &parts[2][..8];
            let created = parts[5];

            println!(
                "  {} {} ref={} created={}",
                style(id).dim(),
                style(job_type).cyan(),
                style(ref_id).dim(),
                created,
            );
        }
    }

    if count == 0 {
        println!("  {}", style("No stuck jobs found").green());
    } else {
        println!();
        println!(
            "{}",
            style(format!("Found {} stuck job(s)", count)).yellow()
        );
        println!();
        println!("Possible causes:");
        println!("  - Job scheduler not running");
        println!("  - NATS consumer not processing messages");
        println!("  - Handler not registered for job type");
    }

    Ok(())
}

/// Reset stuck jobs to be picked up again
pub fn jobs_reset_stuck(ctx: &AppContext, env: &str) -> Result<()> {
    ctx.print_header(&format!("Reset Stuck Jobs: {}", env));

    let cluster = get_cluster_name(env);
    let service = get_service_name(env);

    // First, count stuck jobs
    let count_query = r#"
        SELECT COUNT(*)::text
        FROM jobs
        WHERE status = 'pending'
          AND last_run_at IS NULL
          AND created_at < NOW() - INTERVAL '5 minutes'
    "#;

    let count_result = run_sql_query(&cluster, &service, count_query)?;
    let count: i64 = count_result.trim().parse().unwrap_or(0);

    if count == 0 {
        println!("{}", style("No stuck jobs found to reset").green());
        return Ok(());
    }

    println!(
        "Found {} stuck job(s) that will be reset.",
        style(count).yellow()
    );
    println!();
    println!("This will update next_run_at to NOW() to trigger immediate pickup.");

    if !ctx.confirm("Proceed with reset?", false)? {
        println!("Cancelled.");
        return Ok(());
    }

    // Reset the jobs
    let reset_query = r#"
        UPDATE jobs
        SET next_run_at = NOW(),
            updated_at = NOW()
        WHERE status = 'pending'
          AND last_run_at IS NULL
          AND created_at < NOW() - INTERVAL '5 minutes'
    "#;

    run_sql_query(&cluster, &service, reset_query)?;

    println!();
    ctx.print_success(&format!("Reset {} stuck job(s)", count));

    Ok(())
}

/// View failed jobs
pub fn jobs_failed(ctx: &AppContext, env: &str) -> Result<()> {
    ctx.print_header(&format!("Failed Jobs: {}", env));

    let cluster = get_cluster_name(env);
    let service = get_service_name(env);

    println!("Querying failed jobs...");
    println!();

    let query = r#"
        SELECT
            id::text,
            job_type,
            reference_id::text,
            retry_count,
            max_retries,
            to_char(created_at, 'YYYY-MM-DD HH24:MI:SS'),
            COALESCE(LEFT(error_message, 100), '')
        FROM jobs
        WHERE status IN ('failed', 'dead_letter')
        ORDER BY updated_at DESC
        LIMIT 15
    "#;

    let result = run_sql_query(&cluster, &service, query)?;

    println!("{}", style("Recent failed/dead-letter jobs:").red().bold());
    println!("{}", style("─".repeat(80)).dim());

    let mut count = 0;
    for line in result.lines() {
        if line.trim().is_empty() {
            continue;
        }
        count += 1;
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 7 {
            let id = &parts[0][..8];
            let job_type = parts[1];
            let retry = parts[3];
            let max_retry = parts[4];
            let error = parts[6];

            println!(
                "  {} {} retries={}/{}",
                style(id).dim(),
                style(job_type).cyan(),
                retry,
                max_retry,
            );
            if !error.is_empty() {
                println!("    {}", style(error).red().dim());
            }
        }
    }

    if count == 0 {
        println!("  {}", style("No failed jobs").green());
    }

    Ok(())
}

/// Jobs debugging menu
pub fn jobs_menu(ctx: &AppContext, env: &str) -> Result<()> {
    loop {
        let items = vec![
            "View queue stats",
            "View stuck jobs (pending, never run)",
            "View failed jobs",
            "Reset stuck jobs",
            "Back",
        ];

        let choice = Select::with_theme(&ctx.theme())
            .with_prompt(format!("Job Queue ({})", env))
            .items(&items)
            .default(0)
            .interact()?;

        match choice {
            0 => jobs_stats(ctx, env)?,
            1 => jobs_stuck(ctx, env)?,
            2 => jobs_failed(ctx, env)?,
            3 => jobs_reset_stuck(ctx, env)?,
            _ => return Ok(()),
        }

        println!();
    }
}
