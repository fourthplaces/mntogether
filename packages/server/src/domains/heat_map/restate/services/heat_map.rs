use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

use crate::common::EmptyRequest;
use crate::domains::heat_map::activities;
use crate::domains::heat_map::models::HeatMapPoint;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatMapPointData {
    pub latitude: f64,
    pub longitude: f64,
    pub weight: f64,
    pub entity_type: String,
    pub entity_id: String,
}

impl_restate_serde!(HeatMapPointData);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatMapSnapshotResult {
    pub points_generated: usize,
    pub generated_at: String,
}

impl_restate_serde!(HeatMapSnapshotResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatMapResult {
    pub points: Vec<HeatMapPointData>,
    pub generated_at: String,
}

impl_restate_serde!(HeatMapResult);

// =============================================================================
// Service definition
// =============================================================================

#[restate_sdk::service]
#[name = "HeatMap"]
pub trait HeatMapService {
    async fn compute_snapshot(req: EmptyRequest) -> Result<HeatMapSnapshotResult, HandlerError>;
    async fn get_latest(req: EmptyRequest) -> Result<HeatMapResult, HandlerError>;
}

pub struct HeatMapServiceImpl {
    deps: Arc<ServerDeps>,
}

impl HeatMapServiceImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl HeatMapService for HeatMapServiceImpl {
    async fn compute_snapshot(
        &self,
        ctx: Context<'_>,
        _req: EmptyRequest,
    ) -> Result<HeatMapSnapshotResult, HandlerError> {
        info!("Computing heat map snapshot");

        let rows = activities::compute_heat_map_snapshot(&self.deps)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let count = HeatMapPoint::truncate_and_insert(&rows, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let generated_at = chrono::Utc::now().to_rfc3339();

        info!(points = count, "Heat map snapshot stored");

        // Self-schedule for 1 hour
        ctx.service_client::<HeatMapServiceClient>()
            .compute_snapshot(EmptyRequest {})
            .send_after(Duration::from_secs(3600));

        Ok(HeatMapSnapshotResult {
            points_generated: count,
            generated_at,
        })
    }

    async fn get_latest(
        &self,
        _ctx: Context<'_>,
        _req: EmptyRequest,
    ) -> Result<HeatMapResult, HandlerError> {
        let points = HeatMapPoint::find_latest(&self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let generated_at = HeatMapPoint::latest_generated_at(&self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?
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

        Ok(HeatMapResult {
            points: point_data,
            generated_at,
        })
    }
}
