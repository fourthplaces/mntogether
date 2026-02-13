//! Database schema commands (sqlx migrations)

use anyhow::{Context, Result};
use clap::Subcommand;
use devkit_core::AppContext;
use std::process::Command;

#[derive(Subcommand)]
pub enum DbCommand {
    /// Run pending sqlx migrations
    Migrate,

    /// Drop, create, and migrate the local database
    Reset,

    /// Show migration status
    Status,

    /// Open psql shell connected to the database
    Psql,
}

pub fn run(ctx: &AppContext, cmd: DbCommand) -> Result<()> {
    match cmd {
        DbCommand::Migrate => run_migrate(ctx),
        DbCommand::Reset => run_reset(ctx),
        DbCommand::Status => run_status(ctx),
        DbCommand::Psql => run_psql(ctx),
    }
}

fn get_database_url() -> Result<String> {
    std::env::var("DATABASE_URL").context(
        "DATABASE_URL not set. Create a .env file or set the environment variable.",
    )
}

fn run_migrate(ctx: &AppContext) -> Result<()> {
    ctx.print_header("Running database migrations");

    let db_url = get_database_url()?;
    let migrations_path = ctx.repo.join("packages/server/migrations");

    let status = Command::new("sqlx")
        .args([
            "migrate",
            "run",
            "--source",
            migrations_path.to_str().unwrap(),
            "--database-url",
            &db_url,
        ])
        .status()
        .context("Failed to run sqlx migrate. Is sqlx-cli installed? (cargo install sqlx-cli)")?;

    if status.success() {
        ctx.print_success("Migrations completed successfully");
    } else {
        anyhow::bail!("Migration failed with exit code: {:?}", status.code());
    }

    Ok(())
}

fn run_reset(ctx: &AppContext) -> Result<()> {
    ctx.print_header("Resetting database");

    let db_url = get_database_url()?;
    let migrations_path = ctx.repo.join("packages/server/migrations");

    ctx.print_info("Dropping database...");
    let _ = Command::new("sqlx")
        .args(["database", "drop", "-y", "--database-url", &db_url])
        .status();

    ctx.print_info("Creating database...");
    let status = Command::new("sqlx")
        .args(["database", "create", "--database-url", &db_url])
        .status()
        .context("Failed to create database")?;

    if !status.success() {
        anyhow::bail!("Failed to create database");
    }

    ctx.print_info("Running migrations...");
    let status = Command::new("sqlx")
        .args([
            "migrate",
            "run",
            "--source",
            migrations_path.to_str().unwrap(),
            "--database-url",
            &db_url,
        ])
        .status()
        .context("Failed to run migrations")?;

    if status.success() {
        ctx.print_success("Database reset complete");
    } else {
        anyhow::bail!("Migration failed");
    }

    Ok(())
}

fn run_status(ctx: &AppContext) -> Result<()> {
    ctx.print_header("Migration status");

    let db_url = get_database_url()?;
    let migrations_path = ctx.repo.join("packages/server/migrations");

    let status = Command::new("sqlx")
        .args([
            "migrate",
            "info",
            "--source",
            migrations_path.to_str().unwrap(),
            "--database-url",
            &db_url,
        ])
        .status()
        .context("Failed to get migration status")?;

    if !status.success() {
        anyhow::bail!("Failed to get migration status");
    }

    Ok(())
}

fn run_psql(ctx: &AppContext) -> Result<()> {
    ctx.print_header("Connecting to database");

    let db_url = get_database_url()?;

    let status = Command::new("psql")
        .arg(&db_url)
        .status()
        .context("Failed to run psql. Is PostgreSQL client installed?")?;

    if !status.success() {
        anyhow::bail!("psql exited with error");
    }

    Ok(())
}
