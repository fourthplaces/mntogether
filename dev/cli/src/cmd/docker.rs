//! Docker commands for managing development services

use anyhow::{Context, Result};
use clap::Subcommand;
use console::style;
use devkit_core::AppContext;
use dialoguer::{FuzzySelect, MultiSelect};
use std::process::Command;

/// Available Docker services from docker-compose.yml
const SERVICES: &[(&str, &str)] = &[
    ("postgres", "PostgreSQL database with pgvector"),
    ("redis", "Redis cache and pub/sub"),
    ("nats", "NATS messaging server"),
    ("api", "Rust API server"),
    ("web-next", "Next.js web app (SSR)"),
];

#[derive(Subcommand)]
pub enum DockerCommand {
    /// Start services
    Up {
        /// Services to start (omit for interactive selection)
        #[arg(value_name = "SERVICE")]
        services: Vec<String>,

        /// Start all services
        #[arg(short, long)]
        all: bool,

        /// Include optional services (web-next)
        #[arg(long)]
        full: bool,

        /// Run in detached mode
        #[arg(short, long, default_value = "true")]
        detach: bool,
    },

    /// Stop services
    Down {
        /// Services to stop (omit for all)
        #[arg(value_name = "SERVICE")]
        services: Vec<String>,

        /// Remove volumes (WARNING: deletes data)
        #[arg(short, long)]
        volumes: bool,
    },

    /// Restart services
    Restart {
        /// Services to restart (omit for interactive selection)
        #[arg(value_name = "SERVICE")]
        services: Vec<String>,

        /// Restart all services
        #[arg(short, long)]
        all: bool,
    },

    /// Rebuild service images
    Build {
        /// Services to build (omit for interactive selection)
        #[arg(value_name = "SERVICE")]
        services: Vec<String>,

        /// Build all services
        #[arg(short, long)]
        all: bool,

        /// Don't use cache
        #[arg(long)]
        no_cache: bool,
    },

    /// Follow logs from services
    Logs {
        /// Services to follow (omit for interactive selection)
        #[arg(value_name = "SERVICE")]
        services: Vec<String>,

        /// Follow all services
        #[arg(short, long)]
        all: bool,

        /// Number of lines to show initially
        #[arg(short = 'n', long, default_value = "100")]
        tail: String,

        /// Don't follow, just show recent logs
        #[arg(long)]
        no_follow: bool,
    },

    /// Show status of all services
    Status,

    /// Open a shell in a service container
    Shell {
        /// Service to open shell in
        service: Option<String>,
    },

    /// Run psql in the postgres container
    Psql,
}

pub fn run(ctx: &AppContext, cmd: DockerCommand) -> Result<()> {
    match cmd {
        DockerCommand::Up {
            services,
            all,
            full,
            detach,
        } => run_up(ctx, services, all, full, detach),
        DockerCommand::Down { services, volumes } => run_down(ctx, services, volumes),
        DockerCommand::Restart { services, all } => run_restart(ctx, services, all),
        DockerCommand::Build {
            services,
            all,
            no_cache,
        } => run_build(ctx, services, all, no_cache),
        DockerCommand::Logs {
            services,
            all,
            tail,
            no_follow,
        } => run_logs(ctx, services, all, &tail, no_follow),
        DockerCommand::Status => run_status(ctx),
        DockerCommand::Shell { service } => run_shell(ctx, service),
        DockerCommand::Psql => run_psql(ctx),
    }
}

