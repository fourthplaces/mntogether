//! Edition operations — create, generate, publish, archive editions.

use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use uuid::Uuid;

use crate::domains::editions::data::types::BatchGenerateResult;
use crate::domains::editions::models::county::County;
use crate::domains::editions::models::edition::Edition;
use crate::domains::editions::models::edition_row::EditionRow;
use crate::domains::editions::models::edition_section::EditionSection;
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

            EditionSlot::create_widget_slot(
                widget_edition_row.id,
                widget_row.widget_id,
                0,
                pool,
            )
            .await?;
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

/// Batch approve multiple in_review editions.
pub async fn batch_approve_editions(
    ids: &[Uuid],
    deps: &ServerDeps,
) -> Result<(i32, i32)> {
    let pool = &deps.db_pool;
    let total = ids.len() as i32;
    let approved = Edition::batch_approve(ids, pool).await?;
    let succeeded = approved.len() as i32;
    Ok((succeeded, total - succeeded))
}

/// Batch publish multiple approved editions.
pub async fn batch_publish_editions(
    ids: &[Uuid],
    deps: &ServerDeps,
) -> Result<(i32, i32)> {
    let pool = &deps.db_pool;
    let total = ids.len() as i32;
    let published = Edition::batch_publish(ids, pool).await?;
    let succeeded = published.len() as i32;
    Ok((succeeded, total - succeeded))
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
                match create_and_generate_single(
                    county.id,
                    &county.name,
                    period_start,
                    period_end,
                    deps,
                )
                .await
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
