//! Edition operations — create, generate, publish, unpublish, archive editions.

use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use serde::Serialize;
use uuid::Uuid;

use crate::domains::editions::data::types::BatchGenerateResult;
use crate::domains::editions::models::county::County;
use crate::domains::editions::models::edition::Edition;
use crate::domains::editions::models::edition_row::EditionRow;
use crate::domains::editions::models::edition_section::EditionSection;
use crate::domains::editions::models::edition_slot::EditionSlot;
use crate::kernel::ServerDeps;

use super::layout_engine;

/// One item in a batch lifecycle operation's error list. Carries the edition
/// id that failed and a human-readable reason (e.g. "Cannot publish an
/// edition with no populated slots."). Callers surface this directly in the
/// editor UI so a batch publish that skips empty editions tells the editor
/// *which* ones and *why*.
#[derive(Debug, Clone, Serialize)]
pub struct BatchEditionError {
    pub edition_id: Uuid,
    pub message: String,
}

/// Aggregated result from a batch lifecycle operation (approve, publish).
/// `succeeded` + `failed` == input ids. `errors` has one entry per failure
/// so the UI can render per-edition reasons.
#[derive(Debug, Clone, Serialize)]
pub struct BatchLifecycleResult {
    pub succeeded: i32,
    pub failed: i32,
    pub errors: Vec<BatchEditionError>,
}

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
            auto_title = default_edition_title(&county, period_start);
            Some(auto_title.as_str())
        }
    };

    Edition::create(county_id, period_start, period_end, effective_title, pool).await
}

/// Compose the default edition title for a county.
///
/// Real counties get `"{name} County — Week of Apr 20"`; pseudo counties
/// (e.g. Statewide) drop the trailing " County" so the title doesn't
/// render as the awkward "Statewide County — Week of Apr 20".
fn default_edition_title(county: &County, period_start: NaiveDate) -> String {
    if county.is_pseudo {
        format!("{} — Week of {}", county.name, period_start.format("%b %d"))
    } else {
        format!(
            "{} County — Week of {}",
            county.name,
            period_start.format("%b %d")
        )
    }
}

