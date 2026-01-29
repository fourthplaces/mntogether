//! Database management commands
//!
//! Handles SQL schema migrations (via sqlx) and database utilities.
//! For data migrations, see `cmd/migrate.rs`.

use anyhow::{anyhow, Result};
use console::style;
use dialoguer::Select;
use std::path::PathBuf;

use crate::cmd_builder::CmdBuilder;
use crate::config::Config;
use crate::context::AppContext;
use crate::utils::{cmd_exists, ensure_cargo_tool};

/// Get migrations directory from config
fn get_migrations_dir(ctx: &AppContext) -> Result<PathBuf> {
    let config = Config::load(&ctx.repo)?;
    let db_packages = config.database_packages();
    if db_packages.is_empty() {
        return Err(anyhow!(
            "No packages with [database] capability found.\n\
             Add [database] section to a package's dev.toml."
        ));
    }

    let (pkg_name, _) = db_packages.first().unwrap();
    config
        .migrations_path(pkg_name)
        .ok_or_else(|| anyhow!("No migrations path found for {}", pkg_name))
}

/// Get seeds path from config (if exists)
fn get_seeds_path(ctx: &AppContext) -> Option<PathBuf> {
    let config = Config::load(&ctx.repo).ok()?;
    let db_packages = config.database_packages();
    let (pkg_name, _) = db_packages.first()?;
    config.seeds_path(pkg_name)
}

/// Load DATABASE_URL from .env.dev file
fn get_database_url(ctx: &AppContext) -> Result<String> {
    let env_file = ctx.repo.join(".env.dev");
    if !env_file.exists() {
        return Err(anyhow!(
            ".env.dev not found. Run `dev.sh env pull dev` first."
        ));
    }

    let content = std::fs::read_to_string(&env_file)?;
    content
        .lines()
        .find(|l| l.starts_with("DATABASE_URL="))
        .map(|l| l.trim_start_matches("DATABASE_URL="))
        .map(|v| v.trim_matches('"').trim_matches('\'').to_string())
        .ok_or_else(|| anyhow!("DATABASE_URL not found in .env.dev"))
}

/// Reset the local database (drop and recreate)
pub fn db_reset(ctx: &AppContext) -> Result<()> {
    ctx.print_header("Reset local database");

    if !ctx.confirm("This will DROP and recreate the database. Continue?", false)? {
        println!("Cancelled.");
        return Ok(());
    }

    let db_url = get_database_url(ctx)?;

    // Drop database
    ctx.print_header("Dropping database...");
    let code = CmdBuilder::new("sqlx")
        .args(["database", "drop", "-y"])
        .env("DATABASE_URL", &db_url)
        .cwd(&ctx.repo)
        .run()?;

    if code != 0 {
        ctx.print_warning("Database drop failed (may not exist)");
    }

    // Create database
    ctx.print_header("Creating database...");
    let code = CmdBuilder::new("sqlx")
        .args(["database", "create"])
        .env("DATABASE_URL", &db_url)
        .cwd(&ctx.repo)
        .run()?;

    if code != 0 {
        return Err(anyhow!("Failed to create database"));
    }

    // Run migrations
    ctx.print_header("Running migrations...");
    let migrations_dir = get_migrations_dir(ctx)?;
    let migrations_rel = migrations_dir
        .strip_prefix(&ctx.repo)
        .unwrap_or(&migrations_dir);

    let code = CmdBuilder::new("sqlx")
        .args([
            "migrate",
            "run",
            "--source",
            &migrations_rel.to_string_lossy(),
        ])
        .env("DATABASE_URL", &db_url)
        .cwd(&ctx.repo)
        .run()?;

    if code != 0 {
        return Err(anyhow!("Migrations failed"));
    }

    ctx.print_success("Database reset complete.");
    Ok(())
}

/// Run database seeds
pub fn db_seed(ctx: &AppContext) -> Result<()> {
    ctx.print_header("Seed database");

    let db_url = get_database_url(ctx)?;

    // Check if there's a seed script
    let seed_script = ctx.repo.join("scripts/seed.sh");
    let seed_sql = get_seeds_path(ctx);

    if seed_script.exists() {
        let code = CmdBuilder::new("bash")
            .arg(seed_script.to_string_lossy().to_string())
            .env("DATABASE_URL", &db_url)
            .cwd(&ctx.repo)
            .run()?;

        if code != 0 {
            return Err(anyhow!("Seed script failed"));
        }
    } else if let Some(seed_path) = seed_sql {
        if seed_path.exists() {
            let code = CmdBuilder::new("psql")
                .args(["-f", &seed_path.to_string_lossy()])
                .env("DATABASE_URL", &db_url)
                .cwd(&ctx.repo)
                .run()?;

            if code != 0 {
                return Err(anyhow!("Seed SQL failed"));
            }
        } else {
            ctx.print_warning(&format!(
                "Configured seeds path not found: {}",
                seed_path.display()
            ));
            return Ok(());
        }
    } else {
        ctx.print_warning(
            "No seed script found (scripts/seed.sh or configure [database].seeds in dev.toml)",
        );
        return Ok(());
    }

    ctx.print_success("Database seeded.");
    Ok(())
}

