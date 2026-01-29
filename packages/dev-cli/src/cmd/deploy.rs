//! Deployment commands using Pulumi

use anyhow::{anyhow, Result};
use console::style;
use dialoguer::{MultiSelect, Select};

use crate::cmd_builder::CmdBuilder;
use crate::config::Config;
use crate::context::AppContext;
use crate::utils::cmd_exists;

/// Deploy to an environment with config
pub fn deploy_with_config(
    ctx: &AppContext,
    config: Option<&Config>,
    env: &str,
    stacks: &[String],
    skip_preview: bool,
) -> Result<()> {
    if !cmd_exists("pulumi") {
        return Err(anyhow!(
            "Pulumi CLI not found. Install from: https://www.pulumi.com/docs/install/"
        ));
    }

    // Validate environment against config
    let valid_envs: Vec<&str> = config
        .map(|c| {
            c.global
                .environments
                .available
                .iter()
                .map(|s| s.as_str())
                .collect()
        })
        .unwrap_or_else(|| vec!["dev", "prod"]);

    if !valid_envs.contains(&env) {
        return Err(anyhow!(
            "Invalid environment: {}. Available: {}",
            env,
            valid_envs.join(", ")
        ));
    }

    // Discover available stacks from config
    let available_stacks: Vec<String> = config
        .map(|c| c.discover_infra_stacks().unwrap_or_default())
        .unwrap_or_else(|| vec!["core".to_string(), "api".to_string(), "app".to_string()]);

    // Determine which stacks to deploy
    let stacks_to_deploy: Vec<String> = if stacks.is_empty() {
        available_stacks.clone()
    } else {
        stacks.to_vec()
    };

    // Validate stacks
    for stack in &stacks_to_deploy {
        if !available_stacks.contains(stack) {
            return Err(anyhow!(
                "Invalid stack: {}. Available stacks: {}",
                stack,
                available_stacks.join(", ")
            ));
        }
    }

    ctx.print_header(&format!(
        "Deploying to {} environment",
        style(env).cyan().bold()
    ));
    println!("Stacks: {}", stacks_to_deploy.join(", "));
    let project_name = config
        .map(|c| c.global.project.name.as_str())
        .unwrap_or("shaya");
    println!();

    // Production safety check (skip in quiet mode or with --yes)
    if env == "prod" && !skip_preview && !ctx.quiet {
        println!(
            "{}",
            style("⚠️  WARNING: You are deploying to PRODUCTION")
                .red()
                .bold()
        );
        println!();

        if !ctx.confirm("Are you sure you want to deploy to production?", false)? {
            println!("Deployment cancelled.");
            return Ok(());
        }
        println!();
    }

    // Run preview first (unless skipped with --yes or in quiet mode)
    if !skip_preview && !ctx.quiet {
        ctx.print_header("Running preview...");
        println!();

        for stack in &stacks_to_deploy {
            let stack_name = format!("{}/{}/{}", project_name, stack, env);
            let infra_dir = ctx.repo.join(format!("infra/packages/{}", stack));

            println!("{}", style(format!("Preview: {}", stack_name)).bold());

            let preview_result = CmdBuilder::new("pulumi")
                .args(["preview", "-s", &stack_name, "--diff"])
                .cwd(&infra_dir)
                .run();

            if let Err(e) = preview_result {
                eprintln!("Preview failed for {}: {}", stack, e);
                return Err(anyhow!("Preview failed"));
            }

            println!();
        }

        // Confirm deployment after preview
        if !ctx.confirm("Proceed with deployment?", false)? {
            println!("Deployment cancelled.");
            return Ok(());
        }
        println!();
    }

    // Deploy each stack in order
    ctx.print_header("Deploying...");

    let mut success_count = 0;
    let mut failed_stacks: Vec<String> = Vec::new();

    for stack in &stacks_to_deploy {
        let stack_name = format!("{}/{}/{}", project_name, stack, env);
        let infra_dir = ctx.repo.join(format!("infra/packages/{}", stack));

        println!();
        println!(
            "{}",
            style(format!("Deploying {} to {}...", stack, env)).bold()
        );

        let result = CmdBuilder::new("pulumi")
            .args(["up", "-s", &stack_name, "--yes"])
            .cwd(&infra_dir)
            .run();

        match result {
            Ok(0) => {
                success_count += 1;
                println!(
                    "{}",
                    style(format!("✓ {} deployed successfully", stack)).green()
                );
            }
            Ok(code) => {
                failed_stacks.push(stack.clone());
                eprintln!(
                    "{}",
                    style(format!(
                        "✗ {} deployment failed (exit code {})",
                        stack, code
                    ))
                    .red()
                );
            }
            Err(e) => {
                failed_stacks.push(stack.clone());
                eprintln!(
                    "{}",
                    style(format!("✗ {} deployment error: {}", stack, e)).red()
                );
            }
        }
    }

    println!();

    if failed_stacks.is_empty() {
        ctx.print_success(&format!(
            "Successfully deployed {} stack(s) to {}",
            success_count, env
        ));
    } else {
        eprintln!(
            "{}",
            style(format!(
                "Deployment completed with errors. Failed stacks: {}",
                failed_stacks.join(", ")
            ))
            .red()
        );
        return Err(anyhow!("Some deployments failed"));
    }

    Ok(())
}

