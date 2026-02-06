//! Project-specific development CLI
//!
//! Customize this file to add your own commands and workflows.

use anyhow::Result;
use clap::{Parser, Subcommand};
use devkit_core::AppContext;
use std::process::ExitCode;

mod cmd;

#[derive(Parser)]
#[command(name = "dev")]
#[command(about = "Development environment CLI")]
#[command(version)]
struct Cli {
    /// Run in quiet mode (non-interactive)
    #[arg(short, long, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start development environment (Docker + migrations)
    Up {
        /// Start all services
        #[arg(short, long)]
        all: bool,

        /// Include optional services (web-next)
        #[arg(long)]
        full: bool,
    },

    /// Stop all services
    Down {
        /// Remove volumes (WARNING: deletes data)
        #[arg(short, long)]
        volumes: bool,
    },

    /// Show environment status
    Status,

    /// Check system prerequisites
    Doctor,

    /// Run package-defined commands
    ///
    /// Commands are defined in package dev.toml files.
    /// Example: dev cmd build, dev cmd test
    Cmd {
        /// Command name (e.g., build, test, lint)
        command: Option<String>,

        /// Run in parallel where possible
        #[arg(long)]
        parallel: bool,

        /// Only run for specific packages
        #[arg(short, long)]
        package: Vec<String>,

        /// List all available commands
        #[arg(long)]
        list: bool,
    },

    /// Docker service management
    #[command(subcommand)]
    Docker(cmd::docker::DockerCommand),

    /// Database schema operations (sqlx migrations)
    #[command(subcommand)]
    Db(cmd::db::DbCommand),

    /// Data migrations for surgical database transformations
    #[command(subcommand)]
    Migrate(cmd::migrate::MigrateCommand),

    /// Run utility scripts (seed, embeddings, etc.)
    #[command(subcommand)]
    Scripts(cmd::scripts::ScriptsCommand),
}

fn main() -> ExitCode {
    // Load environment variables
    let _ = dotenvy::dotenv();

    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let ctx = AppContext::new(cli.quiet)?;

    match cli.command {
        Some(Commands::Up { all, full }) => cmd_up(&ctx, all, full),
        Some(Commands::Down { volumes }) => cmd_down(&ctx, volumes),
        Some(Commands::Status) => cmd_status(&ctx),
        Some(Commands::Doctor) => cmd_doctor(&ctx),
        Some(Commands::Cmd {
            command,
            parallel,
            package,
            list,
        }) => cmd_run(&ctx, command, parallel, package, list),
        Some(Commands::Docker(cmd)) => cmd::docker::run(&ctx, cmd),
        Some(Commands::Db(cmd)) => cmd::db::run(&ctx, cmd),
        Some(Commands::Migrate(cmd)) => cmd::migrate::run(&ctx, cmd),
        Some(Commands::Scripts(cmd)) => cmd::scripts::run(&ctx, cmd),
        None => interactive_menu(&ctx),
    }
}

fn cmd_up(ctx: &AppContext, all: bool, full: bool) -> Result<()> {
    ctx.print_header("Starting development environment");

    // Start only the database service first (needed for migrations)
    ctx.print_info("Starting database service...");
    cmd::docker::run(
        ctx,
        cmd::docker::DockerCommand::Up {
            services: vec!["postgres".to_string()],
            all: false,
            full: false,
            detach: true,
        },
    )?;

    // Wait for postgres to be healthy
    ctx.print_info("Waiting for database to be ready...");
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Run database migrations BEFORE starting the API
    ctx.print_info("Running database migrations...");
    if let Err(e) = cmd::db::run(ctx, cmd::db::DbCommand::Migrate) {
        ctx.print_warning(&format!("Migration warning: {}", e));
        ctx.print_info("You may need to install sqlx-cli: cargo install sqlx-cli");
    }

    // Now start remaining Docker services
    ctx.print_info("Starting remaining services...");
    cmd::docker::run(
        ctx,
        cmd::docker::DockerCommand::Up {
            services: vec![],
            all,
            full,
            detach: true,
        },
    )?;

    ctx.print_success("Development environment is ready!");
    println!();
    ctx.print_info("Services:");
    ctx.print_info("  • API:     http://localhost:8080");
    ctx.print_info("  • GraphQL: http://localhost:8080/graphql");
    if full {
        ctx.print_info("  • Web:     http://localhost:3000");
    }

    Ok(())
}

