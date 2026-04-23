//! `dev apikey` subcommands — issue, rotate, revoke, list service-client
//! API keys (Root Signal tokens). Talks to `api_keys` directly via sqlx; no
//! HTTP layer involved.
//!
//! Connection: reads `DATABASE_URL` from the environment, then falls back to
//! the compose default `postgres://postgres:postgres@localhost:5432/rooteditorial`
//! when the var isn't set (matches what the Makefile + docker-compose expose).
//!
//! Token format: `rsk_{env}_<32-char-url-safe-base64>` per spec §14.1.
//! Plaintext is printed exactly once at issuance and never stored.

use anyhow::{Context, Result};
use clap::Subcommand;
use server_core::common::ApiKeyId;
use server_core::domains::posts::models::ApiKey;
use sqlx::PgPool;

#[derive(Subcommand, Debug)]
pub enum ApikeyCommand {
    /// Mint a new API key for a service client.
    Issue {
        /// Client name (e.g. `root-signal-prod`, `root-signal-staging`).
        #[arg(long)]
        client: String,
        /// Environment indicator baked into the token prefix. Valid: live | test | dev.
        #[arg(long, default_value = "dev")]
        env: String,
        /// Comma-separated scope list (e.g. `posts:create`).
        #[arg(long, default_value = "posts:create")]
        scopes: String,
    },
    /// Rotate the active key for a client: issue a new one, `rotated_from_id`
    /// set to the prior active row, both active until explicit revoke.
    Rotate {
        /// Client name whose active key should be rotated.
        #[arg(long)]
        client: String,
        #[arg(long, default_value = "dev")]
        env: String,
    },
    /// Revoke a key by id or by the active key for a client.
    Revoke {
        #[arg(long, conflicts_with = "client", required_unless_present = "client")]
        id: Option<String>,
        #[arg(long, conflicts_with = "id", required_unless_present = "id")]
        client: Option<String>,
    },
    /// List all keys (active + revoked).
    List,
}

pub async fn run(cmd: ApikeyCommand) -> Result<()> {
    let _ = dotenvy::dotenv();
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@localhost:5432/rooteditorial".into()
    });
    let pool = PgPool::connect(&url)
        .await
        .with_context(|| format!("connect to {url}"))?;

    match cmd {
        ApikeyCommand::Issue { client, env, scopes } => issue(&pool, &client, &env, &scopes).await,
        ApikeyCommand::Rotate { client, env } => rotate(&pool, &client, &env).await,
        ApikeyCommand::Revoke { id, client } => revoke(&pool, id.as_deref(), client.as_deref()).await,
        ApikeyCommand::List => list(&pool).await,
    }
}

fn validate_env(env: &str) -> Result<()> {
    match env {
        "live" | "test" | "dev" => Ok(()),
        _ => anyhow::bail!("invalid --env: {env} (expected live | test | dev)"),
    }
}

async fn issue(pool: &PgPool, client: &str, env: &str, scopes: &str) -> Result<()> {
    validate_env(env)?;
    let scope_list: Vec<String> = scopes
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let issued = ApiKey::issue(client, env, &scope_list, pool).await?;
    print_issued(&issued, client);
    Ok(())
}

async fn rotate(pool: &PgPool, client: &str, env: &str) -> Result<()> {
    validate_env(env)?;
    let existing = ApiKey::find_active_by_client(client, pool)
        .await?
        .with_context(|| format!("no active key for client '{client}'"))?;
    let issued = ApiKey::rotate(existing.id, env, pool).await?;
    println!("Rotated.");
    println!("  prior id : {}", existing.id);
    println!("  (still active — run `apikey revoke --id {}` after cutover)", existing.id);
    print_issued(&issued, client);
    Ok(())
}

async fn revoke(pool: &PgPool, id: Option<&str>, client: Option<&str>) -> Result<()> {
    let target = match (id, client) {
        (Some(id_str), _) => ApiKeyId::parse(id_str).context("invalid UUID for --id")?,
        (None, Some(c)) => {
            ApiKey::find_active_by_client(c, pool)
                .await?
                .with_context(|| format!("no active key for client '{c}'"))?
                .id
        }
        (None, None) => anyhow::bail!("must supply --id or --client"),
    };
    ApiKey::revoke(target, pool).await?;
    println!("Revoked key {}", target);
    Ok(())
}

async fn list(pool: &PgPool) -> Result<()> {
    let rows = ApiKey::list_all(pool).await?;
    if rows.is_empty() {
        println!("No API keys.");
        return Ok(());
    }
    println!(
        "{:38}  {:24}  {:12}  {:20}  {:20}  scopes",
        "id", "client", "state", "created_at", "last_used_at"
    );
    for row in rows {
        let state = if row.revoked_at.is_some() {
            "revoked"
        } else {
            "active"
        };
        let last_used = row
            .last_used_at
            .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "-".into());
        println!(
            "{:38}  {:24}  {:12}  {}  {}  [{}]",
            row.id,
            truncate(&row.client_name, 24),
            state,
            row.created_at.format("%Y-%m-%d %H:%M:%S"),
            last_used,
            row.scopes.join(", ")
        );
    }
    Ok(())
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n { s.to_string() } else { format!("{}…", &s[..n.saturating_sub(1)]) }
}

fn print_issued(issued: &server_core::domains::posts::models::IssuedApiKey, client: &str) {
    println!();
    println!("Issued API key for client: {client}");
    println!("  id       : {}", issued.record.id);
    println!("  client   : {}", issued.record.client_name);
    println!("  prefix   : {}", issued.record.prefix);
    println!("  scopes   : {}", issued.record.scopes.join(", "));
    println!();
    println!("  ==========================================================");
    println!("  | TOKEN (shown ONCE, store it now)                         |");
    println!("  | {}", issued.plaintext);
    println!("  ==========================================================");
    println!();
}
