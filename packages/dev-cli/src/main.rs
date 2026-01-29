//! Development CLI
//!
//! A comprehensive development environment management tool.

mod cmd;
mod cmd_builder;
mod compose;
mod config;
mod context;
mod history;
mod interactive;
mod menu;
mod services;
mod utils;

use anyhow::Result;
use clap::{Parser, Subcommand};
use console::style;
use dialoguer::Select;
use std::path::Path;
use std::process::ExitCode;

use cmd::{
    ai::ai_fix,
    ai_lint::ai_lint,
    benchmark::{
        benchmark_menu_with_config, quick_benchmark, run_api_benchmarks, run_load_test_with_config,
    },
    ci::{ci_cancel, ci_logs, ci_menu, ci_rerun, ci_runs, ci_status, ci_trigger, ci_watch},
    coverage::{run_coverage, CoverageFormat},
    db::{db_menu, db_migrate_with_config, db_psql, db_reset, db_seed},
    deploy::{
        deploy_menu_with_config, deploy_with_config, logs_cloudwatch_menu_with_config,
        preview as deploy_preview, refresh as deploy_refresh, show_outputs,
    },
    docker::{
        attach_container_with_logs, docker_compose_build, docker_compose_restart,
        docker_compose_up, docker_nuke_rebuild, docker_shell, logs_menu, stop_docker_containers,
    },
    ecs::{ecs_exec_menu, ecs_health},
    env::{pull_env_with_config, push_env_with_config, set_env_var_with_config, show_deployments},
    houston::start_houston,
    jobs::jobs_menu,
    migrate::data_migrate,
    mobile::{run_mobile_codegen, start_mobile},
    print_doctor,
    quality::{run_check, run_fmt, run_lint},
    release::{list_packages, list_releases, release_interactive, release_packages, rollback},
    status::{init_setup, show_status, sync_all},
    test::{run_tests, watch_tests},
    todos::{
        cleanup_issues, quick_add_with_sprint, reset_checklist, run_checklist_by_name,
        show_status as todos_status, todos_menu,
    },
    tunnel::start_http_tunnel,
    watch::{watch_api, watch_app, watch_menu},
};
use context::AppContext;
use history::record_action;

// =============================================================================
// CLI Arguments (clap)
// =============================================================================

#[derive(Parser)]
#[command(name = "dev")]
#[command(about = "Development CLI - manage your local dev environment")]
#[command(version)]
#[command(disable_help_subcommand = true)]
struct Cli {
    /// Run in quiet mode (non-interactive, use defaults)
    #[arg(short, long, global = true)]
    quiet: bool,

    /// DevOps mode (dangerous operations menu)
    #[arg(long, hide = true)]
    devops: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Clone)]
enum Commands {
    // =========================================================================
    // Quick Start/Stop
    // =========================================================================
    /// Start full development environment (docker + env sync)
    Start,

    /// Stop all services
    Stop,

    // =========================================================================
    // Docker (aliased as 'd')
    // =========================================================================
    /// Docker container management
    #[command(alias = "d")]
    Docker {
        #[command(subcommand)]
        action: DockerAction,
    },

    /// Follow container logs (auto-reconnects)
    Logs {
        /// Container/service name (optional, interactive if omitted)
        service: Option<String>,
    },

    /// Open shell in a running container
    Shell {
        /// Container/service name (optional, interactive if omitted)
        service: Option<String>,
    },

    // =========================================================================
    // Database (aliased as 'db')
    // =========================================================================
    /// Database operations
    #[command(alias = "db")]
    Database {
        #[command(subcommand)]
        action: Option<DbAction>,
    },

    /// Data migration management (cursor-based, resumable)
    ///
    /// For SQL schema migrations, use: ./dev.sh db migrate
    ///
    /// Examples:
    ///   ./dev.sh migrate list                           # List registered migrations
    ///   ./dev.sh migrate estimate <name> --env dev      # Count items to migrate
    ///   ./dev.sh migrate run <name> --env dev           # Dry-run
    ///   ./dev.sh migrate start <name> --env prod        # Execute migration
    ///   ./dev.sh migrate status <name> --env prod       # Check progress
    ///   ./dev.sh migrate pause <name> --env prod        # Pause migration
    ///   ./dev.sh migrate resume <name> --env prod       # Resume migration
    ///   ./dev.sh migrate verify <name> --env prod       # Verify completion
    ///   ./dev.sh migrate complete <name> --env prod     # Mark complete
    Migrate {
        #[command(subcommand)]
        action: MigrateAction,
    },

    // =========================================================================
    // Environment
    // =========================================================================
    /// Manage environment variables (ESC)
    Env {
        #[command(subcommand)]
        action: EnvAction,
    },

    // =========================================================================
    // Code Quality
    // =========================================================================
    /// Run code formatters (cargo fmt, prettier)
    Fmt {
        /// Auto-fix formatting issues
        #[arg(long)]
        fix: bool,
    },

    /// Run linters (clippy, eslint)
    Lint {
        /// Auto-fix lint issues where possible
        #[arg(long)]
        fix: bool,
        /// Use AI to fix lint issues that can't be auto-fixed
        #[arg(long)]
        ai_fix: bool,
    },

    /// Run pre-commit checks (fmt + lint + type check)
    Check,

    // =========================================================================
    // Build (aliased as 'b')
    // =========================================================================
    /// Build packages (uses [cmd.build] from dev.toml)
    ///
    /// Examples:
    ///   ./dev.sh build                      # Build all packages
    ///   ./dev.sh build api-server           # Build specific package
    ///   ./dev.sh build --release            # Build all with release variant
    ///   ./dev.sh build api-server --release # Build specific with release
    #[command(alias = "b")]
    Build {
        /// Package to build (all if omitted)
        package: Option<String>,
        /// Build in release mode
        #[arg(long)]
        release: bool,
        /// Build in watch mode
        #[arg(long)]
        watch: bool,
    },