/// Open psql shell to the local database
pub fn db_psql(ctx: &AppContext) -> Result<()> {
    if !cmd_exists("psql") {
        return Err(anyhow!("psql not found. Install PostgreSQL client tools."));
    }

    ctx.print_header("Connecting to local database...");

    let db_url = get_database_url(ctx)?;

    let code = CmdBuilder::new("psql")
        .arg(&db_url)
        .cwd(&ctx.repo)
        .inherit_io()
        .run()?;

    if code != 0 && code != 130 {
        return Err(anyhow!("psql exited with code {code}"));
    }

    Ok(())
}

/// Connect to a remote database shell via ESC
pub fn remote_db_shell(ctx: &AppContext, env: &str) -> Result<()> {
    remote_db_shell_with_config(ctx, None, env)
}

/// Connect to a remote database shell via ESC with config
pub fn remote_db_shell_with_config(
    ctx: &AppContext,
    config: Option<&Config>,
    env: &str,
) -> Result<()> {
    if !cmd_exists("psql") {
        return Err(anyhow!("psql not found. Install PostgreSQL client tools."));
    }

    if !cmd_exists("esc") {
        return Err(anyhow!(
            "ESC CLI not found. Install from: https://www.pulumi.com/docs/esc-cli/"
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

    ctx.print_header(&format!("Connecting to {} database...", env));
    println!("Fetching credentials via ESC...");

    // Get ESC path from config or use default
    let esc_env = config
        .map(|c| c.esc_path(env))
        .unwrap_or_else(|| format!("shaya/service/{}", env));

    // Run psql via ESC which will inject DATABASE_URL
    let code = CmdBuilder::new("esc")
        .args(["run", &esc_env, "--", "psql"])
        .cwd(&ctx.repo)
        .inherit_io()
        .run()?;

    // psql exit code 130 is normal for Ctrl+C
    if code != 0 && code != 130 {
        return Err(anyhow!("psql exited with code {code}"));
    }

    Ok(())
}

/// Interactive menu for database operations
pub fn db_menu(ctx: &AppContext) -> Result<()> {
    let items = vec![
        "Run migrations (local)",
        "Open psql shell",
        "Reset database (drop + create + migrate)",
        "Seed database",
        "Back",
    ];

    let choice = if ctx.quiet {
        0
    } else {
        Select::with_theme(&ctx.theme())
            .with_prompt("Database operations")
            .items(&items)
            .default(0)
            .interact()?
    };

    match choice {
        0 => db_migrate(ctx, None),
        1 => db_psql(ctx),
        2 => db_reset(ctx),
        3 => db_seed(ctx),
        _ => Ok(()),
    }
}

// =============================================================================
// SQL Schema Migrations (via sqlx)
// =============================================================================

fn ensure_sqlx_cli(ctx: &AppContext) -> Result<()> {
    ensure_cargo_tool(
        ctx,
        "sqlx",
        "It is required to run migrations.",
        &["--no-default-features", "--features", "postgres,rustls"],
    )
}

/// Run SQL schema migrations
///
/// - `env`: None for local, Some("dev"|"prod") for remote
pub fn db_migrate(ctx: &AppContext, env: Option<&str>) -> Result<()> {
    db_migrate_with_config(ctx, None, env)
}

/// Run SQL schema migrations with config
pub fn db_migrate_with_config(
    ctx: &AppContext,
    config: Option<&Config>,
    env: Option<&str>,
) -> Result<()> {
    match env {
        None => migrate_local_db(ctx, config),
        Some(e) => migrate_remote_db(ctx, config, e),
    }
}

/// Run migrations against local database
fn migrate_local_db(ctx: &AppContext, config: Option<&Config>) -> Result<()> {
    use crate::compose::run_docker_compose;
    use crate::utils::ensure_docker;
    use std::net::{SocketAddr, TcpStream};
    use std::time::{Duration, Instant};

    // Parse docker-compose to get postgres config
    let pg_config = read_postgres_config_from_compose(&ctx.repo)?;
    let url = database_url(&pg_config);

    ensure_docker()?;

    ctx.print_header("Starting postgres container (docker compose up -d postgres)...");
    let code = run_docker_compose(&ctx.repo, &["up", "-d", "postgres"])?;
    if code != 0 {
        return Err(anyhow!("docker compose exited with code {code}"));
    }

    ctx.print_header(&format!(
        "Waiting for Postgres on localhost:{} ...",
        pg_config.host_port
    ));

    // Wait for postgres
    let timeout = Duration::from_secs(60);
    let start = Instant::now();
    let addr: SocketAddr = format!("127.0.0.1:{}", pg_config.host_port).parse()?;
    loop {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok() {
            break;
        }
        if start.elapsed() > timeout {
            return Err(anyhow!(
                "Timed out waiting for postgres on port {}",
                pg_config.host_port
            ));
        }
        std::thread::sleep(Duration::from_millis(300));
    }

    ensure_sqlx_cli(ctx)?;

    let migrations_dir = get_migrations_dir_with_config(ctx, config)?;
    ctx.print_header(&format!(
        "Running sqlx migrations from {}...",
        migrations_dir.display()
    ));

    let code = CmdBuilder::new("sqlx")
        .args(["migrate", "run"])
        .cwd(&migrations_dir)
        .env("DATABASE_URL", &url)
        .run()?;

    if code != 0 {
        return Err(anyhow!("sqlx migrate run exited with code {code}"));
    }

    ctx.print_success("Migrations complete.");
    Ok(())
}

/// Run migrations against a remote environment
fn migrate_remote_db(ctx: &AppContext, config: Option<&Config>, env: &str) -> Result<()> {
    // Validate environment
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

    ctx.print_header(&format!("Running SQL migrations for {} environment", env));

    // Safety check for production
    if env == "prod" && !ctx.quiet {
        println!();
        println!(
            "{}",
            style("⚠️  WARNING: You are about to run migrations on PRODUCTION database")
                .red()
                .bold()
        );
        println!(
            "{}",
            style("This operation can cause data loss if migrations are destructive!").red()
        );
        println!();

        if !ctx.confirm("Are you ABSOLUTELY sure you want to continue?", false)? {
            println!("Migration cancelled.");
            return Ok(());
        }

        // Double confirmation for prod
        println!();
        let confirm_text: String = dialoguer::Input::with_theme(&ctx.theme())
            .with_prompt("Type 'migrate prod' to confirm")
            .interact_text()?;

        if confirm_text.trim() != "migrate prod" {
            println!("Confirmation failed. Migration cancelled.");
            return Ok(());
        }
        println!();
    }

    // Check if DATABASE_URL is set (should be injected by ESC in CI)
    let database_url = std::env::var("DATABASE_URL").ok();

    if database_url.is_none() {
        // In CI/quiet mode, DATABASE_URL should be set
        if ctx.quiet {
            return Err(anyhow!(
                "DATABASE_URL not set.\n\
                 In CI, ensure pulumi/esc-action is configured and the ESC environment exports DATABASE_URL."
            ));
        }

        // Try to run via ESC locally
        if !cmd_exists("esc") {
            return Err(anyhow!(
                "DATABASE_URL not set and ESC CLI not found.\n\
                 Either set DATABASE_URL or install ESC CLI: curl -fsSL https://get.pulumi.com/esc/install.sh | sh"
            ));
        }

        ctx.print_header("Running migrations via ESC...");
        let esc_env = config
            .map(|c| c.esc_path(env))
            .unwrap_or_else(|| format!("shaya/service/{}", env));
        let migrations_dir = get_migrations_dir_with_config(ctx, config)?;

        let code = CmdBuilder::new("esc")
            .args(["run", &esc_env, "--", "sqlx", "migrate", "run"])
            .cwd(&migrations_dir)
            .run()?;

        if code != 0 {
            return Err(anyhow!("Migration failed with exit code {}", code));
        }
    } else {
        // DATABASE_URL is set, run sqlx directly
        ensure_sqlx_cli(ctx)?;

        let migrations_dir = get_migrations_dir_with_config(ctx, config)?;
        ctx.print_header(&format!(
            "Running sqlx migrations from {}...",
            migrations_dir.display()
        ));

        let code = CmdBuilder::new("sqlx")
            .args(["migrate", "run"])
            .cwd(&migrations_dir)
            .run()?;

        if code != 0 {
            return Err(anyhow!("sqlx migrate run exited with code {}", code));
        }
    }

    ctx.print_success(&format!("Migrations complete for {} environment.", env));
    Ok(())
}

/// Get migrations directory with optional config
fn get_migrations_dir_with_config(ctx: &AppContext, config: Option<&Config>) -> Result<PathBuf> {
    match config {
        Some(cfg) => find_migrations_package(cfg),
        None => {
            let cfg = Config::load(&ctx.repo)?;
            find_migrations_package(&cfg)
        }
    }
}

fn find_migrations_package(config: &Config) -> Result<PathBuf> {
    let db_packages = config.database_packages();
    if db_packages.is_empty() {
        return Err(anyhow!(
            "No packages with [database] capability found.\n\
             Add [database] section to a package's dev.toml with migrations path."
        ));
    }

    let (pkg_name, _db_config) = db_packages.first().unwrap();
    let pkg = config
        .get_package(pkg_name)
        .ok_or_else(|| anyhow!("Package {} not found in config", pkg_name))?;

    Ok(pkg.path.clone())
}

// =============================================================================
// Docker Compose Postgres Config Parsing
// =============================================================================

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct ComposeFile {
    services: BTreeMap<String, ComposeService>,
}

#[derive(Debug, Deserialize)]
struct ComposeService {
    #[serde(default)]
    environment: Option<serde_yaml::Value>,
    #[serde(default)]
    ports: Option<Vec<serde_yaml::Value>>,
}

#[derive(Debug)]
struct PostgresConfig {
    user: String,
    password: String,
    db: String,
    host_port: u16,
}

fn expand_shell_var(s: &str) -> String {
    if let Some(rest) = s.strip_prefix("${") {
        if let Some(end_idx) = rest.find('}') {
            let inner = &rest[..end_idx];
            if let Some((var_name, default)) = inner.split_once(":-") {
                return std::env::var(var_name).unwrap_or_else(|_| default.to_string());
            }
            if let Ok(val) = std::env::var(inner) {
                return val;
            }
        }
    }
    s.to_string()
}

fn parse_env_map(env: &serde_yaml::Value) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    match env {
        serde_yaml::Value::Sequence(seq) => {
            for v in seq {
                if let serde_yaml::Value::String(s) = v {
                    if let Some((k, val)) = s.split_once('=') {
                        out.insert(k.trim().to_string(), expand_shell_var(val.trim()));
                    }
                }
            }
        }
        serde_yaml::Value::Mapping(map) => {
            for (k, v) in map {
                let k = match k {
                    serde_yaml::Value::String(s) => s.clone(),
                    _ => continue,
                };
                let v = match v {
                    serde_yaml::Value::String(s) => expand_shell_var(s),
                    serde_yaml::Value::Number(n) => n.to_string(),
                    serde_yaml::Value::Bool(b) => b.to_string(),
                    _ => continue,
                };
                out.insert(k, v);
            }
        }
        _ => {}
    }
    out
}

fn parse_host_port(ports: &[serde_yaml::Value], container_port: u16) -> Option<u16> {
    for p in ports {
        let s = match p {
            serde_yaml::Value::String(s) => s.clone(),
            serde_yaml::Value::Number(n) => n.to_string(),
            _ => continue,
        };
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 2 {
            if parts[1].trim().parse::<u16>().ok() == Some(container_port) {
                if let Ok(hp) = parts[0].trim().parse::<u16>() {
                    return Some(hp);
                }
            }
        } else if parts.len() == 3 && parts[2].trim().parse::<u16>().ok() == Some(container_port) {
            if let Ok(hp) = parts[1].trim().parse::<u16>() {
                return Some(hp);
            }
        }
    }
    None
}

fn read_postgres_config_from_compose(repo: &Path) -> Result<PostgresConfig> {
    let compose_path = repo.join("docker-compose.yml");
    let raw = std::fs::read_to_string(&compose_path)
        .map_err(|e| anyhow!("read {}: {}", compose_path.display(), e))?;
    let doc: ComposeFile =
        serde_yaml::from_str(&raw).map_err(|e| anyhow!("parse docker-compose.yml: {}", e))?;

    let svc = doc
        .services
        .get("postgres")
        .ok_or_else(|| anyhow!("docker-compose.yml is missing services.postgres"))?;

    let env_map = svc
        .environment
        .as_ref()
        .map(parse_env_map)
        .unwrap_or_default();

    let user = env_map
        .get("POSTGRES_USER")
        .cloned()
        .unwrap_or_else(|| "user".to_string());
    let password = env_map
        .get("POSTGRES_PASSWORD")
        .cloned()
        .unwrap_or_else(|| "password".to_string());
    let db = env_map
        .get("POSTGRES_DB")
        .cloned()
        .unwrap_or_else(|| "shaya".to_string());

    let host_port = svc
        .ports
        .as_ref()
        .and_then(|p| parse_host_port(p, 5432))
        .unwrap_or(5432);

    Ok(PostgresConfig {
        user,
        password,
        db,
        host_port,
    })
}

fn database_url(cfg: &PostgresConfig) -> String {
    let user = cfg.user.replace('@', "%40").replace(':', "%3A");
    let pass = cfg.password.replace('@', "%40").replace(':', "%3A");
    format!(
        "postgresql://{}:{}@localhost:{}/{}",
        user, pass, cfg.host_port, cfg.db
    )
}