fn cmd_down(ctx: &AppContext, volumes: bool) -> Result<()> {
    ctx.print_header("Stopping development environment");

    cmd::docker::run(
        ctx,
        cmd::docker::DockerCommand::Down {
            services: vec![],
            volumes,
        },
    )
}

fn cmd_status(ctx: &AppContext) -> Result<()> {
    ctx.print_header("Development Environment Status");
    println!();
    println!("Repository: {}", ctx.repo.display());
    println!("Project: {}", ctx.config.global.project.name);
    println!();
    ctx.print_success("✓ Configuration loaded");
    Ok(())
}

fn cmd_doctor(ctx: &AppContext) -> Result<()> {
    ctx.print_header("System Health Check");
    println!();

    let tools = vec![
        ("git", devkit_core::utils::cmd_exists("git")),
        ("cargo", devkit_core::utils::cmd_exists("cargo")),
        ("docker", devkit_core::utils::docker_available()),
    ];

    for (tool, available) in tools {
        if available {
            ctx.print_success(&format!("✓ {}", tool));
        } else {
            ctx.print_warning(&format!("✗ {} (not found)", tool));
        }
    }

    println!();
    ctx.print_success("Health check complete");
    Ok(())
}

fn cmd_run(
    ctx: &AppContext,
    command: Option<String>,
    parallel: bool,
    packages: Vec<String>,
    list: bool,
) -> Result<()> {
    use devkit_tasks::{list_commands, print_results, run_cmd, CmdOptions};

    if list {
        let commands = list_commands(&ctx.config);
        if commands.is_empty() {
            println!("No commands defined.");
            println!();
            println!("Add commands to package dev.toml files:");
            println!();
            println!("  [cmd]");
            println!("  build = \"cargo build\"");
            println!("  test = \"cargo test\"");
            return Ok(());
        }

        println!("Available commands:");
        println!();
        for (cmd, pkgs) in commands {
            println!("  {} ({})", cmd, pkgs.join(", "));
        }
        return Ok(());
    }

    let cmd_name = match command {
        Some(c) => c,
        None => {
            ctx.print_warning("No command specified. Use --list to see available commands.");
            return Ok(());
        }
    };

    let opts = CmdOptions {
        parallel,
        variant: None,
        packages,
        capture: false,
    };

    let results = run_cmd(ctx, &cmd_name, &opts)?;
    print_results(ctx, &results);

    if results.iter().any(|r| !r.success) {
        return Err(anyhow::anyhow!("Some commands failed"));
    }

    Ok(())
}

fn interactive_menu(ctx: &AppContext) -> Result<()> {
    use dialoguer::FuzzySelect;

    let items = vec![
        "🚀 Start environment (up)",
        "🛑 Stop environment (down)",
        "🐳 Docker services →",
        "🗄️  Database →",
        "📦 Data migrations →",
        "🔧 Scripts →",
        "📊 Status",
        "🩺 Doctor",
        "❌ Exit",
    ];

    loop {
        println!();
        let choice = FuzzySelect::with_theme(&ctx.theme())
            .with_prompt("What would you like to do?")
            .items(&items)
            .default(0)
            .interact()?;

        match choice {
            0 => cmd_up(ctx, false, false)?,
            1 => cmd_down(ctx, false)?,
            2 => docker_submenu(ctx)?,
            3 => db_submenu(ctx)?,
            4 => migrate_submenu(ctx)?,
            5 => scripts_submenu(ctx)?,
            6 => cmd_status(ctx)?,
            7 => cmd_doctor(ctx)?,
            _ => break,
        }
    }

    Ok(())
}