/// Generate (or re-generate) the layout for an edition using the layout engine.
/// Clears any existing rows/slots and replaces them with fresh placements.
pub async fn generate_edition(edition_id: Uuid, deps: &ServerDeps) -> Result<Edition> {
    let pool = &deps.db_pool;

    let edition = Edition::find_by_id(edition_id, pool)
        .await?
        .ok_or_else(|| anyhow!("Edition not found: {}", edition_id))?;

    if edition.status == "published" || edition.status == "archived" {
        return Err(anyhow!(
            "Cannot regenerate a {} edition — only draft, in_review, or approved editions can be regenerated",
            edition.status
        ));
    }

    // Reset to draft if the edition was in review or approved
    if edition.status != "draft" {
        Edition::reset_to_draft(edition_id, pool).await?;
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

    // Persist the broadsheet draft, interleaving widget-standalone rows
    // at their rule-based insert positions.
    //
    // Strategy: walk the post rows in order. After each post row at index `i`
    // (0-based), check if any widget row has `insert_after == i` and emit it
    // with the next sort_order. This preserves the relative order of both
    // post rows and widget rows.
    let mut edition_row_ids: Vec<Uuid> = Vec::new();
    let mut sort_order: i32 = 0;
    let widget_rows = &draft.widget_rows;

    for (post_row_idx, row) in draft.rows.iter().enumerate() {
        // Create the post row
        let edition_row =
            EditionRow::create(edition_id, row.row_template_id, sort_order, pool).await?;
        sort_order += 1;

        let slot_data: Vec<(Uuid, String, i32)> = row
            .slots
            .iter()
            .map(|s| (s.post_id, s.post_template_slug.clone(), s.slot_index))
            .collect();

        EditionSlot::replace_for_row(edition_row.id, &slot_data, pool).await?;
        edition_row_ids.push(edition_row.id);

        // Insert any widget rows that should follow this post row
        for widget_row in widget_rows.iter().filter(|w| w.insert_after == post_row_idx) {
            let widget_edition_row =
                EditionRow::create(edition_id, widget_row.row_template_id, sort_order, pool).await?;
            sort_order += 1;

            // Create one slot per widget in this row (1 for standalone, 2 for pair, 3 for trio)
            for ws in &widget_row.widgets {
                EditionSlot::create_widget_slot(
                    widget_edition_row.id,
                    ws.widget_id,
                    ws.widget_template.as_deref(),
                    ws.slot_index,
                    pool,
                )
                .await?;
            }
            edition_row_ids.push(widget_edition_row.id);
        }
    }

    // Persist sections and assign rows
    for (sort_order, section) in draft.sections.iter().enumerate() {
        let es = EditionSection::create(
            edition_id,
            &section.title,
            section.subtitle.as_deref(),
            section.topic_slug.as_deref(),
            sort_order as i32,
            pool,
        )
        .await?;

        for &row_idx in &section.row_indices {
            if let Some(&row_id) = edition_row_ids.get(row_idx) {
                EditionRow::assign_to_section(row_id, Some(es.id), pool).await?;
            }
        }
    }

    // Re-fetch to return up-to-date edition
    Edition::find_by_id(edition_id, pool)
        .await?
        .ok_or_else(|| anyhow!("Edition disappeared after generation"))
}

/// Count the number of populated slots (post or widget assigned) in
/// an edition. A slot is "populated" when either post_id or widget_id
/// is non-NULL; empty slots (neither set) are placeholders the layout
/// engine created but didn't fill.
///
/// Returned as i64 to match sqlx::query_scalar's COUNT(*) type.
async fn count_populated_slots(
    edition_id: Uuid,
    pool: &sqlx::PgPool,
) -> Result<i64> {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM edition_slots es
        JOIN edition_rows er ON er.id = es.edition_row_id
        WHERE er.edition_id = $1
          AND (es.post_id IS NOT NULL OR es.widget_id IS NOT NULL)
        "#,
    )
    .bind(edition_id)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

/// Guard: reject lifecycle transitions on editions with no populated
/// slots. Without this, editors could drive an empty edition through
/// review → approved → published, exposing a broken page on the public
/// site (the "Aitkin 16 rows / 0 slots" state that surfaced in
/// editorial review).
///
/// The gate fires on review/approve/publish. Draft stays writable so
/// the layout engine can re-run generation. Editors should click
/// "Regenerate layout" if their edition has empty rows; the gate
/// prevents them from pushing the empty state forward by accident.
async fn require_populated_edition(
    edition_id: Uuid,
    stage: &str,
    pool: &sqlx::PgPool,
) -> Result<()> {
    let n = count_populated_slots(edition_id, pool).await?;
    if n == 0 {
        return Err(anyhow!(
            "Cannot {} an edition with no populated slots. Regenerate the layout first.",
            stage
        ));
    }
    Ok(())
}

/// Transition a draft edition to in_review (editor has opened it).
pub async fn review_edition(edition_id: Uuid, deps: &ServerDeps) -> Result<Edition> {
    let pool = &deps.db_pool;

    let edition = Edition::find_by_id(edition_id, pool)
        .await?
        .ok_or_else(|| anyhow!("Edition not found: {}", edition_id))?;

    if edition.status != "draft" {
        return Err(anyhow!(
            "Cannot review a {} edition — only drafts can transition to in_review",
            edition.status
        ));
    }

    require_populated_edition(edition_id, "start review on", pool).await?;
    Edition::review(edition_id, pool).await
}

/// Approve an in_review edition (marks it ready for publication).
pub async fn approve_edition(edition_id: Uuid, deps: &ServerDeps) -> Result<Edition> {
    let pool = &deps.db_pool;

    let edition = Edition::find_by_id(edition_id, pool)
        .await?
        .ok_or_else(|| anyhow!("Edition not found: {}", edition_id))?;

    if edition.status != "in_review" {
        return Err(anyhow!(
            "Cannot approve a {} edition — only in_review editions can be approved",
            edition.status
        ));
    }

    require_populated_edition(edition_id, "approve", pool).await?;
    Edition::approve(edition_id, pool).await
}

/// Publish an approved edition.
pub async fn publish_edition(edition_id: Uuid, deps: &ServerDeps) -> Result<Edition> {
    let pool = &deps.db_pool;

    let edition = Edition::find_by_id(edition_id, pool)
        .await?
        .ok_or_else(|| anyhow!("Edition not found: {}", edition_id))?;

    if edition.status == "published" {
        return Err(anyhow!("Edition is already published"));
    }

    if edition.status != "approved" && edition.status != "draft" {
        return Err(anyhow!(
            "Cannot publish a {} edition — only approved (or draft) editions can be published",
            edition.status
        ));
    }

    require_populated_edition(edition_id, "publish", pool).await?;
    Edition::publish(edition_id, pool).await
}

/// Move a published edition back to `approved` so an editor can revise it.
/// `published_at` is preserved (first-publication timestamp is load-bearing
/// audit state); a subsequent `publish_edition` call uses COALESCE to keep
/// it, so publish → unpublish → publish is a no-op on that field.
pub async fn unpublish_edition(edition_id: Uuid, deps: &ServerDeps) -> Result<Edition> {
    let pool = &deps.db_pool;

    let edition = Edition::find_by_id(edition_id, pool)
        .await?
        .ok_or_else(|| anyhow!("Edition not found: {}", edition_id))?;

    if edition.status != "published" {
        return Err(anyhow!(
            "Cannot unpublish a {} edition — only published editions can be unpublished",
            edition.status
        ));
    }

    Edition::unpublish(edition_id, pool).await
}

/// Archive an edition.
pub async fn archive_edition(edition_id: Uuid, deps: &ServerDeps) -> Result<Edition> {
    let pool = &deps.db_pool;

    Edition::find_by_id(edition_id, pool)
        .await?
        .ok_or_else(|| anyhow!("Edition not found: {}", edition_id))?;

    Edition::archive(edition_id, pool).await
}

/// Batch approve multiple in_review editions. Iterates per-id through the
/// single-op activity so the population gate fires per edition; the model
/// layer has no corresponding raw-SQL batch method by design. Returns
/// per-id reasons for any skipped editions.
pub async fn batch_approve_editions(
    ids: &[Uuid],
    deps: &ServerDeps,
) -> Result<BatchLifecycleResult> {
    let mut succeeded = 0i32;
    let mut errors = Vec::new();
    for &id in ids {
        match approve_edition(id, deps).await {
            Ok(_) => succeeded += 1,
            Err(e) => errors.push(BatchEditionError {
                edition_id: id,
                message: e.to_string(),
            }),
        }
    }
    Ok(BatchLifecycleResult { succeeded, failed: errors.len() as i32, errors })
}

/// Batch publish multiple approved editions. Gated per-id like
/// `batch_approve_editions` — this is the fix that prevents the empty-edition
/// publish bug (raw `UPDATE ... WHERE id = ANY($1)` used to bypass the
/// population check).
pub async fn batch_publish_editions(
    ids: &[Uuid],
    deps: &ServerDeps,
) -> Result<BatchLifecycleResult> {
    let mut succeeded = 0i32;
    let mut errors = Vec::new();
    for &id in ids {
        match publish_edition(id, deps).await {
            Ok(_) => succeeded += 1,
            Err(e) => errors.push(BatchEditionError {
                edition_id: id,
                message: e.to_string(),
            }),
        }
    }
    Ok(BatchLifecycleResult { succeeded, failed: errors.len() as i32, errors })
}

/// Batch generate editions for ALL 87 counties for a given date range.
/// - Counties with no edition for this period: create + generate (counted as `created`)
/// - Counties with a draft edition: regenerate layout (counted as `regenerated`)
/// - Counties with reviewing/approved/published editions: skip (counted as `skipped`)
pub async fn batch_generate_editions(
    period_start: NaiveDate,
    period_end: NaiveDate,
    deps: &ServerDeps,
) -> Result<BatchGenerateResult> {
    let pool = &deps.db_pool;
    let counties = County::find_all(pool).await?;
    let total = counties.len() as i32;
    let mut created = 0;
    let mut regenerated = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for county in counties {
        match Edition::find_by_county_and_period(county.id, period_start, pool).await {
            Ok(Some(existing)) => match existing.status.as_str() {
                "draft" => {
                    // Regenerate existing draft with fresh layout
                    match generate_edition(existing.id, deps).await {
                        Ok(_) => regenerated += 1,
                        Err(e) => {
                            tracing::error!(
                                county_name = %county.name,
                                county_id = %county.id,
                                error = %e,
                                "Failed to regenerate draft edition"
                            );
                            failed += 1;
                        }
                    }
                }
                _ => {
                    // in_review, approved, published, archived — don't overwrite
                    skipped += 1;
                }
            },
            Ok(None) => {
                // No edition for this period — create + generate
                match create_and_generate_single(&county, period_start, period_end, deps).await
                {
                    Ok(_) => created += 1,
                    Err(e) => {
                        tracing::error!(
                            county_name = %county.name,
                            county_id = %county.id,
                            error = %e,
                            "Failed to create edition for county"
                        );
                        failed += 1;
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    county_name = %county.name,
                    county_id = %county.id,
                    error = %e,
                    "Failed to look up edition for county"
                );
                failed += 1;
            }
        }
    }

    Ok(BatchGenerateResult {
        created,
        regenerated,
        skipped,
        failed,
        total_counties: total,
    })
}

/// Helper: create and generate a single county edition.
async fn create_and_generate_single(
    county: &County,
    period_start: NaiveDate,
    period_end: NaiveDate,
    deps: &ServerDeps,
) -> Result<Edition> {
    let title = default_edition_title(county, period_start);

    let edition = Edition::create(
        county.id,
        period_start,
        period_end,
        Some(&title),
        &deps.db_pool,
    )
    .await?;

    generate_edition(edition.id, deps).await
}
