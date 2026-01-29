//! Run GitHub Actions locally via `act`.

use anyhow::{bail, Context, Result};
use dialoguer::Select;
use std::fs;
use std::path::Path;

use crate::cmd_builder::CmdBuilder;
use crate::config::Config;
use crate::context::AppContext;
use crate::utils::cmd_exists;

/// Run GitHub Actions workflow locally using `act`.
///
/// # Arguments
/// * `ctx` - Application context
/// * `config` - Loaded configuration
/// * `workflow` - Workflow name (ci, coverage, deploy)
/// * `job` - Optional specific job to run
/// * `env` - Optional environment for secrets (dev, prod)
/// * `extra_args` - Additional arguments to pass to act
pub fn run_local(
    ctx: &AppContext,
    config: &Config,
    workflow: Option<&str>,
    job: Option<&str>,
    env: Option<&str>,
    extra_args: Vec<String>,
) -> Result<()> {
    // Ensure act is installed
    if !cmd_exists("act") {
        bail!(
            "act is not installed. Install it with:\n\
             macOS: brew install act\n\
             Linux: curl -fsSL https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash"
        );
    }

    // Check Docker is running
    let docker_status = std::process::Command::new("docker")
        .args(["info"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    if docker_status.map(|s| !s.success()).unwrap_or(true) {
        bail!("Docker is not running. Please start Docker first.");
    }

    // Show help / list workflows if no args
    let workflow = match workflow {
        Some(w) if w != "-h" && w != "--help" => w.to_string(),
        _ => {
            print_local_help(ctx)?;
            return Ok(());
        }
    };

    // Validate workflow exists
    let workflow_file = ctx.repo.join(format!(".github/workflows/{}.yml", workflow));
    if !workflow_file.exists() {
        let available = list_workflows(&ctx.repo)?;
        bail!(
            "Workflow '{}' not found.\nAvailable: {}",
            workflow,
            available.join(", ")
        );
    }

    // Determine environment
    let env_name = match env {
        Some(e) => e.to_string(),
        None => prompt_environment(ctx, config)?,
    };

    let env_file = ctx.repo.join(format!(".env.{}", env_name));

    // Check if env file exists, offer to pull if not
    if !env_file.exists() {
        ctx.print_warning(&format!(".env.{} not found.", env_name));
        if cmd_exists("esc") {
            if ctx.confirm("Pull from ESC?", true)? {
                pull_env_from_esc(ctx, config, &env_name, &env_file)?;
            } else {
                ctx.print_warning("Continuing without secrets (some tests may fail)");
            }
        } else {
            ctx.print_warning(
                "'esc' CLI not installed. Install it to pull secrets from Pulumi ESC.",
            );
            ctx.print_warning("Continuing without secrets (some tests may fail)");
        }
    }

    // Build act command
    let act_config = &config.global.act;
    let artifact_dir = ctx.repo.join(&act_config.artifact_dir);
    fs::create_dir_all(&artifact_dir)?;

    let mut cmd = CmdBuilder::new("act")
        .arg("-W")
        .arg(workflow_file.to_string_lossy())
        .arg("--container-architecture")
        .arg(&act_config.container_architecture)
        .arg("-P")
        .arg(format!("ubuntu-latest={}", act_config.docker_image))
        .arg("--artifact-server-path")
        .arg(artifact_dir.to_string_lossy())
        .cwd(&ctx.repo);

    // Add job filter if specified
    if let Some(j) = job {
        cmd = cmd.arg("-j").arg(j);
    }

    // Add secret file if it exists
    if env_file.exists() {
        cmd = cmd.arg("--secret-file").arg(env_file.to_string_lossy());
    }

    // Add extra args
    for arg in extra_args {
        cmd = cmd.arg(&arg);
    }

    ctx.print_header(&format!(
        "Running {} workflow locally ({})",
        workflow, env_name
    ));
    println!();

    let code = cmd.run()?;

    if code != 0 {
        bail!("Local CI run failed with code {}", code);
    }

    Ok(())
}

/// Print help for the local command
fn print_local_help(ctx: &AppContext) -> Result<()> {
    println!("Run GitHub Actions locally");
    println!();
    println!("Usage: ./dev.sh local [workflow] [job] [options]");
    println!();
    println!("Arguments:");
    println!("  workflow    Workflow name: ci, coverage, deploy");
    println!("  job         Specific job to run (optional)");
    println!();
    println!("Options:");
    println!("  -e, --env ENV    Environment for secrets: dev or prod (will prompt if not set)");
    println!();
    println!("Options (passed to act):");
    println!("  -v, --verbose    Verbose output");
    println!("  -l, --list       List jobs in workflow");
    println!("  -n, --dryrun     Dry run");
    println!();
    println!("Examples:");
    println!("  ./dev.sh local                   # Show this help");
    println!("  ./dev.sh local ci                # Run CI workflow (prompts for env)");
    println!("  ./dev.sh local ci -e dev         # Run CI with dev secrets");
    println!("  ./dev.sh local ci check -e prod  # Run check job with prod secrets");
    println!("  ./dev.sh local ci test -v        # Run test with verbose output");
    println!("  ./dev.sh local ci -l             # List CI jobs");
    println!("  ./dev.sh local coverage -e dev   # Run coverage workflow");
    println!();
    println!("Available workflows:");

    for workflow in list_workflows(&ctx.repo)? {
        println!("  - {}", workflow);
    }

    Ok(())
}

/// List available workflows
fn list_workflows(repo_root: &Path) -> Result<Vec<String>> {
    let workflows_dir = repo_root.join(".github/workflows");

    if !workflows_dir.exists() {
        return Ok(Vec::new());
    }

    let mut workflows = Vec::new();
    for entry in fs::read_dir(&workflows_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "yml").unwrap_or(false) {
            if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                workflows.push(name.to_string());
            }
        }
    }

    workflows.sort();
    Ok(workflows)
}

/// Prompt user to select environment
fn prompt_environment(ctx: &AppContext, config: &Config) -> Result<String> {
    let envs = &config.global.environments.available;

    let choice = Select::with_theme(&ctx.theme())
        .with_prompt("Select environment for secrets")
        .items(envs)
        .default(0)
        .interact()?;

    Ok(envs[choice].clone())
}

/// Pull environment from ESC
fn pull_env_from_esc(
    ctx: &AppContext,
    config: &Config,
    env: &str,
    output_path: &Path,
) -> Result<()> {
    let esc_path = config.esc_path(env);

    ctx.print_header(&format!("Pulling {} environment from ESC...", env));

    let output = std::process::Command::new("esc")
        .args([
            "env",
            "get",
            &esc_path,
            "--value",
            "dotenv",
            "--show-secrets",
        ])
        .output()
        .context("Failed to run esc")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to pull from ESC: {}", stderr);
    }

    fs::write(output_path, &output.stdout)
        .with_context(|| format!("Failed to write {}", output_path.display()))?;

    ctx.print_success(&format!("Wrote {}", output_path.display()));
    Ok(())
}

/// Interactive menu for local CI runs
pub fn local_menu(ctx: &AppContext, config: &Config) -> Result<()> {
    let workflows = list_workflows(&ctx.repo)?;

    if workflows.is_empty() {
        bail!("No workflows found in .github/workflows/");
    }

    let workflow_choice = Select::with_theme(&ctx.theme())
        .with_prompt("Select workflow")
        .items(&workflows)
        .default(0)
        .interact()?;

    let env_name = prompt_environment(ctx, config)?;

    run_local(
        ctx,
        config,
        Some(&workflows[workflow_choice]),
        None,
        Some(&env_name),
        Vec::new(),
    )
}
