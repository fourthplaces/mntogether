// Fix migration 88 by deleting its record and letting it re-run
use sqlx::PgPool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    println!("Deleting migration 88 record...");
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 88")
        .execute(&pool)
        .await?;

    println!("✅ Migration 88 record deleted. Now run 'sqlx migrate run' to reapply it.");

    Ok(())
}
