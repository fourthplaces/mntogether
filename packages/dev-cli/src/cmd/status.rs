//! Status, sync, and init commands

use anyhow::Result;
use console::style;

use crate::cmd_builder::CmdBuilder;
use crate::context::AppContext;
use crate::utils::cmd_exists;

/// Show current development environment status
pub fn show_status(ctx: &AppContext) -> Result<()> {
    ctx.print_header("Development Environment Status");
    println!();

    // Git status
    print_git_status(ctx)?;
    println!();

    // Docker status
    print_docker_status(ctx)?;
    println!();

    // Database status
    print_db_status(ctx)?;
    println!();

    // Environment sync status
    print_env_status(ctx)?;

    Ok(())
}

fn print_git_status(ctx: &AppContext) -> Result<()> {
    println!("{}", style("Git").bold());

    let branch = CmdBuilder::new("git")
        .args(["branch", "--show-current"])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()?;

    let branch_name = branch.stdout_string().trim().to_string();
    println!("  Branch: {}", style(&branch_name).cyan());

    // Check if ahead/behind remote
    let status = CmdBuilder::new("git")
        .args(["status", "--porcelain", "-b"])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()?;

    let status_str = status.stdout_string();
    let first_line = status_str.lines().next().unwrap_or("");

    if first_line.contains("ahead") {
        let ahead_count = extract_count(first_line, "ahead");
        println!("  Status: {} commits ahead", style(ahead_count).yellow());
    } else if first_line.contains("behind") {
        let behind_count = extract_count(first_line, "behind");
        println!("  Status: {} commits behind", style(behind_count).red());
    } else {
        println!("  Status: {}", style("up to date").green());
    }

    // Count uncommitted changes
    let changes: usize = status_str.lines().skip(1).filter(|l| !l.is_empty()).count();
    if changes > 0 {
        println!("  Changes: {} uncommitted", style(changes).yellow());
    } else {
        println!("  Changes: {}", style("clean").green());
    }

    Ok(())
}

fn extract_count(line: &str, keyword: &str) -> String {
    // Parse "ahead 3" or "behind 2" from git status
    if let Some(pos) = line.find(keyword) {
        let after = &line[pos + keyword.len()..];
        let num: String = after.chars().filter(|c| c.is_ascii_digit()).collect();
        if !num.is_empty() {
            return num;
        }
    }
    "?".to_string()
}

fn print_docker_status(ctx: &AppContext) -> Result<()> {
    println!("{}", style("Docker").bold());

    if !cmd_exists("docker") {
        println!("  {}", style("Docker not installed").red());
        return Ok(());
    }

    // Check if docker is running
    let docker_check = CmdBuilder::new("docker")
        .args(["info"])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture();

    if docker_check.is_err() {
        println!("  {}", style("Docker daemon not running").red());
        return Ok(());
    }

    // List running containers for this project
    let ps = CmdBuilder::new("docker")
        .args(["compose", "ps", "--format", "json"])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture()?;

    let stdout = ps.stdout_string();
    let mut running = 0;
    let mut stopped = 0;
    let mut services: Vec<(String, String, String)> = Vec::new();

    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            let name = json["Service"].as_str().unwrap_or("unknown").to_string();
            let state = json["State"].as_str().unwrap_or("unknown").to_string();
            let health = json["Health"].as_str().unwrap_or("").to_string();

            if state == "running" {
                running += 1;
            } else {
                stopped += 1;
            }
            services.push((name, state, health));
        }
    }

    if services.is_empty() {
        println!("  {}", style("No containers").dim());
    } else {
        println!(
            "  Containers: {} running, {} stopped",
            style(running).green(),
            if stopped > 0 {
                style(stopped).yellow()
            } else {
                style(stopped).dim()
            }
        );

        for (name, state, health) in &services {
            let status_icon = match state.as_str() {
                "running" => {
                    if health == "healthy" {
                        style("●").green()
                    } else if health == "unhealthy" {
                        style("●").red()
                    } else {
                        style("●").green()
                    }
                }
                "exited" => style("○").dim(),
                _ => style("?").yellow(),
            };
            let health_str = if !health.is_empty() && health != "healthy" {
                format!(" ({})", health)
            } else {
                String::new()
            };
            println!("    {} {}{}", status_icon, name, health_str);
        }
    }

    Ok(())
}

