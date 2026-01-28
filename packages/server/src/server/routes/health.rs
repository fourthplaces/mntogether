use axum::{http::StatusCode, Json};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Serialize)]
pub struct HealthResponse {
    status: String,
    database: String,
}

/// Health check endpoint
pub async fn health_handler(pool: axum::extract::State<PgPool>) -> (StatusCode, Json<HealthResponse>) {
    // Check database connection
    let db_status = match sqlx::query("SELECT 1").execute(&*pool).await {
        Ok(_) => "ok",
        Err(_) => "error",
    };

    let overall_status = if db_status == "ok" { "healthy" } else { "unhealthy" };

    let status_code = if db_status == "ok" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        Json(HealthResponse {
            status: overall_status.to_string(),
            database: db_status.to_string(),
        }),
    )
}
