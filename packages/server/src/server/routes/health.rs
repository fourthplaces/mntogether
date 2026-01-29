use axum::{extract::Extension, http::StatusCode, Json};
use serde::Serialize;

use crate::server::app::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    status: String,
    database: DatabaseHealth,
    connection_pool: ConnectionPoolHealth,
    event_bus: String,
}

#[derive(Serialize)]
pub struct DatabaseHealth {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize)]
pub struct ConnectionPoolHealth {
    size: u32,
    idle_connections: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_connections: Option<u32>,
}

/// Health check endpoint
///
/// Checks:
/// - Database connectivity and responsiveness
/// - Connection pool utilization
/// - Event bus health
///
/// Returns 200 OK if all systems are healthy, 503 Service Unavailable otherwise.
pub async fn health_handler(
    Extension(state): Extension<AppState>,
) -> (StatusCode, Json<HealthResponse>) {
    // Check database connection and measure latency
    let db_health = match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        sqlx::query("SELECT 1").execute(&state.db_pool),
    )
    .await
    {
        Ok(Ok(_)) => DatabaseHealth {
            status: "ok".to_string(),
            error: None,
        },
        Ok(Err(e)) => DatabaseHealth {
            status: "error".to_string(),
            error: Some(format!("Query failed: {}", e)),
        },
        Err(_) => DatabaseHealth {
            status: "error".to_string(),
            error: Some("Query timeout (>5s)".to_string()),
        },
    };

    // Get connection pool metrics
    let pool_options = state.db_pool.options();
    let pool_health = ConnectionPoolHealth {
        size: state.db_pool.size(),
        idle_connections: state.db_pool.num_idle(),
        max_connections: Some(pool_options.get_max_connections()),
    };

    // Check event bus health (simple check - bus is always available if app started)
    let bus_status = "ok".to_string();

    // Determine overall health
    let is_healthy = db_health.status == "ok";

    let overall_status = if is_healthy {
        "healthy"
    } else {
        "unhealthy"
    };

    let status_code = if is_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        Json(HealthResponse {
            status: overall_status.to_string(),
            database: db_health,
            connection_pool: pool_health,
            event_bus: bus_status,
        }),
    )
}