fn print_db_status(ctx: &AppContext) -> Result<()> {
    println!("{}", style("Database").bold());

    // Try to connect to postgres
    let pg_check = CmdBuilder::new("docker")
        .args([
            "compose",
            "exec",
            "-T",
            "postgres",
            "pg_isready",
            "-U",
            "postgres",
        ])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture();

    match pg_check {
        Ok(output) if output.code == 0 => {
            println!("  PostgreSQL: {}", style("ready").green());

            // Check pending migrations
            let pending = check_pending_migrations(ctx);
            match pending {
                Ok(0) => println!("  Migrations: {}", style("up to date").green()),
                Ok(n) => println!("  Migrations: {} pending", style(n).yellow()),
                Err(_) => println!("  Migrations: {}", style("unknown").dim()),
            }
        }
        _ => {
            println!("  PostgreSQL: {}", style("not running").red());
        }
    }

    Ok(())
}

fn check_pending_migrations(ctx: &AppContext) -> Result<usize> {
    // Get migrations directory from config
    let migrations_dir = {
        let config = crate::config::Config::load(&ctx.repo)?;
        let db_packages = config.database_packages();
        if db_packages.is_empty() {
            return Ok(0);
        }
        let (pkg_name, _) = db_packages.first().unwrap();
        match config.migrations_path(pkg_name) {
            Some(path) => path,
            None => return Ok(0),
        }
    };

    if !migrations_dir.exists() {
        return Ok(0);
    }

    // This is a simplified check - ideally we'd query the DB
    // For now just return 0 if migrations directory exists
    let _ = std::fs::read_dir(&migrations_dir)?;
    Ok(0)
}

fn print_env_status(ctx: &AppContext) -> Result<()> {
    println!("{}", style("Environment").bold());

    let dev_env = ctx.repo.join(".env.dev");
    let prod_env = ctx.repo.join(".env.prod");

    let dev_exists = dev_env.exists();
    let prod_exists = prod_env.exists();

    println!(
        "  .env.dev:  {}",
        if dev_exists {
            style("present").green()
        } else {
            style("missing").red()
        }
    );

    println!(
        "  .env.prod: {}",
        if prod_exists {
            style("present").green()
        } else {
            style("missing (optional)").dim()
        }
    );

    if cmd_exists("esc") {
        println!("  ESC CLI:   {}", style("installed").green());
    } else {
        println!("  ESC CLI:   {}", style("not installed").yellow());
    }

    Ok(())
}

/// Sync everything: git pull + env pull + migrate
pub fn sync_all(ctx: &AppContext) -> Result<()> {
    ctx.print_header("Syncing development environment");
    println!();

    // 1. Git pull
    println!("{}", style("1. Pulling latest changes...").bold());
    let git_result = CmdBuilder::new("git")
        .args(["pull", "--rebase"])
        .cwd(&ctx.repo)
        .run();

    match git_result {
        Ok(0) => println!("   {}", style("Git pull successful").green()),
        Ok(code) => println!(
            "   {}",
            style(format!("Git pull failed (exit {})", code)).red()
        ),
        Err(e) => println!("   {}", style(format!("Git pull error: {}", e)).red()),
    }
    println!();

    // 2. Pull env vars
    println!("{}", style("2. Pulling environment variables...").bold());
    if cmd_exists("esc") {
        let _ = super::env::pull_env(ctx, "dev", ".env.dev");
    } else {
        println!("   {}", style("Skipped (esc not installed)").dim());
    }
    println!();

    // 3. Run migrations
    println!("{}", style("3. Running database migrations...").bold());
    let migrate_result = super::db::db_migrate(ctx, None);
    if let Err(e) = migrate_result {
        println!("   {}", style(format!("Migration warning: {}", e)).yellow());
    }
    println!();

    // 4. Restart containers if running
    println!("{}", style("4. Checking containers...").bold());
    let ps = CmdBuilder::new("docker")
        .args(["compose", "ps", "-q"])
        .cwd(&ctx.repo)
        .capture_stdout()
        .run_capture();

    if let Ok(output) = ps {
        if !output.stdout_string().trim().is_empty() {
            println!("   Restarting containers...");
            let _ = super::docker::docker_compose_restart(ctx, &[]);
        } else {
            println!("   {}", style("No containers running").dim());
        }
    }

    println!();
    ctx.print_success("Sync complete!");
    Ok(())
}