/// Show deployment preview without deploying
pub fn preview(
    ctx: &AppContext,
    config: Option<&Config>,
    env: &str,
    stacks: &[String],
) -> Result<()> {
    if !cmd_exists("pulumi") {
        return Err(anyhow!(
            "Pulumi CLI not found. Install from: https://www.pulumi.com/docs/install/"
        ));
    }

    // Discover available stacks from config
    let available_stacks: Vec<String> = config
        .map(|c| c.discover_infra_stacks().unwrap_or_default())
        .unwrap_or_else(|| vec!["core".to_string(), "api".to_string(), "app".to_string()]);

    let stacks_to_preview: Vec<String> = if stacks.is_empty() {
        available_stacks
    } else {
        stacks.to_vec()
    };

    let project_name = config
        .map(|c| c.global.project.name.as_str())
        .unwrap_or("shaya");

    ctx.print_header(&format!("Preview for {} environment", env));
    println!();

    for stack in &stacks_to_preview {
        let stack_name = format!("{}/{}/{}", project_name, stack, env);
        let infra_dir = ctx.repo.join(format!("infra/packages/{}", stack));

        println!("{}", style(format!("Stack: {}", stack_name)).bold());

        CmdBuilder::new("pulumi")
            .args(["preview", "-s", &stack_name, "--diff"])
            .cwd(&infra_dir)
            .run()?;

        println!();
    }

    Ok(())
}

/// Show stack outputs
pub fn show_outputs(
    ctx: &AppContext,
    config: Option<&Config>,
    env: &str,
    stack: Option<&str>,
) -> Result<()> {
    if !cmd_exists("pulumi") {
        return Err(anyhow!("Pulumi CLI not found."));
    }

    // Discover available stacks from config
    let available_stacks: Vec<String> = config
        .map(|c| c.discover_infra_stacks().unwrap_or_default())
        .unwrap_or_else(|| vec!["core".to_string(), "api".to_string(), "app".to_string()]);

    let stacks_to_show: Vec<String> = match stack {
        Some(s) => vec![s.to_string()],
        None => available_stacks,
    };

    let project_name = config
        .map(|c| c.global.project.name.as_str())
        .unwrap_or("shaya");

    ctx.print_header(&format!("Stack outputs for {} environment", env));
    println!();

    for stack in &stacks_to_show {
        let stack_name = format!("{}/{}/{}", project_name, stack, env);
        let infra_dir = ctx.repo.join(format!("infra/packages/{}", stack));

        println!("{}", style(format!("[{}]", stack)).bold());

        let result = CmdBuilder::new("pulumi")
            .args(["stack", "output", "-s", &stack_name, "--json"])
            .cwd(&infra_dir)
            .capture_stdout()
            .run_capture();

        match result {
            Ok(output) if output.code == 0 => {
                let stdout = output.stdout_string();
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    if let Some(obj) = json.as_object() {
                        for (key, value) in obj {
                            let display_val = match value {
                                serde_json::Value::String(s) => s.clone(),
                                other => other.to_string(),
                            };
                            println!("  {}: {}", key, display_val);
                        }
                    }
                } else {
                    println!("  (no outputs)");
                }
            }
            _ => {
                println!("  (failed to fetch outputs)");
            }
        }
        println!();
    }

    Ok(())
}