    // =========================================================================
    // Testing (aliased as 't')
    // =========================================================================
    /// Run tests
    #[command(alias = "t")]
    Test {
        /// Package to test
        #[arg(short, long)]
        package: Option<String>,
        /// Test name filter
        #[arg(short, long)]
        filter: Option<String>,
        /// Watch for changes
        #[arg(short, long)]
        watch: bool,
    },

    /// Generate code coverage report
    Coverage {
        /// Package to measure coverage for
        #[arg(short, long)]
        package: Option<String>,
        /// Test name filter
        #[arg(short, long)]
        filter: Option<String>,
        /// Output format: html, lcov, or summary (default: html)
        #[arg(short, long, default_value = "html")]
        output: String,
        /// Open HTML report in browser after generation
        #[arg(long)]
        open: bool,
    },

    // =========================================================================
    // Mobile (aliased as 'm')
    // =========================================================================
    /// Mobile development (iOS/Android)
    #[command(alias = "m")]
    Mobile {
        #[command(subcommand)]
        action: Option<MobileAction>,
    },

    // =========================================================================
    // Utilities
    // =========================================================================
    /// Start HTTP tunnel
    Tunnel,

    /// Open a URL from config (e.g., dev open playground)
    Open {
        /// URL key from config (shows menu if omitted)
        key: Option<String>,
    },

    /// Check prerequisites and system health
    Doctor,

    /// Show development environment status
    Status,

    /// Sync everything (git pull + env + migrate)
    Sync,

    /// Watch mode for development (auto-rebuild)
    Watch {
        #[command(subcommand)]
        target: Option<WatchTarget>,
    },

    /// Start Houston monitoring dashboard (dev mode with HMR)
    Houston,

    /// Release packages (api, app, or all)
    Release {
        /// Packages to release (api, app, all) - interactive if omitted
        #[arg(value_name = "PACKAGE")]
        targets: Vec<String>,
        /// Bump type (patch, minor, major, hotfix)
        #[arg(short, long)]
        bump: Option<String>,
        /// Dry run (show what would happen)
        #[arg(long)]
        dry_run: bool,
        #[command(subcommand)]
        action: Option<ReleaseAction>,
    },

    /// Rollback to a previous version
    ///
    /// Examples:
    ///   ./dev.sh rollback           # Interactive: select env and version
    ///   ./dev.sh rollback v1.2.3    # Rollback to specific version (prompts for env)
    ///   ./dev.sh rollback v1.2.3 --env prod  # Rollback prod to v1.2.3
    Rollback {
        /// Version to rollback to (e.g., v1.2.3)
        version: Option<String>,
        /// Environment to rollback (dev or prod)
        #[arg(short, long)]
        env: Option<String>,
    },

    /// First-time developer setup
    Init,

    /// Interactive todo/checklist management
    ///
    /// Work through checklists stored in docs/checklist/*.toml
    /// or manage your own todo items.
    ///
    /// Examples:
    ///   ./dev.sh todo                     # Interactive menu
    ///   ./dev.sh todo "Fix the bug"       # Quick add (prompts for sprint)
    ///   ./dev.sh todo launch              # Work through launch checklist
    ///   ./dev.sh todo status              # Show progress
    Todo {
        /// Quick add: task text to add (prompts for sprint)
        text: Option<String>,
        #[command(subcommand)]
        action: Option<TodoAction>,
    },

    /// Run performance benchmarks
    Benchmark {
        #[command(subcommand)]
        action: Option<BenchmarkAction>,
    },

    /// Deploy to cloud environments (Pulumi)
    Deploy {
        #[command(subcommand)]
        action: Option<DeployAction>,
    },

    /// CI/CD operations (GitHub Actions)
    Ci {
        #[command(subcommand)]
        action: Option<CiAction>,
    },

    /// AI assistant - lint, fix, and run AI tasks
    ///
    /// Examples:
    ///   dev ai lint              # Run all linters
    ///   dev ai fix test          # Run tests with AI fix
    ///   dev ai security-audit    # Run preset task
    #[command(alias = "q")]
    Ai {
        /// List all available AI commands
        #[arg(long)]
        list: bool,

        #[command(subcommand)]
        action: Option<AiAction>,
    },