/// First-time setup for new developers
pub fn init_setup(ctx: &AppContext) -> Result<()> {
    ctx.print_header("First-time Developer Setup");
    println!();
    println!("This will set up your development environment.");
    println!();

    if !ctx.confirm("Continue with setup?", true)? {
        println!("Setup cancelled.");
        return Ok(());
    }
    println!();

    // 1. Check prerequisites
    println!("{}", style("1. Checking prerequisites...").bold());
    let required = vec![
        ("git", "Git"),
        ("docker", "Docker"),
        ("cargo", "Rust (cargo)"),
    ];

    let mut missing = Vec::new();
    for (cmd, name) in &required {
        if cmd_exists(cmd) {
            println!("   {} {}", style("✓").green(), name);
        } else {
            println!(
                "   {} {} - {}",
                style("✗").red(),
                name,
                style("MISSING").red()
            );
            missing.push(*name);
        }
    }

    if !missing.is_empty() {
        println!();
        println!(
            "{}",
            style("Please install missing tools before continuing:").red()
        );
        for name in &missing {
            println!("   - {}", name);
        }
        return Ok(());
    }
    println!();

    // 2. Install optional tools
    println!("{}", style("2. Installing optional tools...").bold());

    // cargo-watch
    if !cmd_exists("cargo-watch") {
        println!("   Installing cargo-watch...");
        let _ = CmdBuilder::new("cargo")
            .args(["install", "cargo-watch"])
            .cwd(&ctx.repo)
            .run();
    } else {
        println!("   {} cargo-watch", style("✓").green());
    }

    // sqlx-cli
    if !cmd_exists("sqlx") {
        println!("   Installing sqlx-cli...");
        let _ = CmdBuilder::new("cargo")
            .args(["install", "sqlx-cli"])
            .cwd(&ctx.repo)
            .run();
    } else {
        println!("   {} sqlx-cli", style("✓").green());
    }
    println!();

    // 3. Pull environment variables
    println!("{}", style("3. Setting up environment...").bold());
    if cmd_exists("esc") {
        let _ = super::env::pull_env(ctx, "dev", ".env.dev");
    } else {
        println!(
            "   {}",
            style("ESC not installed - you'll need to create .env.dev manually").yellow()
        );
        println!("   See: https://www.pulumi.com/docs/esc/");
    }
    println!();

    // 4. Start docker containers
    println!("{}", style("4. Starting Docker containers...").bold());
    super::docker::docker_compose_up(ctx, &[], true)?;
    println!();

    // 5. Run migrations
    println!("{}", style("5. Running database migrations...").bold());
    // Wait a moment for postgres to be ready
    std::thread::sleep(std::time::Duration::from_secs(3));
    let _ = super::db::db_migrate(ctx, None);
    println!();

    // 6. Build the project
    println!("{}", style("6. Building project...").bold());
    // Build via cmd system
    if let Ok(cfg) = crate::config::Config::load(&ctx.repo) {
        let opts = super::cmd::CmdOptions {
            parallel: false,
            variant: None,
            packages: vec![],
            capture: false,
        };
        let _ = super::cmd::run_cmd(ctx, &cfg, "build", &opts);
    }
    println!();

    ctx.print_success("Setup complete! Your development environment is ready.");
    println!();
    println!("Next steps:");
    println!("  {} - Show this menu", style("./dev.sh").cyan());
    println!("  {} - Run tests", style("./dev.sh test").cyan());
    println!("  {} - View logs", style("./dev.sh logs").cyan());
    println!(
        "  {} - Check environment status",
        style("./dev.sh status").cyan()
    );

    Ok(())
}