fn select_services(ctx: &AppContext, prompt: &str, allow_all: bool) -> Result<Vec<String>> {
    if ctx.quiet {
        // In quiet mode, default to core services
        return Ok(vec![
            "postgres".to_string(),
            "redis".to_string(),
            "nats".to_string(),
            "api".to_string(),
        ]);
    }

    let items: Vec<String> = SERVICES
        .iter()
        .map(|(name, desc)| format!("{} - {}", name, desc))
        .collect();

    let mut items_with_all = items.clone();
    if allow_all {
        items_with_all.insert(0, "All services".to_string());
    }

    let selections = MultiSelect::with_theme(&ctx.theme())
        .with_prompt(prompt)
        .items(&items_with_all)
        .interact()?;

    if selections.is_empty() {
        anyhow::bail!("No services selected");
    }

    // Check if "All services" was selected
    if allow_all && selections.contains(&0) {
        return Ok(SERVICES.iter().map(|(name, _)| name.to_string()).collect());
    }

    let offset = if allow_all { 1 } else { 0 };
    Ok(selections
        .into_iter()
        .filter(|&i| i >= offset)
        .map(|i| SERVICES[i - offset].0.to_string())
        .collect())
}

fn select_single_service(ctx: &AppContext, prompt: &str) -> Result<String> {
    if ctx.quiet {
        anyhow::bail!("Service selection requires interactive mode");
    }

    let items: Vec<String> = SERVICES
        .iter()
        .map(|(name, desc)| format!("{} - {}", name, desc))
        .collect();

    let selection = FuzzySelect::with_theme(&ctx.theme())
        .with_prompt(prompt)
        .items(&items)
        .default(0)
        .interact()?;

    Ok(SERVICES[selection].0.to_string())
}

fn docker_compose(ctx: &AppContext) -> Command {
    let mut cmd = Command::new("docker");
    cmd.args(["compose", "-f"]);
    cmd.arg(ctx.repo.join("docker-compose.yml"));
    cmd
}

fn run_up(
    ctx: &AppContext,
    services: Vec<String>,
    all: bool,
    full: bool,
    detach: bool,
) -> Result<()> {
    let services = if all {
        SERVICES
            .iter()
            .filter(|(name, _)| *name != "web-next" || full)
            .map(|(name, _)| name.to_string())
            .collect()
    } else if services.is_empty() {
        select_services(ctx, "Select services to start", true)?
    } else {
        services
    };

    ctx.print_header("Starting services");
    for svc in &services {
        println!("  • {}", style(svc).cyan());
    }
    println!();

    let mut cmd = docker_compose(ctx);
    if full {
        cmd.args(["--profile", "full"]);
    }
    cmd.arg("up");
    if detach {
        cmd.arg("-d");
    }
    cmd.args(&services);

    let status = cmd.status().context("Failed to run docker compose")?;

    if status.success() {
        ctx.print_success("Services started");
    } else {
        anyhow::bail!("Failed to start services");
    }

    Ok(())
}

fn run_down(ctx: &AppContext, services: Vec<String>, volumes: bool) -> Result<()> {
    ctx.print_header("Stopping services");

    if volumes {
        ctx.print_warning("WARNING: This will delete all data volumes!");
    }

    let mut cmd = docker_compose(ctx);
    cmd.arg("down");
    if volumes {
        cmd.arg("-v");
    }
    if !services.is_empty() {
        cmd.args(&services);
    }

    let status = cmd.status().context("Failed to run docker compose")?;

    if status.success() {
        ctx.print_success("Services stopped");
    } else {
        anyhow::bail!("Failed to stop services");
    }

    Ok(())
}

fn run_restart(ctx: &AppContext, services: Vec<String>, all: bool) -> Result<()> {
    let services = if all {
        SERVICES.iter().map(|(name, _)| name.to_string()).collect()
    } else if services.is_empty() {
        select_services(ctx, "Select services to restart", true)?
    } else {
        services
    };

    ctx.print_header("Restarting services");
    for svc in &services {
        println!("  • {}", style(svc).cyan());
    }
    println!();

    let mut cmd = docker_compose(ctx);
    cmd.arg("restart");
    cmd.args(&services);

    let status = cmd.status().context("Failed to run docker compose")?;

    if status.success() {
        ctx.print_success("Services restarted");
    } else {
        anyhow::bail!("Failed to restart services");
    }

    Ok(())
}