/// Refresh stack state from cloud provider
pub fn refresh(
    ctx: &AppContext,
    config: Option<&Config>,
    env: &str,
    stacks: &[String],
) -> Result<()> {
    if !cmd_exists("pulumi") {
        return Err(anyhow!("Pulumi CLI not found."));
    }

    // Discover available stacks from config
    let available_stacks: Vec<String> = config
        .map(|c| c.discover_infra_stacks().unwrap_or_default())
        .unwrap_or_else(|| vec!["core".to_string(), "api".to_string(), "app".to_string()]);

    let stacks_to_refresh: Vec<String> = if stacks.is_empty() {
        available_stacks
    } else {
        stacks.to_vec()
    };

    let project_name = config
        .map(|c| c.global.project.name.as_str())
        .unwrap_or("shaya");

    ctx.print_header(&format!("Refreshing {} environment", env));

    for stack in &stacks_to_refresh {
        let stack_name = format!("{}/{}/{}", project_name, stack, env);
        let infra_dir = ctx.repo.join(format!("infra/packages/{}", stack));

        println!();
        println!("{}", style(format!("Refreshing {}...", stack_name)).bold());

        CmdBuilder::new("pulumi")
            .args(["refresh", "-s", &stack_name, "--yes"])
            .cwd(&infra_dir)
            .run()?;
    }

    ctx.print_success("Refresh complete");
    Ok(())
}

