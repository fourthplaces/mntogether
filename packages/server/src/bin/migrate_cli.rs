//! CLI for executing data migrations
//!
//! This binary is called by dev-cli to perform the actual migration work.
//! It outputs JSON for parsing by the dev-cli.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Serialize;
use server_core::config::Config;
use server_core::data_migrations::{
    all_migrations, find_migration, MigrationContext, MigrationResult, MigrationWorkflow,
    VerifyResult,
};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "migrate_cli")]
#[command(about = "Data migration CLI (called by dev-cli)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all registered migrations
    List,

    /// Estimate items to migrate
    Estimate { name: String },

    /// Initialize or reset a workflow
    Init {
        name: String,
        #[arg(long)]
        dry_run: bool,
    },

    /// Run a batch of migrations
    Batch {
        name: String,
        #[arg(long)]
        dry_run: bool,
    },

    /// Get workflow status
    Status { name: String },

    /// Pause a workflow
    Pause { name: String },

    /// Resume a workflow
    Resume { name: String },

    /// Verify migration completion
    Verify { name: String },

    /// Mark migration as complete
    Complete { name: String },
}

// ============================================================================
// JSON Response Types
// ============================================================================

#[derive(Serialize)]
struct Response {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    migrations: Option<Vec<MigrationInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<WorkflowStatusResponse>,
}

#[derive(Serialize)]
struct MigrationInfo {
    name: String,
    description: Option<String>,
}

#[derive(Serialize)]
struct WorkflowStatusResponse {
    phase: String,
    total_items: i64,
    completed_items: i64,
    failed_items: i64,
    skipped_items: i64,
    error_rate: f64,
}

#[derive(Serialize)]
struct ProgressUpdate {
    #[serde(rename = "type")]
    update_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    completed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

fn output(resp: Response) {
    println!("{}", serde_json::to_string(&resp).unwrap());
}

fn output_progress(update: ProgressUpdate) {
    println!("{}", serde_json::to_string(&update).unwrap());
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List => cmd_list(),
        Commands::Estimate { name } => cmd_estimate(&name).await,
        Commands::Init { name, dry_run } => cmd_init(&name, dry_run).await,
        Commands::Batch { name, dry_run } => cmd_batch(&name, dry_run).await,
        Commands::Status { name } => cmd_status(&name).await,
        Commands::Pause { name } => cmd_pause(&name).await,
        Commands::Resume { name } => cmd_resume(&name).await,
        Commands::Verify { name } => cmd_verify(&name).await,
        Commands::Complete { name } => cmd_complete(&name).await,
    }
}

async fn get_pool() -> Result<PgPool> {
    let config = Config::from_env()?;
    PgPool::connect(&config.database_url)
        .await
        .context("Failed to connect to database")
}

// ============================================================================
// Commands
// ============================================================================

fn cmd_list() -> Result<()> {
    let migrations: Vec<MigrationInfo> = all_migrations()
        .into_iter()
        .map(|e| MigrationInfo {
            name: e.migration.name().to_string(),
            description: {
                let desc = e.migration.description();
                if desc.is_empty() {
                    None
                } else {
                    Some(desc.to_string())
                }
            },
        })
        .collect();

    output(Response {
        success: true,
        message: None,
        count: None,
        migrations: Some(migrations),
        status: None,
    });

    Ok(())
}

async fn cmd_estimate(name: &str) -> Result<()> {
    let pool = get_pool().await?;

    let entry = match find_migration(name) {
        Some(e) => e,
        None => {
            output(Response {
                success: false,
                message: Some(format!("Migration '{}' not found", name)),
                count: None,
                migrations: None,
                status: None,
            });
            return Ok(());
        }
    };

    let count = entry.migration.estimate(&pool).await?;

    output(Response {
        success: true,
        message: None,
        count: Some(count),
        migrations: None,
        status: None,
    });

    Ok(())
}

async fn cmd_init(name: &str, dry_run: bool) -> Result<()> {
    let pool = get_pool().await?;

    let entry = match find_migration(name) {
        Some(e) => e,
        None => {
            output(Response {
                success: false,
                message: Some(format!("Migration '{}' not found", name)),
                count: None,
                migrations: None,
                status: None,
            });
            return Ok(());
        }
    };

    let total = entry.migration.estimate(&pool).await?;
    let error_budget = entry.migration.error_budget();

    let workflow = MigrationWorkflow::create(name, total, dry_run, error_budget, &pool).await?;

    output(Response {
        success: true,
        message: Some(format!("Initialized workflow with {} items", total)),
        count: Some(total),
        migrations: None,
        status: Some(WorkflowStatusResponse {
            phase: workflow.phase.clone(),
            total_items: workflow.total_items,
            completed_items: workflow.completed_items,
            failed_items: workflow.failed_items,
            skipped_items: workflow.skipped_items,
            error_rate: workflow.error_rate(),
        }),
    });

    Ok(())
}