    /// Run package-defined commands
    ///
    /// Commands are defined in package dev.toml files:
    ///   [cmd]
    ///   test = "npx jest"
    ///
    ///   [cmd.build]
    ///   default = "npx tsc"
    ///   watch = "npx tsc --watch"
    ///   deps = ["common:build"]
    ///
    /// Examples:
    ///   dev cmd typecheck              # Run typecheck on all packages
    ///   dev cmd lint:fix               # Run lint with fix variant
    ///   dev cmd build:watch            # Run build in watch mode
    ///   dev cmd lint --parallel        # Run lint in parallel
    ///   dev cmd build -p app           # Run build on specific package
    ///   dev cmd --list                 # List all available commands
    Cmd {
        /// Command to run (e.g., build, build:watch, lint:fix)
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
}

// =============================================================================
// Subcommand Enums
// =============================================================================

#[derive(Subcommand, Clone)]
enum DockerAction {
    /// Start containers (optionally rebuild first)
    Up {
        /// Specific services to start
        services: Vec<String>,
        /// Rebuild images before starting
        #[arg(long)]
        build: bool,
    },
    /// Stop all containers
    #[command(alias = "down")]
    Stop,
    /// Restart containers
    Restart {
        /// Specific services to restart
        services: Vec<String>,
    },
    /// Build docker images
    Build {
        /// Specific services to build
        services: Vec<String>,
        /// Pull newer base images first
        #[arg(long)]
        pull: bool,
        /// Disable build cache
        #[arg(long)]
        no_cache: bool,
    },
    /// Nuke and rebuild (stop, remove images, rebuild from scratch)
    Nuke {
        /// Specific services to nuke and rebuild
        services: Vec<String>,
    },
}

#[derive(Subcommand, Clone)]
enum DbAction {
    /// Run SQL schema migrations (via sqlx)
    ///
    /// Examples:
    ///   ./dev.sh db migrate              # Local database
    ///   ./dev.sh db migrate --env dev    # Remote dev environment
    ///   ./dev.sh db migrate --env prod   # Remote prod (with confirmation)
    Migrate {
        /// Environment: dev, prod. Omit for local.
        #[arg(short, long)]
        env: Option<String>,
    },
    /// Reset database (drop + create + migrate)
    Reset,
    /// Seed database with test data
    Seed,
    /// Open psql shell
    Psql,
}

#[derive(Subcommand, Clone)]
enum MigrateAction {
    /// List all registered data migrations
    List,
    /// Estimate items needing migration
    Estimate {
        /// Migration name
        name: String,
        /// Environment (dev, prod)
        #[arg(short, long, default_value = "dev")]
        env: String,
    },
    /// Dry-run: validate migration without mutations
    Run {
        /// Migration name
        name: String,
        /// Environment (dev, prod)
        #[arg(short, long, default_value = "dev")]
        env: String,
        /// Batch size for processing
        #[arg(long, default_value = "100")]
        batch_size: i64,
    },
    /// Start the migration (commits changes)
    Start {
        /// Migration name
        name: String,
        /// Environment (dev, prod)
        #[arg(short, long)]
        env: String,
        /// Error budget (0.01 = 1%)
        #[arg(long, default_value = "0.01")]
        error_budget: f64,
        /// Batch size for processing
        #[arg(long, default_value = "100")]
        batch_size: i64,
    },
    /// Check migration progress
    Status {
        /// Migration name
        name: String,
        /// Environment (dev, prod)
        #[arg(short, long, default_value = "dev")]
        env: String,
    },
    /// Pause a running migration
    Pause {
        /// Migration name
        name: String,
        /// Environment (dev, prod)
        #[arg(short, long)]
        env: String,
    },
    /// Resume a paused migration
    Resume {
        /// Migration name
        name: String,
        /// Environment (dev, prod)
        #[arg(short, long)]
        env: String,
    },
    /// Verify migration integrity
    Verify {
        /// Migration name
        name: String,
        /// Environment (dev, prod)
        #[arg(short, long, default_value = "dev")]
        env: String,
    },
    /// Mark migration as complete
    Complete {
        /// Migration name
        name: String,
        /// Environment (dev, prod)
        #[arg(short, long)]
        env: String,
    },
}

#[derive(Subcommand, Clone)]
enum EnvAction {
    /// Pull environment variables from ESC
    Pull {
        /// Environment: dev, prod, or all
        #[arg(default_value = "all")]
        env: String,
    },
    /// Push environment variables to ESC
    Push {
        /// Environment: dev, prod, or all
        #[arg(default_value = "all")]
        env: String,
    },
    /// Set an environment variable
    Set {
        /// Environment (dev or prod)
        #[arg(short, long)]
        env: String,
        /// Variable name
        key: String,
        /// Variable value
        value: String,
        /// Mark as secret
        #[arg(short, long)]
        secret: bool,
    },
    /// Show deployment info
    Info {
        /// Environment: dev or prod
        env: Option<String>,
    },
}

#[derive(Subcommand, Clone)]
enum MobileAction {
    /// Start mobile development
    Start {
        /// Platform to run: ios, android, expo, or web
        #[arg(short, long)]
        platform: Option<String>,
    },
    /// Regenerate GraphQL types from the API schema
    Codegen,
}

#[derive(Subcommand, Clone)]
enum WatchTarget {
    /// Watch API for changes (auto-rebuild)
    Api,
    /// Watch App for changes (hot reload)
    App,
    /// Watch all (containers + API)
    All,
}

#[derive(Subcommand, Clone)]
enum ReleaseAction {
    /// List releasable packages
    Packages,
    /// List recent releases
    List {
        /// Number of releases to show per package
        #[arg(short, long, default_value = "5")]
        count: u32,
    },
}

#[derive(Subcommand, Clone)]
enum TodoAction {
    /// Add a new todo item
    Add {
        /// The todo item text
        text: String,
        /// Optional section/category
        #[arg(short, long)]
        section: Option<String>,
    },
    /// Remove a todo item
    Remove {
        /// The todo item ID or text to match
        item: String,
    },
    /// List all todos
    List {
        /// Show only incomplete items
        #[arg(long)]
        pending: bool,
    },
    /// Show progress status
    Status,
    /// Reset progress for a checklist
    Reset {
        /// Checklist name to reset
        name: String,
    },
    /// Work through a specific checklist
    Run {
        /// Checklist name (e.g., "launch")
        name: String,
    },
    /// Clean up backlog issues - review and tidy each one
    Cleanup {
        /// Include issues from a specific milestone (default: backlog only)
        #[arg(short, long)]
        milestone: Option<String>,
        /// Include all issues (not just backlog)
        #[arg(long)]
        all: bool,
    },
}

#[derive(Subcommand, Clone)]
enum BenchmarkAction {
    /// Quick benchmark (check + build + test times)
    Quick,
    /// Run cargo benchmarks
    Cargo {
        /// Filter benchmarks by name
        filter: Option<String>,
    },
    /// Load test an API endpoint
    Load {
        /// Endpoint URL
        #[arg(default_value = "http://localhost:8080/health")]
        endpoint: String,
        /// Number of requests
        #[arg(short, long, default_value = "1000")]
        requests: u32,
        /// Concurrency level
        #[arg(short, long, default_value = "10")]
        concurrency: u32,
    },
}

#[derive(Subcommand, Clone)]
enum DeployAction {
    /// Deploy to an environment
    Up {
        /// Environment (dev or prod)
        env: String,
        /// Specific stacks to deploy (default: all)
        #[arg(short, long)]
        stack: Vec<String>,
        /// Skip preview and deploy immediately
        #[arg(long)]
        yes: bool,
    },
    /// Preview deployment changes
    Preview {
        /// Environment (dev or prod)
        env: String,
        /// Specific stacks to preview
        #[arg(short, long)]
        stack: Vec<String>,
    },
    /// Show stack outputs
    Outputs {
        /// Environment (dev or prod)
        env: String,
        /// Specific stack
        #[arg(short, long)]
        stack: Option<String>,
    },
    /// Refresh state from cloud provider
    Refresh {
        /// Environment (dev or prod)
        env: String,
        /// Specific stacks to refresh
        #[arg(short, long)]
        stack: Vec<String>,
    },
}

#[derive(Subcommand, Clone)]
enum CiAction {
    /// Show CI status for current branch
    Status,
    /// List recent workflow runs
    Runs {
        /// Number of runs to show
        #[arg(short, long, default_value = "10")]
        limit: u32,
        /// Filter by workflow name
        #[arg(short, long)]
        workflow: Option<String>,
    },
    /// View logs for a workflow run
    Logs {
        /// Run ID (interactive if omitted)
        run_id: Option<String>,
    },
    /// Watch a running workflow
    Watch {
        /// Run ID (latest in-progress if omitted)
        run_id: Option<String>,
    },
    /// Trigger a workflow manually
    Trigger {
        /// Workflow name
        workflow: Option<String>,
        /// Branch to run on
        #[arg(short, long)]
        branch: Option<String>,
    },
    /// Re-run a failed workflow
    Rerun {
        /// Run ID (interactive if omitted)
        run_id: Option<String>,
        /// Only re-run failed jobs
        #[arg(long)]
        failed: bool,
    },
    /// Cancel a running workflow
    Cancel {
        /// Run ID (interactive if omitted)
        run_id: Option<String>,
    },
}

#[derive(Subcommand, Clone)]
enum AiAction {
    /// Run auto-fix steps and use AI for remaining issues
    ///
    /// Examples:
    ///   dev ai fix              # Run all: fmt, lint, test
    ///   dev ai fix test         # Just tests with AI fix
    ///   dev ai fix lint sec     # Security lint with AI fix
    ///   dev ai fix lint tc      # Test coverage lint with AI fix
    ///   dev ai fix docker       # Build Docker images with AI fix
    ///   dev ai fix "cargo build"  # Custom command
    Fix {
        /// What to fix: fmt, lint, lint sec, lint tc, test, docker, all (default), or custom command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    /// Lint code for issues (security, test coverage, migrations)
    ///
    /// Examples:
    ///   dev ai lint              # Run all linters
    ///   dev ai lint tc           # Test coverage only
    ///   dev ai lint sec          # Security only
    ///   dev ai lint migrations   # Migration safety only
    ///   dev ai lint tc --fix     # Lint and fix with Claude
    Lint {
        /// Category: tc (test-coverage), sec (security), migrations, all (default)
        category: Option<String>,
        /// Use Claude to fix violations
        #[arg(long)]
        fix: bool,
        /// Show all available lint rules
        #[arg(long)]
        rules: bool,
        /// Base branch to compare against (default: main)
        #[arg(long, default_value = "main")]
        base: String,
        /// Specific files to check (uses git diff if omitted)
        files: Vec<String>,
    },
    /// Run any other AI task (from docs/ai/tasks/)
    ///
    /// Examples:
    ///   dev ai security-audit
    ///   dev ai review-pr
    #[command(external_subcommand)]
    Task(Vec<String>),
}

// =============================================================================
// Environment Loading
// =============================================================================

/// Get the default environment from .dev/config.toml
fn get_default_env(repo_root: &Path) -> String {
    let config_path = repo_root.join(".dev/config.toml");
    if !config_path.exists() {
        return "dev".to_string();
    }

    // Minimal parsing - just extract environments.default
    #[derive(serde::Deserialize, Default)]
    struct EnvConfig {
        #[serde(default)]
        environments: EnvSection,
    }
    #[derive(serde::Deserialize)]
    struct EnvSection {
        #[serde(default = "default_env")]
        default: String,
    }
    impl Default for EnvSection {
        fn default() -> Self {
            Self {
                default: "dev".to_string(),
            }
        }
    }
    fn default_env() -> String {
        "dev".to_string()
    }

    std::fs::read_to_string(&config_path)
        .ok()
        .and_then(|content| toml::from_str::<EnvConfig>(&content).ok())
        .map(|c| c.environments.default)
        .unwrap_or_else(|| "dev".to_string())
}

/// Load .env files using dotenvy. Files loaded later override earlier ones.
fn load_env_files() {
    // Get repo root from REPO_ROOT env var (set by dev.sh)
    let repo_root = std::env::var("REPO_ROOT").unwrap_or_else(|_| ".".to_string());
    let root = Path::new(&repo_root);

    // Get default environment from config
    let default_env = get_default_env(root);

    // Load in order: .env, .env.{default}, .env.local (later files override earlier)
    let env_files = [
        ".env".to_string(),
        format!(".env.{}", default_env),
        ".env.local".to_string(),
    ];

    for env_file in env_files {
        let path = root.join(&env_file);
        if path.exists() {
            let _ = dotenvy::from_path(&path);
        }
    }
}

// =============================================================================
// Main Entry Points
// =============================================================================

fn main() -> ExitCode {
    // Load environment variables first
    load_env_files();

    if let Err(e) = real_main() {
        eprintln!("dev-cli error: {:#}", e);
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn real_main() -> Result<()> {
    let cli = Cli::parse();
    let ctx = AppContext::new(cli.quiet)?;

    // DevOps mode - separate menu for dangerous operations
    if cli.devops {
        return match cli.command {
            Some(cmd) => run_command(&ctx, cmd),
            None => run_devops_interactive(&ctx),
        };
    }

    match cli.command {
        Some(cmd) => run_command(&ctx, cmd),
        None => interactive::run_interactive(&ctx),
    }
}

fn run_command(ctx: &AppContext, cmd: Commands) -> Result<()> {
    // Use config from context (already loaded in AppContext::new)
    let config_ref = Some(&ctx.config);

    match cmd {
        // =====================================================================
        // Quick Start/Stop
        // =====================================================================
        Commands::Start => {
            record_action("Start development environment");
            ctx.print_header("Starting development environment");

            // Pull env vars
            if !ctx.quiet {
                println!("[start] Syncing environment variables...");
            }
            let _ = pull_env_with_config(ctx, config_ref, "dev", ".env.dev"); // Don't fail if esc not installed

            // Start docker
            if !ctx.quiet {
                println!("[start] Starting docker containers...");
            }
            docker_compose_up(ctx, &[], false)?;

            ctx.print_success("Development environment started!");
            Ok(())
        }

        Commands::Stop => {
            record_action("Stop development environment");
            stop_docker_containers(ctx)
        }

        // =====================================================================
        // Docker
        // =====================================================================
        Commands::Docker { action } => match action {
            DockerAction::Up { services, build } => {
                record_action("Start docker containers");
                docker_compose_up(ctx, &services, build)
            }
            DockerAction::Stop => {
                record_action("Stop docker containers");
                stop_docker_containers(ctx)
            }
            DockerAction::Restart { services } => {
                record_action("Restart docker containers");
                docker_compose_restart(ctx, &services)
            }
            DockerAction::Build {
                services,
                pull,
                no_cache,
            } => {
                record_action("Build docker images");
                docker_compose_build(ctx, &services, pull, no_cache)
            }
            DockerAction::Nuke { services } => {
                record_action("Nuke & rebuild docker images");
                docker_nuke_rebuild(ctx, &services)
            }
        },

        Commands::Logs { service } => {
            record_action("Follow container logs");
            if let Some(name) = service {
                attach_container_with_logs(ctx, &name)
            } else {
                logs_menu(ctx)
            }
        }

        Commands::Shell { service } => {
            record_action("Open container shell");
            docker_shell(ctx, service.as_deref())
        }

        // =====================================================================
        // Database
        // =====================================================================
        Commands::Database { action } => match action {
            Some(DbAction::Migrate { env }) => {
                record_action("SQL migrate");
                db_migrate_with_config(ctx, config_ref, env.as_deref())
            }
            Some(DbAction::Reset) => {
                record_action("Reset database");
                db_reset(ctx)
            }
            Some(DbAction::Seed) => {
                record_action("Seed database");
                db_seed(ctx)
            }
            Some(DbAction::Psql) => {
                record_action("Open psql shell");
                db_psql(ctx)
            }
            None => db_menu(ctx),
        },

        Commands::Migrate { action } => {
            record_action("Data migrate");
            data_migrate(ctx, config_ref, action)
        }

        // =====================================================================
        // Environment
        // =====================================================================
        Commands::Env { action } => {
            // Get available environments from config or use defaults
            let available_envs: Vec<String> = config_ref
                .map(|c| c.global.environments.available.clone())
                .unwrap_or_else(|| vec!["dev".to_string(), "prod".to_string()]);

            match action {
                EnvAction::Pull { env } => {
                    record_action(&format!("Pull env {}", env));
                    if env == "all" {
                        for e in &available_envs {
                            let env_file = format!(".env.{}", e);
                            pull_env_with_config(ctx, config_ref, e, &env_file)?;
                        }
                        Ok(())
                    } else if available_envs.iter().any(|e| e == &env) {
                        let env_file = format!(".env.{}", env);
                        pull_env_with_config(ctx, config_ref, &env, &env_file)
                    } else {
                        Err(anyhow::anyhow!(
                            "Invalid environment: {}. Available: {}, all",
                            env,
                            available_envs.join(", ")
                        ))
                    }
                }
                EnvAction::Push { env } => {
                    record_action(&format!("Push env {}", env));
                    if env == "all" {
                        for e in &available_envs {
                            let env_file = format!(".env.{}", e);
                            push_env_with_config(ctx, config_ref, e, &env_file)?;
                        }
                        Ok(())
                    } else if available_envs.iter().any(|e| e == &env) {
                        let env_file = format!(".env.{}", env);
                        push_env_with_config(ctx, config_ref, &env, &env_file)
                    } else {
                        Err(anyhow::anyhow!(
                            "Invalid environment: {}. Available: {}, all",
                            env,
                            available_envs.join(", ")
                        ))
                    }
                }
                EnvAction::Set {
                    env,
                    key,
                    value,
                    secret,
                } => {
                    record_action(&format!("Set env var {}", env));
                    set_env_var_with_config(ctx, config_ref, &env, &key, &value, secret)
                }
                EnvAction::Info { env } => {
                    record_action("Show deployment info");
                    show_deployments(ctx, config_ref, env.as_deref())
                }
            }
        }

        // =====================================================================
        // Code Quality
        // =====================================================================
        Commands::Fmt { fix } => {
            record_action("Run formatters");
            run_fmt(ctx, fix, false)?;
            Ok(())
        }

        Commands::Lint { fix, ai_fix } => {
            record_action("Run linters");
            run_lint(ctx, fix, ai_fix)?;
            Ok(())
        }

        Commands::Check => {
            record_action("Run pre-commit checks");
            run_check(ctx)
        }

        // =====================================================================
        // Build (routes to [cmd.build])
        // =====================================================================
        Commands::Build {
            package,
            release,
            watch,
        } => {
            let cfg = config_ref.ok_or_else(|| {
                anyhow::anyhow!("Config required for build. Create .dev/config.toml")
            })?;
            record_action("Build");
            let variant = if watch {
                Some("watch".to_string())
            } else if release {
                Some("release".to_string())
            } else {
                None
            };
            let packages = package.map(|p| vec![p]).unwrap_or_default();
            let opts = cmd::cmd::CmdOptions {
                parallel: false,
                variant,
                packages,
                capture: false,
            };
            let results = cmd::cmd::run_cmd(ctx, cfg, "build", &opts)?;
            cmd::cmd::print_results(ctx, &results);
            if results.iter().any(|r| !r.success) {
                return Err(anyhow::anyhow!("Build failed"));
            }
            Ok(())
        }

        // =====================================================================
        // Testing
        // =====================================================================
        Commands::Test {
            package,
            filter,
            watch,
        } => {
            record_action("Run tests");
            if watch {
                watch_tests(ctx)
            } else {
                run_tests(ctx, package.as_deref(), filter.as_deref(), false)?;
                Ok(())
            }
        }

        Commands::Coverage {
            package,
            filter,
            output,
            open,
        } => {
            record_action("Generate code coverage");
            let format = CoverageFormat::from_str(&output).unwrap_or(CoverageFormat::Html);
            run_coverage(ctx, package.as_deref(), filter.as_deref(), format, open)
        }

        // =====================================================================
        // Mobile
        // =====================================================================
        Commands::Mobile { action } => match action {
            Some(MobileAction::Start { platform }) => {
                record_action("Start mobile development");
                start_mobile(ctx, platform.as_deref())
            }
            Some(MobileAction::Codegen) => {
                record_action("Regenerate mobile GraphQL");
                run_mobile_codegen(ctx)
            }
            None => {
                record_action("Start mobile development");
                start_mobile(ctx, None)
            }
        },

        // =====================================================================
        // Utilities
        // =====================================================================
        Commands::Tunnel => {
            record_action("Start HTTP tunnel");
            start_http_tunnel(ctx)
        }

        Commands::Open { key } => {
            // Open command requires config
            let cfg = config_ref.ok_or_else(|| {
                anyhow::anyhow!("Config required for open command. Create .dev/config.toml with [urls.*] entries")
            })?;
            match key {
                Some(k) => {
                    record_action(&format!("Open {}", k));
                    cmd::open_url(ctx, cfg, &k)
                }
                None => {
                    record_action("Open URL menu");
                    cmd::open_url_menu(ctx, cfg)
                }
            }
        }

        Commands::Doctor => {
            record_action("Doctor");
            print_doctor(ctx);
            Ok(())
        }

        // =====================================================================
        // Status / Sync / Init
        // =====================================================================
        Commands::Status => {
            record_action("Status");
            show_status(ctx)
        }

        Commands::Sync => {
            record_action("Sync");
            sync_all(ctx)
        }

        Commands::Init => {
            record_action("Init");
            init_setup(ctx)
        }

        // =====================================================================
        // Todo
        // =====================================================================
        Commands::Todo { text, action } => {
            // If text is provided without a subcommand, do quick add with sprint selection
            if let Some(task_text) = text {
                if action.is_none() {
                    record_action("Quick add todo");
                    return quick_add_with_sprint(ctx, &task_text);
                }
            }

            match action {
                Some(TodoAction::Add { text, section }) => {
                    record_action("Add todo");
                    cmd::todos::add_todo(ctx, &text, section.as_deref())
                }
                Some(TodoAction::Remove { item }) => {
                    record_action("Remove todo");
                    cmd::todos::remove_todo(ctx, &item)
                }
                Some(TodoAction::List { pending }) => {
                    record_action("List todos");
                    cmd::todos::list_todos(ctx, pending)
                }
                Some(TodoAction::Status) => {
                    record_action("Todo status");
                    todos_status(ctx)
                }
                Some(TodoAction::Reset { name }) => {
                    record_action(&format!("Reset checklist {}", name));
                    reset_checklist(ctx, &name)
                }
                Some(TodoAction::Run { name }) => {
                    record_action(&format!("Run checklist {}", name));
                    run_checklist_by_name(ctx, &name)
                }
                Some(TodoAction::Cleanup { milestone, all }) => {
                    record_action("Cleanup backlog");
                    cleanup_issues(ctx, milestone.as_deref(), all)
                }
                None => {
                    record_action("Todo menu");
                    todos_menu(ctx)
                }
            }
        }

        // =====================================================================
        // Watch
        // =====================================================================
        Commands::Watch { target } => match target {
            Some(WatchTarget::Api) => {
                record_action("Watch API");
                watch_api(ctx, config_ref)
            }
            Some(WatchTarget::App) => {
                record_action("Watch App");
                watch_app(ctx, config_ref)
            }
            Some(WatchTarget::All) => {
                record_action("Watch All");
                cmd::watch::watch_all(ctx, config_ref)
            }
            None => watch_menu(ctx, config_ref),
        },

        // =====================================================================
        // Houston
        // =====================================================================
        Commands::Houston => {
            record_action("Houston");
            start_houston(ctx)
        }

        // =====================================================================
        // Release
        // =====================================================================
        Commands::Release {
            targets,
            bump,
            dry_run,
            action,
        } => {
            let config = config_ref.ok_or_else(|| {
                anyhow::anyhow!("Release commands require config. Create .dev/config.toml")
            })?;
            match action {
                Some(ReleaseAction::Packages) => {
                    record_action("List packages");
                    list_packages(ctx, config)
                }
                Some(ReleaseAction::List { count }) => {
                    record_action("List releases");
                    list_releases(ctx, config, count)
                }
                None => {
                    if targets.is_empty() {
                        record_action("Release");
                        release_interactive(ctx, config, dry_run)
                    } else {
                        record_action(&format!("Release {}", targets.join(", ")));
                        release_packages(ctx, config, &targets, bump.as_deref(), dry_run)
                    }
                }
            }
        }

        // =====================================================================
        // Rollback
        // =====================================================================
        Commands::Rollback { version, env } => {
            record_action("Rollback");
            rollback(ctx, version.as_deref(), env.as_deref())
        }

        // =====================================================================
        // Benchmarks
        // =====================================================================
        Commands::Benchmark { action } => match action {
            Some(BenchmarkAction::Quick) => {
                record_action("Quick Benchmark");
                quick_benchmark(ctx, config_ref)
            }
            Some(BenchmarkAction::Cargo { filter }) => {
                record_action("Cargo Benchmarks");
                run_api_benchmarks(ctx, config_ref, filter.as_deref())
            }
            Some(BenchmarkAction::Load {
                endpoint,
                requests,
                concurrency,
            }) => {
                record_action("Load Test");
                run_load_test_with_config(ctx, config_ref, Some(&endpoint), requests, concurrency)
            }
            None => benchmark_menu_with_config(ctx, config_ref),
        },

        // =====================================================================
        // Deploy
        // =====================================================================
        Commands::Deploy { action } => match action {
            Some(DeployAction::Up { env, stack, yes }) => {
                record_action(&format!("Deploy to {}", env));
                deploy_with_config(ctx, config_ref, &env, &stack, yes)
            }
            Some(DeployAction::Preview { env, stack }) => {
                record_action(&format!("Preview {}", env));
                deploy_preview(ctx, config_ref, &env, &stack)
            }
            Some(DeployAction::Outputs { env, stack }) => {
                record_action(&format!("Show {} outputs", env));
                show_outputs(ctx, config_ref, &env, stack.as_deref())
            }
            Some(DeployAction::Refresh { env, stack }) => {
                record_action(&format!("Refresh {}", env));
                deploy_refresh(ctx, config_ref, &env, &stack)
            }
            None => deploy_menu_with_config(ctx, config_ref),
        },

        // =====================================================================
        // CI/CD
        // =====================================================================
        Commands::Ci { action } => match action {
            Some(CiAction::Status) => {
                record_action("CI status");
                ci_status(ctx)
            }
            Some(CiAction::Runs { limit, workflow }) => {
                record_action("CI runs");
                ci_runs(ctx, limit, workflow.as_deref())
            }
            Some(CiAction::Logs { run_id }) => {
                record_action("CI logs");
                ci_logs(ctx, run_id.as_deref())
            }
            Some(CiAction::Watch { run_id }) => {
                record_action("CI watch");
                ci_watch(ctx, run_id.as_deref())
            }
            Some(CiAction::Trigger { workflow, branch }) => {
                record_action("CI trigger");
                ci_trigger(ctx, workflow.as_deref(), branch.as_deref())
            }
            Some(CiAction::Rerun { run_id, failed }) => {
                record_action("CI rerun");
                ci_rerun(ctx, run_id.as_deref(), failed)
            }
            Some(CiAction::Cancel { run_id }) => {
                record_action("CI cancel");
                ci_cancel(ctx, run_id.as_deref())
            }
            None => ci_menu(ctx),
        },

        // =====================================================================
        // AI Assistant
        // =====================================================================
        Commands::Ai { list, action } => {
            if list {
                return cmd::ai_assistant::list_all_commands(ctx);
            }
            match action {
                Some(AiAction::Fix { command }) => {
                    record_action("AI fix");
                    ai_fix(ctx, &command)
                }
                Some(AiAction::Lint {
                    category,
                    fix,
                    rules,
                    base,
                    files,
                }) => {
                    record_action("AI lint");
                    let files_opt = if files.is_empty() { None } else { Some(files) };
                    ai_lint(ctx, category.as_deref(), fix, rules, Some(&base), files_opt)
                }
                Some(AiAction::Task(args)) => {
                    if args.is_empty() {
                        cmd::ai_assistant::task_menu(ctx)
                    } else {
                        let task_name = &args[0];
                        record_action(&format!("AI: {}", task_name));
                        cmd::ai_assistant::run_task_by_id(ctx, task_name)
                    }
                }
                None => {
                    // Interactive menu
                    cmd::ai_assistant::ai_menu(ctx)
                }
            }
        }

        // =====================================================================
        // Package Commands
        // =====================================================================
        Commands::Cmd {
            command,
            parallel,
            package,
            list,
        } => {
            let cfg = config_ref.ok_or_else(|| {
                anyhow::anyhow!(
                    "Config required for cmd. Create .dev/config.toml and add [cmd] sections to package dev.toml files"
                )
            })?;

            if list {
                // List all available commands
                let commands = cmd::cmd::list_commands(cfg);
                if commands.is_empty() {
                    println!("No commands defined in any package.");
                    println!();
                    println!("Add commands to package dev.toml files:");
                    println!();
                    println!("  [cmd]");
                    println!("  build = \"npx tsc\"");
                    println!("  test = \"npx jest\"");
                    return Ok(());
                }

                println!("Available commands:");
                println!();
                let mut sorted: Vec<_> = commands.iter().collect();
                sorted.sort_by_key(|(k, _)| *k);
                for (cmd_name, packages) in sorted {
                    println!("  {} ({})", style(cmd_name).cyan(), packages.join(", "));
                }
                return Ok(());
            }

            match command {
                Some(cmd_str) => {
                    // Parse cmd:variant syntax (e.g., "build:watch" -> cmd="build", variant="watch")
                    let (cmd_name, variant) = if let Some(pos) = cmd_str.find(':') {
                        (&cmd_str[..pos], Some(cmd_str[pos + 1..].to_string()))
                    } else {
                        (cmd_str.as_str(), None)
                    };

                    record_action(&format!("cmd {}", cmd_str));
                    let opts = cmd::cmd::CmdOptions {
                        parallel,
                        variant,
                        packages: package.clone(),
                        capture: false,
                    };
                    let results = cmd::cmd::run_cmd(ctx, cfg, cmd_name, &opts)?;
                    cmd::cmd::print_results(ctx, &results);

                    // Return error if any failed
                    if results.iter().any(|r| !r.success) {
                        return Err(anyhow::anyhow!("Some commands failed"));
                    }
                    Ok(())
                }
                None => {
                    // Interactive: list commands and let user pick
                    let commands = cmd::cmd::list_commands(cfg);
                    if commands.is_empty() {
                        println!(
                            "No commands defined. Add [cmd] sections to package dev.toml files."
                        );
                        return Ok(());
                    }

                    let mut sorted: Vec<_> = commands.keys().collect();
                    sorted.sort();

                    let choice = Select::with_theme(&ctx.theme())
                        .with_prompt("Select command to run")
                        .items(&sorted)
                        .default(0)
                        .interact()?;

                    let cmd_name = sorted[choice];
                    record_action(&format!("cmd {}", cmd_name));

                    let opts = cmd::cmd::CmdOptions {
                        parallel,
                        variant: None,
                        packages: package.clone(),
                        capture: false,
                    };
                    let results = cmd::cmd::run_cmd(ctx, cfg, cmd_name, &opts)?;
                    cmd::cmd::print_results(ctx, &results);

                    if results.iter().any(|r| !r.success) {
                        return Err(anyhow::anyhow!("Some commands failed"));
                    }
                    Ok(())
                }
            }
        }
    }
}

// =============================================================================
// DevOps Interactive Mode
// =============================================================================

fn run_devops_interactive(ctx: &AppContext) -> Result<()> {
    use console::style;

    // Use config from context
    let config_ref = Some(&ctx.config);

    if !ctx.quiet {
        println!();
        println!(
            "{}",
            style("┌──────────────────────────────────────────────────────────────┐").red()
        );
        println!(
            "{}",
            style("│              DEVOPS - DANGER ZONE                            │")
                .red()
                .bold()
        );
        println!(
            "{}",
            style("│  These commands affect REMOTE/PRODUCTION systems.            │").red()
        );
        println!(
            "{}",
            style("└──────────────────────────────────────────────────────────────┘").red()
        );
        println!();
    }

    // Get available environments from config or use defaults
    let envs: Vec<String> = config_ref
        .map(|c| c.global.environments.available.clone())
        .unwrap_or_else(|| vec!["dev".to_string(), "prod".to_string()]);
    let envs_strs: Vec<&str> = envs.iter().map(|s| s.as_str()).collect();

    loop {
        let items = vec![
            "SSH into container (ECS Exec)",
            "Job queue debugging",
            "ECS health status",
            "Release",
            "CloudWatch logs",
            "CI/CD status",
            "Debug locally (act)",
            "Remote DB shell",
            "Exit",
        ];

        let choice = Select::with_theme(&ctx.theme())
            .with_prompt("DevOps Operations")
            .items(&items)
            .default(0)
            .interact()?;

        match choice {
            0 => {
                record_action("ECS Exec");
                ecs_exec_menu(ctx, config_ref)?
            }
            1 => {
                let env = select_env(ctx, &envs_strs)?;
                record_action(&format!("Job queue {}", env));
                jobs_menu(ctx, env)?
            }
            2 => {
                let env = select_env(ctx, &envs_strs)?;
                record_action(&format!("ECS health {}", env));
                ecs_health(ctx, env)?
            }
            3 => {
                record_action("Release");
                let cfg = config_ref.ok_or_else(|| {
                    anyhow::anyhow!("Release requires config. Create .dev/config.toml")
                })?;
                release_menu(ctx, cfg)?
            }
            4 => {
                record_action("CloudWatch logs");
                logs_cloudwatch_menu_with_config(ctx, config_ref)?
            }
            5 => {
                record_action("CI/CD");
                ci_menu(ctx)?
            }
            6 => {
                record_action("Debug locally");
                run_local_ci_menu(ctx)?
            }
            7 => {
                let env = select_env(ctx, &envs_strs)?;
                if confirm_devops_action(ctx, &format!("connect to {} database", env))? {
                    record_action(&format!("DB shell {}", env));
                    cmd::db::remote_db_shell(ctx, env)?
                }
            }
            _ => break,
        }
    }

    Ok(())
}

fn release_menu(ctx: &AppContext, config: &config::Config) -> Result<()> {
    loop {
        let items = vec![
            "Create release",
            "List packages",
            "List recent releases",
            "Back",
        ];

        let choice = Select::with_theme(&ctx.theme())
            .with_prompt("Release")
            .items(&items)
            .default(0)
            .interact()?;

        match choice {
            0 => {
                record_action("Create release");
                release_interactive(ctx, config, false)?
            }
            1 => {
                record_action("List packages");
                list_packages(ctx, config)?
            }
            2 => {
                record_action("List releases");
                list_releases(ctx, config, 5)?
            }
            _ => return Ok(()),
        }
    }
}

/// Run GitHub Actions locally via act
fn run_local_ci_menu(ctx: &AppContext) -> Result<()> {
    let config = config::Config::load(&ctx.repo)?;
    cmd::local::local_menu(ctx, &config)
}

fn select_env<'a>(ctx: &AppContext, envs: &'a [&str]) -> Result<&'a str> {
    let choice = Select::with_theme(&ctx.theme())
        .with_prompt("Environment")
        .items(envs)
        .default(0)
        .interact()?;
    Ok(envs[choice])
}

fn confirm_devops_action(ctx: &AppContext, action: &str) -> Result<bool> {
    // In CI mode, skip confirmation
    if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
        return Ok(true);
    }

    ctx.confirm(&format!("Are you sure you want to {}?", action), false)
}
