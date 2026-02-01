// Quick script to fix migration 88 checksum
use sqlx::PgPool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    let new_checksum = "992509c80157f79489ffb52ff7b089dbdffe87f1b9f6d5938342240f5633c32d";

    sqlx::query(
        "UPDATE _sqlx_migrations SET checksum = decode($1, 'hex') WHERE version = 88"
    )
    .bind(new_checksum)
    .execute(&pool)
    .await?;

    println!("✅ Migration 88 checksum updated successfully!");

    // Verify
    let result: (i64, String, Vec<u8>) = sqlx::query_as(
        "SELECT version, description, checksum FROM _sqlx_migrations WHERE version = 88"
    )
    .fetch_one(&pool)
    .await?;

    println!("Version: {}, Description: {}, Checksum: {}",
        result.0, result.1, hex::encode(result.2));

    Ok(())
}
