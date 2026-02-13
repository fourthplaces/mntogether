use anyhow::Result;
use tracing::info;

use crate::domains::heat_map::models::HeatMapRow;
use crate::kernel::ServerDeps;

/// Compute heat map snapshot by joining locationables + noteables.
///
/// Aggregates max note severity per entity, fans out to each entity's locations,
/// and returns weighted (lat, lng) tuples with provenance.
///
/// Weight mapping: urgent=10, notice=5, info=1
pub async fn compute_heat_map_snapshot(deps: &ServerDeps) -> Result<Vec<HeatMapRow>> {
    let rows = sqlx::query_as::<_, HeatMapRow>(
        r#"
        SELECT
            l.latitude,
            l.longitude,
            MAX(CASE n.severity
                WHEN 'urgent' THEN 10.0
                WHEN 'notice' THEN 5.0
                ELSE 1.0
            END) AS weight,
            loc.locatable_type AS entity_type,
            loc.locatable_id AS entity_id
        FROM locationables loc
        JOIN locations l ON l.id = loc.location_id
        JOIN noteables nb
            ON nb.noteable_type = loc.locatable_type
            AND nb.noteable_id = loc.locatable_id
        JOIN notes n ON n.id = nb.note_id
        WHERE l.latitude IS NOT NULL
            AND l.longitude IS NOT NULL
            AND (n.expired_at IS NULL OR n.expired_at > NOW())
        GROUP BY l.latitude, l.longitude, loc.locatable_type, loc.locatable_id
        "#,
    )
    .fetch_all(&deps.db_pool)
    .await?;

    info!(points = rows.len(), "Computed heat map snapshot");
    Ok(rows)
}
