use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::api::error::ApiResult;
use crate::api::state::AppState;
use crate::domains::heat_map::activities;
use crate::domains::heat_map::models::HeatMapPoint;

// --- Request types ---

#[derive(Debug, Deserialize)]
pub struct EmptyRequest {}

// --- Response types ---

#[derive(Debug, Serialize)]
pub struct HeatMapPointData {
    pub latitude: f64,
    pub longitude: f64,
    pub weight: f64,
    pub entity_type: String,
    pub entity_id: String,
}

#[derive(Debug, Serialize)]
pub struct HeatMapSnapshotResult {
    pub points_generated: usize,
    pub generated_at: String,
}

#[derive(Debug, Serialize)]
pub struct HeatMapResult {
    pub points: Vec<HeatMapPointData>,
    pub generated_at: String,
}

// --- Handlers ---

async fn compute_snapshot(
    State(state): State<AppState>,
    Json(_req): Json<EmptyRequest>,
) -> ApiResult<Json<HeatMapSnapshotResult>> {
    info!("HeatMap/compute_snapshot");

    let rows = activities::compute_heat_map_snapshot(&state.deps).await?;

    let count = HeatMapPoint::truncate_and_insert(&rows, &state.deps.db_pool).await?;

    let generated_at = chrono::Utc::now().to_rfc3339();

    info!(points = count, "Heat map snapshot stored");

    Ok(Json(HeatMapSnapshotResult {
        points_generated: count,
        generated_at,
    }))
}

async fn get_latest(
    State(state): State<AppState>,
    Json(_req): Json<EmptyRequest>,
) -> ApiResult<Json<HeatMapResult>> {
    let points = HeatMapPoint::find_latest(&state.deps.db_pool).await?;

    let generated_at = HeatMapPoint::latest_generated_at(&state.deps.db_pool)
        .await?
        .map(|ts| ts.to_rfc3339())
        .unwrap_or_default();

    let point_data: Vec<HeatMapPointData> = points
        .into_iter()
        .map(|p| HeatMapPointData {
            latitude: p.latitude,
            longitude: p.longitude,
            weight: p.weight,
            entity_type: p.entity_type,
            entity_id: p.entity_id.to_string(),
        })
        .collect();

    Ok(Json(HeatMapResult {
        points: point_data,
        generated_at,
    }))
}

// --- Router ---

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/HeatMap/compute_snapshot", post(compute_snapshot))
        .route("/HeatMap/get_latest", post(get_latest))
}
