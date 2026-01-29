//! ECS container operations (SSH/exec into running containers)
//!
//! Provides interactive shell access to ECS Fargate containers via ECS Exec.

use anyhow::{anyhow, Result};
use console::style;
use dialoguer::Select;
use serde::Deserialize;

use crate::cmd_builder::CmdBuilder;
use crate::config::Config;
use crate::context::AppContext;
use crate::utils::cmd_exists;

/// ECS task info from AWS CLI
#[derive(Debug, Deserialize)]
struct EcsTaskList {
    #[serde(rename = "taskArns")]
    task_arns: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct EcsTaskDescription {
    tasks: Vec<EcsTask>,
}

#[derive(Debug, Deserialize)]
struct EcsTask {
    #[serde(rename = "taskArn")]
    task_arn: String,
    #[serde(rename = "lastStatus")]
    last_status: String,
    containers: Vec<EcsContainer>,
    #[serde(rename = "startedAt", default)]
    started_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EcsContainer {
    name: String,
    #[serde(rename = "lastStatus")]
    last_status: String,
}

/// Get cluster name for an environment
fn get_cluster_name(env: &str) -> String {
    format!("api-cluster-{}", env)
}

/// Get service name for an environment
fn get_service_name(env: &str) -> String {
    format!("api-service-{}", env)
}

/// List running tasks in a cluster/service
fn list_tasks(cluster: &str, service: &str) -> Result<Vec<String>> {
    let output = std::process::Command::new("aws")
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

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to list ECS tasks: {}", stderr));
    }