async fn cmd_batch(name: &str, dry_run: bool) -> Result<()> {
    let pool = get_pool().await?;

    let entry = match find_migration(name) {
        Some(e) => e,
        None => {
            output(Response {
                success: false,
                message: Some(format!("Migration '{}' not found", name)),
                count: None,
                migrations: None,
                status: None,
            });
            return Ok(());
        }
    };

    let migration = entry.migration;
    let batch_size = migration.batch_size();

    // Get or create workflow
    let total = migration.estimate(&pool).await?;
    let error_budget = migration.error_budget();
    let mut workflow = MigrationWorkflow::create(name, total, dry_run, error_budget, &pool).await?;

    // Send init progress
    output_progress(ProgressUpdate {
        update_type: "init".to_string(),
        completed: Some(workflow.completed_items),
        total: Some(workflow.total_items),
        result: None,
        message: None,
    });

    let ctx = MigrationContext {
        db_pool: pool.clone(),
        dry_run,
    };

    // Process batches until done
    loop {
        let cursor = workflow.last_processed_id;
        let work = migration.find_work(cursor, batch_size, &pool).await?;

        if work.is_empty() {
            break;
        }

        let mut completed = 0i64;
        let mut failed = 0i64;
        let mut skipped = 0i64;
        let mut last_id: Option<Uuid> = None;

        for id in work {
            let result = match migration.execute_one(id, &ctx).await {
                Ok(r) => r,
                Err(_) => {
                    failed += 1;
                    last_id = Some(id);
                    continue;
                }
            };

            match result {
                MigrationResult::Migrated | MigrationResult::WouldMigrate => {
                    completed += 1;
                }
                MigrationResult::Skipped | MigrationResult::WouldSkip => {
                    skipped += 1;
                }
            }

            last_id = Some(id);
        }

        // Update workflow progress
        if let Some(last) = last_id {
            workflow =
                MigrationWorkflow::update_progress(name, completed, failed, skipped, last, &pool)
                    .await?;

            output_progress(ProgressUpdate {
                update_type: "progress".to_string(),
                completed: Some(workflow.completed_items),
                total: Some(workflow.total_items),
                result: None,
                message: None,
            });

            // Check error budget
            if workflow.error_budget_exceeded() {
                MigrationWorkflow::fail(name, &pool).await?;
                output_progress(ProgressUpdate {
                    update_type: "error".to_string(),
                    completed: None,
                    total: None,
                    result: None,
                    message: Some("Error budget exceeded".to_string()),
                });
                return Ok(());
            }
        }
    }

    output_progress(ProgressUpdate {
        update_type: "complete".to_string(),
        completed: Some(workflow.completed_items),
        total: Some(workflow.total_items),
        result: None,
        message: Some(format!(
            "Completed: {}, Skipped: {}, Failed: {}",
            workflow.completed_items, workflow.skipped_items, workflow.failed_items
        )),
    });

    Ok(())
}

async fn cmd_status(name: &str) -> Result<()> {
    let pool = get_pool().await?;

    let workflow = match MigrationWorkflow::find_by_name(name, &pool).await? {
        Some(w) => w,
        None => {
            output(Response {
                success: true,
                message: Some(format!("No workflow found for '{}'", name)),
                count: None,
                migrations: None,
                status: None,
            });
            return Ok(());
        }
    };

    output(Response {
        success: true,
        message: None,
        count: None,
        migrations: None,
        status: Some(WorkflowStatusResponse {
            phase: workflow.phase.clone(),
            total_items: workflow.total_items,
            completed_items: workflow.completed_items,
            failed_items: workflow.failed_items,
            skipped_items: workflow.skipped_items,
            error_rate: workflow.error_rate(),
        }),
    });

    Ok(())
}

async fn cmd_pause(name: &str) -> Result<()> {
    let pool = get_pool().await?;

    match MigrationWorkflow::pause(name, &pool).await {
        Ok(_) => {
            output(Response {
                success: true,
                message: Some("Migration paused".to_string()),
                count: None,
                migrations: None,
                status: None,
            });
        }
        Err(e) => {
            output(Response {
                success: false,
                message: Some(format!("Failed to pause: {}", e)),
                count: None,
                migrations: None,
                status: None,
            });
        }
    }

    Ok(())
}

async fn cmd_resume(name: &str) -> Result<()> {
    let pool = get_pool().await?;

    match MigrationWorkflow::resume(name, &pool).await {
        Ok(_) => {
            output(Response {
                success: true,
                message: Some("Migration resumed".to_string()),
                count: None,
                migrations: None,
                status: None,
            });
        }
        Err(e) => {
            output(Response {
                success: false,
                message: Some(format!("Failed to resume: {}", e)),
                count: None,
                migrations: None,
                status: None,
            });
        }
    }

    Ok(())
}

async fn cmd_verify(name: &str) -> Result<()> {
    let pool = get_pool().await?;

    let entry = match find_migration(name) {
        Some(e) => e,
        None => {
            output(Response {
                success: false,
                message: Some(format!("Migration '{}' not found", name)),
                count: None,
                migrations: None,
                status: None,
            });
            return Ok(());
        }
    };

    let result = entry.migration.verify(&pool).await?;

    match result {
        VerifyResult::Passed => {
            output(Response {
                success: true,
                message: Some("Verification passed".to_string()),
                count: None,
                migrations: None,
                status: None,
            });
        }
        VerifyResult::Incomplete { remaining } => {
            output(Response {
                success: false,
                message: Some(format!("{} items remaining", remaining)),
                count: Some(remaining),
                migrations: None,
                status: None,
            });
        }
        VerifyResult::Failed { issues } => {
            output(Response {
                success: false,
                message: Some(issues.join("; ")),
                count: None,
                migrations: None,
                status: None,
            });
        }
    }

    Ok(())
}

async fn cmd_complete(name: &str) -> Result<()> {
    let pool = get_pool().await?;

    match MigrationWorkflow::complete(name, &pool).await {
        Ok(_) => {
            output(Response {
                success: true,
                message: Some("Migration marked as complete".to_string()),
                count: None,
                migrations: None,
                status: None,
            });
        }
        Err(e) => {
            output(Response {
                success: false,
                message: Some(format!("Failed to complete: {}", e)),
                count: None,
                migrations: None,
                status: None,
            });
        }
    }

    Ok(())
}