/// Tail CloudWatch logs for a service with config
pub fn cloudwatch_logs_with_config(
    ctx: &AppContext,
    config: Option<&Config>,
    env: &str,
    service: Option<&str>,
    follow: bool,
    since: Option<&str>,
) -> Result<()> {
    if !cmd_exists("aws") {
        return Err(anyhow!(
            "AWS CLI not found. Install from: https://aws.amazon.com/cli/"
        ));
    }

    // Validate environment against config
    let valid_envs: Vec<&str> = config
        .map(|c| {
            c.global
                .environments
                .available
                .iter()
                .map(|s| s.as_str())
                .collect()
        })
        .unwrap_or_else(|| vec!["dev", "prod"]);

    if !valid_envs.contains(&env) {
        return Err(anyhow!(
            "Invalid environment: {}. Available: {}",
            env,
            valid_envs.join(", ")
        ));
    }

    // Get deployable packages from config (those with ecs_service defined)
    let deployable: Vec<(&str, &str)> = config
        .map(|c| {
            c.deployable_packages()
                .iter()
                .filter_map(|pkg| {
                    pkg.ecs_service
                        .as_ref()
                        .map(|svc| (pkg.name.as_str(), svc.as_str()))
                })
                .collect()
        })
        .unwrap_or_default();

    if deployable.is_empty() {
        return Err(anyhow!("No deployable packages found. Add 'ecs_service' to dev.toml for packages you want to view logs for."));
    }

    // Select service if not provided
    let (svc_name, ecs_service) = match service {
        Some(s) => {
            let found = deployable
                .iter()
                .find(|(name, _)| *name == s)
                .ok_or_else(|| {
                    anyhow!(
                        "Unknown service: {}. Available: {}",
                        s,
                        deployable
                            .iter()
                            .map(|(n, _)| *n)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })?;
            (found.0.to_string(), found.1.to_string())
        }
        None => {
            let service_names: Vec<&str> = deployable.iter().map(|(name, _)| *name).collect();
            let choice = Select::with_theme(&ctx.theme())
                .with_prompt("Select service")
                .items(&service_names)
                .default(0)
                .interact()?;
            let (name, svc) = deployable[choice];
            (name.to_string(), svc.to_string())
        }
    };

    // Build log group name from ECS service
    let log_group = format!("/ecs/{}-{}", ecs_service, env);

    ctx.print_header(&format!("CloudWatch Logs: {} ({})", svc_name, env));
    println!("Log group: {}", style(&log_group).dim());
    println!();

    let mut args = vec!["logs".to_string(), "tail".to_string(), log_group];

    if follow {
        args.push("--follow".to_string());
    }

    if let Some(s) = since {
        args.push("--since".to_string());
        args.push(s.to_string());
    } else {
        // Default to last 30 minutes
        args.push("--since".to_string());
        args.push("30m".to_string());
    }

    // Add color output
    args.push("--format".to_string());
    args.push("short".to_string());

    let code = CmdBuilder::new("aws").args(&args).cwd(&ctx.repo).run()?;

    if code != 0 {
        return Err(anyhow!("Failed to tail logs"));
    }

    Ok(())
}

/// List CloudWatch log groups
pub fn list_log_groups(ctx: &AppContext, env: &str) -> Result<()> {
    if !cmd_exists("aws") {
        return Err(anyhow!("AWS CLI not found."));
    }

    ctx.print_header(&format!("Log Groups ({})", env));

    let prefix = "/ecs/shaya-";

    let code = CmdBuilder::new("aws")
        .args([
            "logs",
            "describe-log-groups",
            "--log-group-name-prefix",
            prefix,
            "--query",
            "logGroups[*].[logGroupName,storedBytes]",
            "--output",
            "table",
        ])
        .cwd(&ctx.repo)
        .run()?;

    if code != 0 {
        return Err(anyhow!("Failed to list log groups"));
    }

    Ok(())
}

/// Interactive CloudWatch logs menu with config
pub fn logs_cloudwatch_menu_with_config(ctx: &AppContext, config: Option<&Config>) -> Result<()> {
    if !cmd_exists("aws") {
        return Err(anyhow!(
            "AWS CLI not found. Install from: https://aws.amazon.com/cli/"
        ));
    }

    // Select environment from config or defaults
    let envs: Vec<&str> = config
        .map(|c| {
            c.global
                .environments
                .available
                .iter()
                .map(|s| s.as_str())
                .collect()
        })
        .unwrap_or_else(|| vec!["dev", "prod"]);
    let env_choice = Select::with_theme(&ctx.theme())
        .with_prompt("Select environment")
        .items(&envs)
        .default(0)
        .interact()?;
    let env = envs[env_choice];

    // Select action
    let actions = vec![
        "Tail logs (follow)",
        "View recent logs",
        "List log groups",
        "Back",
    ];

    let action_choice = Select::with_theme(&ctx.theme())
        .with_prompt("What do you want to do?")
        .items(&actions)
        .default(0)
        .interact()?;

    match action_choice {
        0 => cloudwatch_logs_with_config(ctx, config, env, None, true, None),
        1 => cloudwatch_logs_with_config(ctx, config, env, None, false, Some("1h")),
        2 => list_log_groups(ctx, env),
        _ => Ok(()),
    }
}

/// Interactive deploy menu with config
pub fn deploy_menu_with_config(ctx: &AppContext, config: Option<&Config>) -> Result<()> {
    if !cmd_exists("pulumi") {
        return Err(anyhow!(
            "Pulumi CLI not found. Install from: https://www.pulumi.com/docs/install/"
        ));
    }

    // Select environment from config or defaults
    let envs: Vec<&str> = config
        .map(|c| {
            c.global
                .environments
                .available
                .iter()
                .map(|s| s.as_str())
                .collect()
        })
        .unwrap_or_else(|| vec!["dev", "prod"]);
    let env_choice = Select::with_theme(&ctx.theme())
        .with_prompt("Select environment")
        .items(&envs)
        .default(0)
        .interact()?;
    let env = envs[env_choice];

    // Select action
    let actions = vec![
        "Deploy (with preview)",
        "Preview only",
        "Show outputs",
        "Refresh state",
        "Cancel",
    ];
    let action_choice = Select::with_theme(&ctx.theme())
        .with_prompt("What do you want to do?")
        .items(&actions)
        .default(0)
        .interact()?;

    match action_choice {
        0 => {
            // Deploy with stack selection - discover from config
            let stack_items: Vec<String> = config
                .map(|c| c.discover_infra_stacks().unwrap_or_default())
                .unwrap_or_else(|| vec!["core".to_string(), "api".to_string(), "app".to_string()]);
            let defaults = vec![true; stack_items.len()];

            let selections = MultiSelect::with_theme(&ctx.theme())
                .with_prompt("Select stacks to deploy")
                .items(&stack_items)
                .defaults(&defaults)
                .interact()?;

            if selections.is_empty() {
                println!("No stacks selected.");
                return Ok(());
            }

            let selected_stacks: Vec<String> =
                selections.iter().map(|&i| stack_items[i].clone()).collect();

            deploy_with_config(ctx, config, env, &selected_stacks, false)
        }
        1 => preview(ctx, config, env, &[]),
        2 => show_outputs(ctx, config, env, None),
        3 => refresh(ctx, config, env, &[]),
        _ => Ok(()),
    }
}