    let task_list: EcsTaskList = serde_json::from_slice(&output.stdout)?;
    Ok(task_list.task_arns)
}

/// Get task details
fn describe_tasks(cluster: &str, task_arns: &[String]) -> Result<Vec<EcsTask>> {
    if task_arns.is_empty() {
        return Ok(vec![]);
    }

    let output = std::process::Command::new("aws")
        .args(["ecs", "describe-tasks", "--cluster", cluster, "--tasks"])
        .args(task_arns)
        .args(["--output", "json"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to describe ECS tasks: {}", stderr));
    }

    let description: EcsTaskDescription = serde_json::from_slice(&output.stdout)?;
    Ok(description.tasks)
}

/// Extract task ID from task ARN
fn task_id_from_arn(arn: &str) -> &str {
    arn.rsplit('/').next().unwrap_or(arn)
}

/// Open an interactive shell in an ECS container
pub fn ecs_exec(ctx: &AppContext, env: &str, container: Option<&str>) -> Result<()> {
    if !cmd_exists("aws") {
        return Err(anyhow!(
            "AWS CLI not found. Install from: https://aws.amazon.com/cli/"
        ));
    }

    // Check for Session Manager plugin
    let session_manager_check = std::process::Command::new("session-manager-plugin")
        .arg("--version")
        .output();

    if session_manager_check.is_err() {
        return Err(anyhow!(
            "Session Manager plugin not found.\n\
             Install from: https://docs.aws.amazon.com/systems-manager/latest/userguide/session-manager-working-with-install-plugin.html"
        ));
    }

    let cluster = get_cluster_name(env);
    let service = get_service_name(env);

    ctx.print_header(&format!("Connecting to {} environment...", env));
    println!("Cluster: {}", style(&cluster).cyan());
    println!("Service: {}", style(&service).cyan());
    println!();

    // List running tasks
    println!("Finding running tasks...");
    let task_arns = list_tasks(&cluster, &service)?;

    if task_arns.is_empty() {
        return Err(anyhow!(
            "No running tasks found in {}/{}. Is the service running?",
            cluster,
            service
        ));
    }

    // Get task details
    let tasks = describe_tasks(&cluster, &task_arns)?;
    let running_tasks: Vec<_> = tasks
        .iter()
        .filter(|t| t.last_status == "RUNNING")
        .collect();

    if running_tasks.is_empty() {
        return Err(anyhow!("No tasks in RUNNING state found"));
    }

    // Select task if multiple
    let selected_task = if running_tasks.len() == 1 {
        running_tasks[0]
    } else {
        let items: Vec<String> = running_tasks
            .iter()
            .map(|t| {
                let id = task_id_from_arn(&t.task_arn);
                let started = t.started_at.as_deref().unwrap_or("unknown");
                format!("{} (started: {})", id, started)
            })
            .collect();

        let choice = Select::with_theme(&ctx.theme())
            .with_prompt("Select task")
            .items(&items)
            .default(0)
            .interact()?;

        running_tasks[choice]
    };

    let task_id = task_id_from_arn(&selected_task.task_arn);

    // Select container if not specified
    let container_name = if let Some(c) = container {
        c.to_string()
    } else {
        let running_containers: Vec<_> = selected_task
            .containers
            .iter()
            .filter(|c| c.last_status == "RUNNING")
            .collect();

        if running_containers.is_empty() {
            return Err(anyhow!("No running containers in task"));
        }

        if running_containers.len() == 1 {
            running_containers[0].name.clone()
        } else {
            let items: Vec<&str> = running_containers.iter().map(|c| c.name.as_str()).collect();

            let choice = Select::with_theme(&ctx.theme())
                .with_prompt("Select container")
                .items(&items)
                .default(0)
                .interact()?;

            running_containers[choice].name.clone()
        }
    };

    println!();
    println!(
        "Connecting to {} in task {}...",
        style(&container_name).green(),
        style(task_id).cyan()
    );
    println!();
    println!("{}", style("Type 'exit' to disconnect").dim());
    println!();

    // Execute ECS Exec
    let code = CmdBuilder::new("aws")
        .args([
            "ecs",
            "execute-command",
            "--cluster",
            &cluster,
            "--task",
            task_id,
            "--container",
            &container_name,
            "--interactive",
            "--command",
            "/bin/sh",
        ])
        .inherit_io()
        .run()?;

    if code != 0 && code != 130 {
        // 130 is Ctrl+C
        return Err(anyhow!("ECS Exec exited with code {}", code));
    }

    Ok(())
}

/// Show ECS exec menu
pub fn ecs_exec_menu(ctx: &AppContext, config: Option<&Config>) -> Result<()> {
    // Get available environments from config or use defaults
    let envs: Vec<String> = config
        .map(|c| c.global.environments.available.clone())
        .unwrap_or_else(|| vec!["dev".to_string(), "prod".to_string()]);

    let env_strs: Vec<&str> = envs.iter().map(|s| s.as_str()).collect();

    let choice = Select::with_theme(&ctx.theme())
        .with_prompt("Select environment")
        .items(&env_strs)
        .default(0)
        .interact()?;

    let env = env_strs[choice];

    // Production safety check
    if env == "prod" && !ctx.quiet {
        println!();
        println!(
            "{}",
            style("WARNING: You are connecting to PRODUCTION")
                .red()
                .bold()
        );
        println!();

        if !ctx.confirm("Are you sure you want to continue?", false)? {
            println!("Cancelled.");
            return Ok(());
        }
    }

    ecs_exec(ctx, env, None)
}

/// Check health of ECS services
pub fn ecs_health(ctx: &AppContext, env: &str) -> Result<()> {
    if !cmd_exists("aws") {
        return Err(anyhow!("AWS CLI not found"));
    }

    let cluster = get_cluster_name(env);
    let service = get_service_name(env);

    ctx.print_header(&format!("ECS Health: {}", env));

    // Get service status
    let output = std::process::Command::new("aws")
        .args([
            "ecs",
            "describe-services",
            "--cluster",
            &cluster,
            "--services",
            &service,
            "--output",
            "json",
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to describe service: {}", stderr));
    }

    #[derive(Deserialize)]
    struct ServiceDescription {
        services: Vec<ServiceInfo>,
    }

    #[derive(Deserialize)]
    struct ServiceInfo {
        #[serde(rename = "serviceName")]
        service_name: String,
        status: String,
        #[serde(rename = "runningCount")]
        running_count: i32,
        #[serde(rename = "desiredCount")]
        desired_count: i32,
        #[serde(rename = "pendingCount")]
        pending_count: i32,
    }

    let desc: ServiceDescription = serde_json::from_slice(&output.stdout)?;

    if let Some(svc) = desc.services.first() {
        println!("Service: {}", style(&svc.service_name).cyan());
        println!("Status:  {}", style(&svc.status).green());
        println!(
            "Tasks:   {} running, {} desired, {} pending",
            style(svc.running_count).green(),
            svc.desired_count,
            svc.pending_count
        );

        if svc.running_count < svc.desired_count {
            println!();
            println!(
                "{}",
                style("WARNING: Running tasks below desired count!").yellow()
            );
        }
    } else {
        println!("{}", style("Service not found").red());
    }

    // List tasks
    println!();
    let task_arns = list_tasks(&cluster, &service)?;

    if task_arns.is_empty() {
        println!("{}", style("No running tasks").yellow());
    } else {
        let tasks = describe_tasks(&cluster, &task_arns)?;
        println!("Running tasks:");
        for task in &tasks {
            let id = task_id_from_arn(&task.task_arn);
            let started = task.started_at.as_deref().unwrap_or("unknown");
            println!(
                "  {} - {} (started: {})",
                style(id).cyan(),
                task.last_status,
                started
            );
            for container in &task.containers {
                println!("    └─ {} [{}]", container.name, container.last_status);
            }
        }
    }

    Ok(())
}