fn run_build(ctx: &AppContext, services: Vec<String>, all: bool, no_cache: bool) -> Result<()> {
    // Only services with Dockerfiles can be built
    let buildable = vec!["api", "web-next"];

    let services = if all {
        buildable.iter().map(|s| s.to_string()).collect()
    } else if services.is_empty() {
        let items: Vec<String> = buildable
            .iter()
            .map(|s| {
                let desc = SERVICES
                    .iter()
                    .find(|(n, _)| n == s)
                    .map(|(_, d)| *d)
                    .unwrap_or("");
                format!("{} - {}", s, desc)
            })
            .collect();

        if ctx.quiet {
            buildable.iter().map(|s| s.to_string()).collect()
        } else {
            let selections = MultiSelect::with_theme(&ctx.theme())
                .with_prompt("Select services to build")
                .items(&items)
                .interact()?;

            selections
                .into_iter()
                .map(|i| buildable[i].to_string())
                .collect()
        }
    } else {
        services
    };

    if services.is_empty() {
        ctx.print_info("No services selected to build");
        return Ok(());
    }

    ctx.print_header("Building services");
    for svc in &services {
        println!("  • {}", style(svc).cyan());
    }
    println!();

    let mut cmd = docker_compose(ctx);
    cmd.arg("build");
    if no_cache {
        cmd.arg("--no-cache");
    }
    cmd.args(&services);

    let status = cmd.status().context("Failed to run docker compose")?;

    if status.success() {
        ctx.print_success("Build complete");
    } else {
        anyhow::bail!("Build failed");
    }

    Ok(())
}

fn run_logs(
    ctx: &AppContext,
    services: Vec<String>,
    all: bool,
    tail: &str,
    no_follow: bool,
) -> Result<()> {
    let services = if all {
        vec![] // Empty means all services for logs
    } else if services.is_empty() {
        select_services(ctx, "Select services to follow logs", true)?
    } else {
        services
    };

    ctx.print_header("Following logs");

    let mut cmd = docker_compose(ctx);
    cmd.arg("logs");
    cmd.args(["--tail", tail]);
    if !no_follow {
        cmd.arg("-f");
    }
    if !services.is_empty() {
        cmd.args(&services);
    }

    let status = cmd.status().context("Failed to run docker compose")?;

    if !status.success() && !no_follow {
        // Ctrl+C exits with non-zero, which is fine for logs
    }

    Ok(())
}

fn run_status(ctx: &AppContext) -> Result<()> {
    ctx.print_header("Docker service status");
    println!();

    let mut cmd = docker_compose(ctx);
    cmd.args(["ps", "--format", "table {{.Name}}\t{{.Status}}\t{{.Ports}}"]);

    let status = cmd.status().context("Failed to run docker compose")?;

    if !status.success() {
        anyhow::bail!("Failed to get status");
    }

    Ok(())
}

fn run_shell(ctx: &AppContext, service: Option<String>) -> Result<()> {
    let service = match service {
        Some(s) => s,
        None => select_single_service(ctx, "Select service to open shell")?,
    };

    ctx.print_header(&format!("Opening shell in {}", service));

    let mut cmd = docker_compose(ctx);
    cmd.args(["exec", &service]);

    // Different shells for different containers
    let shell = match service.as_str() {
        "api" => "bash",
        _ => "sh",
    };
    cmd.arg(shell);

    let status = cmd.status().context("Failed to open shell")?;

    if !status.success() {
        anyhow::bail!("Shell exited with error");
    }

    Ok(())
}

fn run_psql(ctx: &AppContext) -> Result<()> {
    ctx.print_header("Connecting to PostgreSQL");

    let mut cmd = docker_compose(ctx);
    cmd.args([
        "exec",
        "postgres",
        "psql",
        "-U",
        "postgres",
        "-d",
        "mndigitalaid",
    ]);

    let status = cmd.status().context("Failed to connect to PostgreSQL")?;

    if !status.success() {
        anyhow::bail!("psql exited with error");
    }

    Ok(())
}
