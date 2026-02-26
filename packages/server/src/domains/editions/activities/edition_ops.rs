//! Edition operations — create, generate, publish, archive editions.

use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use uuid::Uuid;

use crate::domains::editions::data::types::BatchGenerateResult;
use crate::domains::editions::models::county::County;
use crate::domains::editions::models::edition::Edition;
use crate::domains::editions::models::edition_row::EditionRow;
use crate::domains::editions::models::edition_slot::EditionSlot;
use crate::kernel::ServerDeps;

use super::layout_engine;

/// Create a new draft edition for a county and period.
pub async fn create_edition(
    county_id: Uuid,
    period_start: NaiveDate,
    period_end: NaiveDate,
    title: Option<&str>,
    deps: &ServerDeps,
) -> Result<Edition> {
    let pool = &deps.db_pool;

    // Verify county exists (reuse result for title generation)
    let county = County::find_by_id(county_id, pool)
        .await?
        .ok_or_else(|| anyhow!("County not found: {}", county_id))?;

    // Auto-generate title if not provided
    let auto_title;
    let effective_title = match title {
        Some(t) => Some(t),
        None => {
            auto_title = format!(
                "{} County — Week of {}",
                county.name,
                period_start.format("%b %d")
            );
            Some(auto_title.as_str())
        }
    };

    Edition::create(county_id, period_start, period_end, effective_title, pool).await
}

/// Generate (or re-generate) the layout for an edition using the layout engine.
/// Clears any existing rows/slots and replaces them with fresh placements.
pub async fn generate_edition(edition_id: Uuid, deps: &ServerDeps) -> Result<Edition> {
    let pool = &deps.db_pool;

    let edition = Edition::find_by_id(edition_id, pool)
        .await?
        .ok_or_else(|| anyhow!("Edition not found: {}", edition_id))?;

    if edition.status != "draft" {
        return Err(anyhow!(
            "Cannot regenerate a {} edition — only drafts can be regenerated",
            edition.status
        ));
    }

    // Clear existing layout
    Edition::clear_layout(edition_id, pool).await?;

    // Run layout engine
    let draft = layout_engine::generate_broadsheet(
        edition.county_id,
        edition.period_start,
        edition.period_end,
        deps,
    )
    .await?;

    // Persist the broadsheet draft
    for (sort_order, row) in draft.rows.iter().enumerate() {
        let edition_row =
            EditionRow::create(edition_id, row.row_template_id, sort_order as i32, pool).await?;

        let slot_data: Vec<(Uuid, String, i32)> = row
            .slots
            .iter()
            .map(|s| (s.post_id, s.post_template_slug.clone(), s.slot_index))
            .collect();

        EditionSlot::replace_for_row(edition_row.id, &slot_data, pool).await?;
    }

    // Re-fetch to return up-to-date edition
    Edition::find_by_id(edition_id, pool)
        .await?
        .ok_or_else(|| anyhow!("Edition disappeared after generation"))
}

/// Publish an edition.
pub async fn publish_edition(edition_id: Uuid, deps: &ServerDeps) -> Result<Edition> {
    let pool = &deps.db_pool;

    let edition = Edition::find_by_id(edition_id, pool)
        .await?
        .ok_or_else(|| anyhow!("Edition not found: {}", edition_id))?;

    if edition.status == "published" {
        return Err(anyhow!("Edition is already published"));
    }

    Edition::publish(edition_id, pool).await
}

/// Archive an edition.
pub async fn archive_edition(edition_id: Uuid, deps: &ServerDeps) -> Result<Edition> {
    let pool = &deps.db_pool;

    Edition::find_by_id(edition_id, pool)
        .await?
        .ok_or_else(|| anyhow!("Edition not found: {}", edition_id))?;

    Edition::archive(edition_id, pool).await
}

/// Batch generate editions for ALL 87 counties for a given date range.
/// Creates draft editions and runs the layout engine for each.
pub async fn batch_generate_editions(
    period_start: NaiveDate,
    period_end: NaiveDate,
    deps: &ServerDeps,
) -> Result<BatchGenerateResult> {
    let pool = &deps.db_pool;
    let counties = County::find_all(pool).await?;
    let total = counties.len() as i32;
    let mut created = 0;
    let mut failed = 0;

    for county in counties {
        match create_and_generate_single(county.id, &county.name, period_start, period_end, deps)
            .await
        {
            Ok(_) => created += 1,
            Err(e) => {
                tracing::error!(
                    county_name = %county.name,
                    county_id = %county.id,
                    error = %e,
                    "Failed to generate edition for county"
                );
                failed += 1;
            }
        }
    }

    Ok(BatchGenerateResult {
        created,
        failed,
        total_counties: total,
    })
}

/// Helper: create and generate a single county edition.
async fn create_and_generate_single(
    county_id: Uuid,
    county_name: &str,
    period_start: NaiveDate,
    period_end: NaiveDate,
    deps: &ServerDeps,
) -> Result<Edition> {
    let title = format!(
        "{} County — Week of {}",
        county_name,
        period_start.format("%b %d")
    );

    let edition = Edition::create(
        county_id,
        period_start,
        period_end,
        Some(&title),
        &deps.db_pool,
    )
    .await?;

    generate_edition(edition.id, deps).await
}