fn docker_submenu(ctx: &AppContext) -> Result<()> {
    use dialoguer::Select;

    let items = vec![
        "Start services",
        "Stop services",
        "Restart services",
        "Rebuild images",
        "Follow logs",
        "Status",
        "Shell into container",
        "PostgreSQL shell",
        "← Back",
    ];

    let choice = Select::with_theme(&ctx.theme())
        .with_prompt("Docker")
        .items(&items)
        .default(0)
        .interact()?;

    match choice {
        0 => cmd::docker::run(
            ctx,
            cmd::docker::DockerCommand::Up {
                services: vec![],
                all: false,
                full: false,
                detach: true,
            },
        ),
        1 => cmd::docker::run(
            ctx,
            cmd::docker::DockerCommand::Down {
                services: vec![],
                volumes: false,
            },
        ),
        2 => cmd::docker::run(
            ctx,
            cmd::docker::DockerCommand::Restart {
                services: vec![],
                all: false,
            },
        ),
        3 => cmd::docker::run(
            ctx,
            cmd::docker::DockerCommand::Build {
                services: vec![],
                all: false,
                no_cache: false,
            },
        ),
        4 => cmd::docker::run(
            ctx,
            cmd::docker::DockerCommand::Logs {
                services: vec![],
                all: false,
                tail: "100".to_string(),
                no_follow: false,
            },
        ),
        5 => cmd::docker::run(ctx, cmd::docker::DockerCommand::Status),
        6 => cmd::docker::run(ctx, cmd::docker::DockerCommand::Shell { service: None }),
        7 => cmd::docker::run(ctx, cmd::docker::DockerCommand::Psql),
        _ => Ok(()),
    }
}

fn db_submenu(ctx: &AppContext) -> Result<()> {
    use dialoguer::Select;

    let items = vec![
        "Run migrations",
        "Reset database (drop + migrate)",
        "Migration status",
        "PostgreSQL shell",
        "← Back",
    ];

    let choice = Select::with_theme(&ctx.theme())
        .with_prompt("Database")
        .items(&items)
        .default(0)
        .interact()?;

    match choice {
        0 => cmd::db::run(ctx, cmd::db::DbCommand::Migrate),
        1 => cmd::db::run(ctx, cmd::db::DbCommand::Reset),
        2 => cmd::db::run(ctx, cmd::db::DbCommand::Status),
        3 => cmd::db::run(ctx, cmd::db::DbCommand::Psql),
        _ => Ok(()),
    }
}

fn migrate_submenu(ctx: &AppContext) -> Result<()> {
    use dialoguer::Select;

    let items = vec![
        "Status (all migrations)",
        "Run all pending (dry-run)",
        "Start all pending",
        "Run one (dry-run)",
        "Start one",
        "Check one status",
        "Verify one",
        "← Back",
    ];

    let choice = Select::with_theme(&ctx.theme())
        .with_prompt("Data Migrations")
        .items(&items)
        .default(0)
        .interact()?;

    match choice {
        0 => cmd::migrate::run(ctx, cmd::migrate::MigrateCommand::StatusAll),
        1 => cmd::migrate::run(ctx, cmd::migrate::MigrateCommand::RunAll),
        2 => cmd::migrate::run(ctx, cmd::migrate::MigrateCommand::StartAll),
        3 => cmd::migrate::run(ctx, cmd::migrate::MigrateCommand::Run { name: None }),
        4 => cmd::migrate::run(ctx, cmd::migrate::MigrateCommand::Start { name: None }),
        5 => cmd::migrate::run(ctx, cmd::migrate::MigrateCommand::Status { name: None }),
        6 => cmd::migrate::run(ctx, cmd::migrate::MigrateCommand::Verify { name: None }),
        _ => Ok(()),
    }
}

fn scripts_submenu(ctx: &AppContext) -> Result<()> {
    use dialoguer::Select;

    let items = vec![
        "List scripts",
        "Seed organizations",
        "Generate embeddings",
        "← Back",
    ];

    let choice = Select::with_theme(&ctx.theme())
        .with_prompt("Scripts")
        .items(&items)
        .default(0)
        .interact()?;

    match choice {
        0 => cmd::scripts::run(ctx, cmd::scripts::ScriptsCommand::List),
        1 => cmd::scripts::run(ctx, cmd::scripts::ScriptsCommand::Seed),
        2 => cmd::scripts::run(ctx, cmd::scripts::ScriptsCommand::Embeddings),
        _ => Ok(()),
    }
}
