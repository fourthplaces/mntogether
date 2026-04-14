//! Layout engine — height-balanced placement with density progression.
//!
//! Takes a county_id and date range, finds eligible posts, then assigns them
//! to row template slots using:
//! - **Height balancing**: stacked cells match the lead cell's height (via height_units)
//! - **Family consistency**: prefer same post_type within a row
//! - **Density progression**: heavy features at top → medium mid → light/ticker bottom
//!
//! The algorithm is greedy (no backtracking) but produces visually balanced broadsheets
//! by using integer height estimates on post templates.

use std::collections::HashMap;

use anyhow::Result;
use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::editions::data::types::{
    BroadsheetDraft, BroadsheetRow, BroadsheetSection, BroadsheetSlot, BroadsheetWidgetRow,
    LayoutPost,
};
use crate::domains::editions::models::post_template_config::PostTemplateConfig;
use crate::domains::editions::models::row_template_config::RowTemplateWithSlots;
use crate::domains::editions::models::row_template_config::RowTemplateConfig;
use crate::domains::widgets::models::widget::Widget;
use crate::kernel::ServerDeps;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Generate a broadsheet draft for a county and date range.
pub async fn generate_broadsheet(
    county_id: Uuid,
    period_start: NaiveDate,
    _period_end: NaiveDate,
    deps: &ServerDeps,
) -> Result<BroadsheetDraft> {
    let pool = &deps.db_pool;

    let posts = load_county_posts(county_id, period_start, pool).await?;

    let heavy_count = posts.iter().filter(|p| p.weight == "heavy").count();
    let medium_count = posts.iter().filter(|p| p.weight == "medium").count();
    let light_count = posts.iter().filter(|p| p.weight == "light").count();
    tracing::info!(
        county_id = %county_id,
        total = posts.len(),
        heavy = heavy_count,
        medium = medium_count,
        light = light_count,
        "Layout engine: loaded posts by weight tier"
    );

    let templates = RowTemplateConfig::find_all_with_slots(pool).await?;
    let post_templates = PostTemplateConfig::find_all(pool).await?;

    // Build height_units lookup from post templates
    let height_map: HashMap<String, i32> = post_templates
        .iter()
        .map(|pt| (pt.slug.clone(), pt.height_units))
        .collect();

    let mut draft = place_posts(posts, &templates, &post_templates, &height_map);

    // Load evergreen widgets available for this county+date and interleave
    // widget-standalone rows into the draft at rule-based positions.
    let available_widgets = Widget::find_available(county_id, period_start, pool).await?;
    let widget_standalone_id = templates
        .iter()
        .find(|t| t.config.layout_variant == "widget-standalone")
        .map(|t| t.config.id);
    if let Some(row_template_id) = widget_standalone_id {
        draft.widget_rows = place_widgets(&draft.rows, &available_widgets, row_template_id);
    }

    tracing::info!(
        rows = draft.rows.len(),
        widget_rows = draft.widget_rows.len(),
        total_slots = draft.rows.iter().map(|r| r.slots.len()).sum::<usize>(),
        sections = draft.sections.len(),
        "Layout engine: placement complete"
    );

    Ok(draft)
}

/// Place widget-standalone rows at rule-based positions within the broadsheet.
///
/// Rules (applied best-effort; widget availability may skip some positions):
///   - After row 2: section_sep (establishes mid-page transition)
///   - After row 4: pull_quote (editorial interlude)
///   - After row 6: section_sep (another section break)
///   - After row 8: resource_bar (support info near the bottom)
///   - After row 9: section_sep (before dense/classifieds zone)
///
/// Widgets are picked round-robin from the available pool of each type.
///
/// Section separator variant:
///   - Default: ledger (center-aligned) — feels like a "chapter break"
///   - When the FOLLOWING row is a ticker-style full-width left-aligned row,
///     use the default section-sep (left-aligned) so the separator aligns with
///     the content below it.
fn place_widgets(
    rows: &[BroadsheetRow],
    widgets: &[Widget],
    row_template_id: Uuid,
) -> Vec<BroadsheetWidgetRow> {
    if rows.is_empty() || widgets.is_empty() {
        return Vec::new();
    }

    // Group widgets by type for round-robin selection
    let mut by_type: HashMap<&str, Vec<&Widget>> = HashMap::new();
    for w in widgets {
        by_type.entry(w.widget_type.as_str()).or_default().push(w);
    }

    // (insert_after_row_index, widget_type) — template chosen per-insert below
    let rules: [(usize, &str); 5] = [
        (2, "section_sep"),
        (4, "pull_quote"),
        (6, "section_sep"),
        (8, "resource_bar"),
        (9, "section_sep"),
    ];

    // Track used widgets to round-robin without immediate repeat
    let mut type_cursor: HashMap<&str, usize> = HashMap::new();
    let mut result = Vec::new();

    for (after_idx, wtype) in rules {
        // Skip rules that would insert past the last row (we still allow == last)
        if after_idx >= rows.len() {
            continue;
        }
        let pool = match by_type.get(wtype) {
            Some(p) if !p.is_empty() => p,
            _ => continue,
        };
        let cursor = type_cursor.entry(wtype).or_insert(0);
        let picked = pool[*cursor % pool.len()];
        *cursor += 1;

        // For section_sep, pick left-aligned (default) when followed by a
        // left-aligned full-width row (ticker / ticker-updates). Otherwise use
        // the centered ledger variant as the editorial default.
        let widget_template = if wtype == "section_sep" {
            let next_row = rows.get(after_idx + 1);
            let next_is_left_aligned_full = next_row
                .map(|r| {
                    r.row_template_slug == "ticker" || r.row_template_slug == "ticker-updates"
                })
                .unwrap_or(false);
            if next_is_left_aligned_full {
                None // section-sep default (left-aligned)
            } else {
                Some("ledger".to_string()) // led-section-break (center-aligned)
            }
        } else {
            None
        };

        result.push(BroadsheetWidgetRow {
            widget_id: picked.id,
            widget_template,
            insert_after: after_idx,
            row_template_id,
        });
    }

    result
}

// ---------------------------------------------------------------------------
// Core placement
// ---------------------------------------------------------------------------

/// The core placement algorithm. Pure function — no I/O.
///
/// Strategy:
/// 1. Select row templates in density-progression order (hero → mid → dense)
/// 2. Fill each row's slots with height-balanced, family-consistent posts
/// 3. Only emit fully-filled rows (no partial rows)
fn place_posts(
    posts: Vec<LayoutPost>,
    templates: &[RowTemplateWithSlots],
    post_templates: &[PostTemplateConfig],
    height_map: &HashMap<String, i32>,
) -> BroadsheetDraft {
    if posts.is_empty() {
        return BroadsheetDraft { rows: vec![], sections: vec![], widget_rows: vec![] };
    }

    let heavy_count = posts.iter().filter(|p| p.weight == "heavy").count();
    let medium_count = posts.iter().filter(|p| p.weight == "medium").count();
    let light_count = posts.iter().filter(|p| p.weight == "light").count();

    // Select row templates with density progression
    let selected_rows = select_row_templates(heavy_count, medium_count, light_count, templates);

    tracing::info!(
        selected = selected_rows.len(),
        templates = ?selected_rows.iter().map(|(cfg, _)| &cfg.slug).collect::<Vec<_>>(),
        "Layout engine: selected row templates"
    );

    let mut placed: Vec<bool> = vec![false; posts.len()];
    let mut broadsheet_rows: Vec<BroadsheetRow> = Vec::new();

    for (template, template_with_slots) in &selected_rows {
        let row = fill_row(
            template,
            template_with_slots,
            &posts,
            &mut placed,
            post_templates,
            height_map,
        );

        if let Some(row) = row {
            broadsheet_rows.push(row);
        }
    }

    // Density progression ordering: hero rows first, then mid, then dense.
    // Within each density zone, sort by max_priority descending.
    order_rows_by_density(&mut broadsheet_rows, templates);

    let total_placed = placed.iter().filter(|&&p| p).count();
    tracing::info!(
        placed = total_placed,
        unplaced = posts.len() - total_placed,
        rows = broadsheet_rows.len(),
        "Layout engine: slot filling summary"
    );

    let sections = build_topic_sections(&broadsheet_rows, &posts);

    BroadsheetDraft {
        rows: broadsheet_rows,
        sections,
        widget_rows: Vec::new(), // Widget placement runs separately in generate_broadsheet.
    }
}

/// Fill a single row template with posts, using height-balanced placement.
///
/// Returns None if the row can't be fully filled (all slot groups must be satisfied).
fn fill_row(
    template: &RowTemplateConfig,
    template_with_slots: &RowTemplateWithSlots,
    posts: &[LayoutPost],
    placed: &mut [bool],
    post_templates: &[PostTemplateConfig],
    height_map: &HashMap<String, i32>,
) -> Option<BroadsheetRow> {
    let mut row_slots: Vec<BroadsheetSlot> = Vec::new();
    let mut row_max_priority: i32 = 0;
    let mut anchor_type: Option<String> = None;
    let mut anchor_height: i32 = 0;

    // Group slots by slot_index to identify anchor vs. stacked slots.
    // Slot index 0 is always the anchor (widest/heaviest cell).
    let slots = &template_with_slots.slots;

    // Identify if this is a stacked layout (lead-stack, pair-stack)
    let is_stacked = matches!(
        template.layout_variant.as_str(),
        "lead-stack" | "pair-stack"
    );

    for slot_def in slots {
        let is_anchor = slot_def.slot_index == 0;

        // For stacked layouts, the non-anchor slots should height-balance
        let target_height = if is_stacked && !is_anchor {
            anchor_height
        } else {
            0 // No height target for anchor or non-stacked layouts
        };

        let filled = fill_slot_group(
            slot_def,
            posts,
            placed,
            post_templates,
            height_map,
            &anchor_type,
            target_height,
            template.layout_variant.as_str(),
        );

        if filled.is_empty() {
            // Can't fill this slot group — abandon the entire row.
            // Undo any placements we already made for this row.
            for slot in &row_slots {
                if let Some(idx) = posts.iter().position(|p| p.id == slot.post_id) {
                    placed[idx] = false;
                }
            }
            tracing::debug!(
                template = %template.slug,
                slot_index = slot_def.slot_index,
                "Layout engine: abandoned row — unfillable slot"
            );
            return None;
        }

        // Track anchor info for family consistency and height balancing
        if is_anchor {
            if let Some(first) = filled.first() {
                if let Some(post) = posts.iter().find(|p| p.id == first.post_id) {
                    anchor_type = Some(post.post_type.clone());
                }
                anchor_height = filled.iter()
                    .map(|s| height_map.get(&s.post_template_slug).copied().unwrap_or(4))
                    .sum();
            }
        }

        for slot in &filled {
            if let Some(post) = posts.iter().find(|p| p.id == slot.post_id) {
                if post.priority > row_max_priority {
                    row_max_priority = post.priority;
                }
            }
        }

        row_slots.extend(filled);
    }

    Some(BroadsheetRow {
        row_template_slug: template.slug.clone(),
        row_template_id: template.id,
        slots: row_slots,
        max_priority: row_max_priority,
    })
}

/// Fill a single slot group (one slot_index with N posts).
///
/// For stacked layouts with a target_height > 0, fills greedily until
/// cumulative height_units ≈ target_height.
///
/// Applies family consistency: prefers posts matching anchor_type.
fn fill_slot_group(
    slot_def: &crate::domains::editions::models::row_template_slot::RowTemplateSlot,
    posts: &[LayoutPost],
    placed: &mut [bool],
    post_templates: &[PostTemplateConfig],
    height_map: &HashMap<String, i32>,
    anchor_type: &Option<String>,
    target_height: i32,
    layout_variant: &str,
) -> Vec<BroadsheetSlot> {
    // Guard: light posts should never be the anchor (slot 0) in pair layouts.
    // Light content in a span-3 column looks visually wrong.
    if slot_def.weight == "light" && slot_def.slot_index == 0
        && (layout_variant == "pair" || layout_variant == "pair-stack")
    {
        return Vec::new();
    }

    let mut filled: Vec<BroadsheetSlot> = Vec::new();
    let mut cumulative_height: i32 = 0;

    // Build candidate list: all unplaced posts matching weight + type
    let mut candidates: Vec<(usize, &LayoutPost, i32)> = Vec::new();
    for (i, post) in posts.iter().enumerate() {
        if placed[i] {
            continue;
        }
        if post.weight != slot_def.weight {
            continue;
        }
        if !slot_def.accepts_type(&post.post_type) {
            continue;
        }

        // Verify a compatible post template exists
        if resolve_post_template(post, slot_def, post_templates).is_none() {
            continue;
        }

        // Score: priority + type consistency bonus.
        // In trio/classifieds rows (multiple posts in one slot group),
        // matching the anchor's post_type is heavily rewarded so all items
        // in the row share the same visual treatment (harmony within).
        let mut score = post.priority;
        if let Some(ref anchor) = anchor_type {
            if &post.post_type == anchor {
                score += 50; // Strong type consistency bonus
            }
        }

        candidates.push((i, post, score));
    }

    // Sort by score descending (priority + family bonus)
    candidates.sort_by(|a, b| b.2.cmp(&a.2));

    // Track the type of the first placed post for intra-group consistency.
    // When multiple posts share a slot group (trio cells, classifieds),
    // prefer matching types for visual harmony.
    // Track first post's type AND template for intra-group consistency.
    // "Harmony within" — all posts in a cell should share the same visual treatment.
    let mut group_type: Option<String> = None;
    let mut group_template: Option<String> = None;

    for (i, _post, _score) in &candidates {
        if filled.len() as i32 >= slot_def.count {
            break;
        }

        let pt_slug = resolve_post_template(posts.get(*i).unwrap(), slot_def, post_templates)
            .expect("already verified above");

        // After the first post, prefer same type AND same template
        if filled.len() > 0 {
            let type_matches = group_type.as_ref().map_or(true, |gt| &posts[*i].post_type == gt);
            let template_matches = group_template.as_ref().map_or(true, |gt| &pt_slug == gt);

            if !type_matches || !template_matches {
                // Skip if enough matching candidates remain
                let slots_remaining = slot_def.count as usize - filled.len();
                let remaining_matching = candidates.iter()
                    .filter(|(ci, _, _)| {
                        if placed[*ci] { return false; }
                        let type_ok = group_type.as_ref().map_or(true, |gt| &posts[*ci].post_type == gt);
                        if !type_ok { return false; }
                        // Check template compatibility
                        resolve_post_template(&posts[*ci], slot_def, post_templates)
                            .map_or(false, |slug| group_template.as_ref().map_or(true, |gt| &slug == gt))
                    })
                    .count();
                if remaining_matching >= slots_remaining {
                    continue;
                }
            }
        }

        let h = height_map.get(&pt_slug).copied().unwrap_or(4);

        // Height-balance check for stacked slots
        if target_height > 0 && !filled.is_empty() {
            let would_be = cumulative_height + h;
            let overshoot = would_be - target_height;
            let undershoot = target_height - cumulative_height;
            if overshoot > 0 && overshoot > undershoot {
                break;
            }
        }

        if group_type.is_none() {
            group_type = Some(posts[*i].post_type.clone());
        }
        if group_template.is_none() {
            group_template = Some(pt_slug.clone());
        }

        filled.push(BroadsheetSlot {
            post_id: posts[*i].id,
            post_template_slug: pt_slug,
            slot_index: slot_def.slot_index,
        });
        placed[*i] = true;
        cumulative_height += h;
    }

    filled
}

/// Resolve the post template slug for a post in a slot.
fn resolve_post_template(
    post: &LayoutPost,
    slot_def: &crate::domains::editions::models::row_template_slot::RowTemplateSlot,
    post_templates: &[PostTemplateConfig],
) -> Option<String> {
    // If the slot specifies a required template, only that template is allowed.
    // No fallback — if the post isn't compatible, it doesn't belong in this slot.
    if let Some(ref slug) = slot_def.post_template_slug {
        if post_templates.iter().any(|pt| &pt.slug == slug && pt.is_compatible(&post.post_type)) {
            return Some(slug.clone());
        }
        return None;
    }
    // No required template — find any compatible template matching the slot weight
    find_compatible_post_template(post, &slot_def.weight, post_templates)
}

// ---------------------------------------------------------------------------
// Row template selection
// ---------------------------------------------------------------------------

/// Select row templates to accommodate the weight distribution of posts.
///
/// Three-phase approach for density progression:
/// 1. Hero zone: templates with heavy slots (lead-stack, full, lead)
/// 2. Mid zone: medium-focused templates (trio, pair, lead)
/// 3. Dense zone: light-focused templates (classifieds, tickers)
fn select_row_templates<'a>(
    mut heavy: usize,
    mut medium: usize,
    mut light: usize,
    templates: &'a [RowTemplateWithSlots],
) -> Vec<(&'a RowTemplateConfig, &'a RowTemplateWithSlots)> {
    let mut selected: Vec<(&RowTemplateConfig, &RowTemplateWithSlots)> = Vec::new();
    let max_rows = 12;
    let mut variant_counts: HashMap<&str, usize> = HashMap::new();

    // Phase 1: Hero rows (templates with heavy slots). Cap at 3.
    let max_hero_rows = 3.min(heavy);
    for _ in 0..max_hero_rows {
        if heavy == 0 { break; }

        let last_slug = selected.last().map(|(cfg, _)| cfg.slug.as_str());
        let best = pick_best_template(
            templates, heavy, medium, light, last_slug, &variant_counts, &selected,
            |tws| tws.slots.iter().any(|s| s.weight == "heavy"),
        );

        if let Some(tws) = best {
            deduct_slots(tws, &mut heavy, &mut medium, &mut light);
            *variant_counts.entry(tws.config.layout_variant.as_str()).or_insert(0) += 1;
            selected.push((&tws.config, tws));
        } else {
            break;
        }
    }

    // Phase 2: Medium-focused rows
    let max_mid_rows = 6.min(max_rows - selected.len());
    for _ in 0..max_mid_rows {
        if medium == 0 { break; }

        let last_slug = selected.last().map(|(cfg, _)| cfg.slug.as_str());
        let best = pick_best_template(
            templates, heavy, medium, light, last_slug, &variant_counts, &selected,
            |tws| {
                tws.slots.iter().any(|s| s.weight == "medium")
                    && !tws.slots.iter().any(|s| s.weight == "heavy")
            },
        );

        if let Some(tws) = best {
            deduct_slots(tws, &mut heavy, &mut medium, &mut light);
            *variant_counts.entry(tws.config.layout_variant.as_str()).or_insert(0) += 1;
            selected.push((&tws.config, tws));
        } else {
            break;
        }
    }

    // Phase 3: Light/dense rows (tickers, classifieds, digests)
    let remaining = max_rows - selected.len();
    for _ in 0..remaining {
        if light == 0 { break; }

        let last_slug = selected.last().map(|(cfg, _)| cfg.slug.as_str());
        let best = pick_best_template(
            templates, heavy, medium, light, last_slug, &variant_counts, &selected,
            |tws| {
                tws.slots.iter().all(|s| s.weight == "light")
            },
        );

        if let Some(tws) = best {
            deduct_slots(tws, &mut heavy, &mut medium, &mut light);
            *variant_counts.entry(tws.config.layout_variant.as_str()).or_insert(0) += 1;
            selected.push((&tws.config, tws));
        } else {
            break;
        }
    }

    selected
}

/// Pick the best template from candidates, applying variety scoring.
fn pick_best_template<'a, F>(
    templates: &'a [RowTemplateWithSlots],
    heavy: usize,
    medium: usize,
    light: usize,
    last_slug: Option<&str>,
    variant_counts: &HashMap<&str, usize>,
    already_selected: &[(&RowTemplateConfig, &RowTemplateWithSlots)],
    filter: F,
) -> Option<&'a RowTemplateWithSlots>
where
    F: Fn(&RowTemplateWithSlots) -> bool,
{
    let mut best: Option<(&RowTemplateWithSlots, f64)> = None;

    for tws in templates {
        if !filter(tws) {
            continue;
        }

        // Score by how many slots can be filled
        let mut score = 0usize;
        let mut h = heavy;
        let mut m = medium;
        let mut l = light;

        for slot in &tws.slots {
            let available = match slot.weight.as_str() {
                "heavy" => &mut h,
                "medium" => &mut m,
                "light" => &mut l,
                _ => continue,
            };
            let consume = (slot.count as usize).min(*available);
            score += consume;
            *available -= consume;
        }

        if score == 0 {
            continue;
        }

        // Check that ALL slots can be filled (avoid partial rows)
        let total_needed: usize = tws.slots.iter().map(|s| s.count as usize).sum();
        if score < total_needed {
            continue; // Can't fully fill this template
        }

        let mut adj_score = score as f64;

        // Avoid repeating the same slug consecutively
        if let Some(last) = last_slug {
            if tws.config.slug == last {
                adj_score *= 0.3;
            }
        }

        // Block consecutive same layout_variant (harmony within, diversity between)
        if let Some((last_cfg, _)) = already_selected.last() {
            if tws.config.layout_variant == last_cfg.layout_variant {
                continue; // Hard block: never place same variant back-to-back
            }
        }

        // Penalize overused layout variants
        let variant_count = variant_counts
            .get(tws.config.layout_variant.as_str())
            .copied()
            .unwrap_or(0);
        // Full-width rows limited to 2 per edition (allows two ticker groups
        // at different page positions, but not an overwhelming wall of tickers)
        if tws.config.layout_variant == "full" && variant_count >= 2 {
            continue;
        }
        if variant_count >= 2 {
            adj_score *= 0.3;
        }

        // Boost templates not yet used
        let already = already_selected.iter().any(|(cfg, _)| cfg.slug == tws.config.slug);
        if !already {
            adj_score *= 1.5;
        }

        if best.is_none() || adj_score > best.unwrap().1 {
            best = Some((tws, adj_score));
        }
    }

    best.map(|(tws, _)| tws)
}

/// Deduct post counts consumed by a row template's slots.
fn deduct_slots(tws: &RowTemplateWithSlots, heavy: &mut usize, medium: &mut usize, light: &mut usize) {
    for slot in &tws.slots {
        let available = match slot.weight.as_str() {
            "heavy" => &mut *heavy,
            "medium" => &mut *medium,
            "light" => &mut *light,
            _ => continue,
        };
        let consume = (slot.count as usize).min(*available);
        *available -= consume;
    }
}

// ---------------------------------------------------------------------------
// Row ordering — density progression
// ---------------------------------------------------------------------------

/// Order rows by density zone, then by max_priority within each zone.
///
/// Zone 1 (hero): rows with heavy posts → top of broadsheet
/// Zone 2 (mid): rows with medium posts → middle
/// Zone 3 (dense): rows with only light posts → bottom
fn order_rows_by_density(rows: &mut Vec<BroadsheetRow>, templates: &[RowTemplateWithSlots]) {
    // Build a slug → layout_variant lookup
    let variant_map: HashMap<&str, &str> = templates
        .iter()
        .map(|t| (t.config.slug.as_str(), t.config.layout_variant.as_str()))
        .collect();

    // Classify each row into a density zone
    fn density_zone(row: &BroadsheetRow, variant_map: &HashMap<&str, &str>) -> i32 {
        let variant = variant_map.get(row.row_template_slug.as_str()).copied().unwrap_or("full");
        // Hero zone: lead-stack and full rows tend to have heavy content
        // But we classify by the actual template, not just variant
        match variant {
            "lead-stack" | "lead" => 0, // Hero zone
            "full" => {
                // Full-width could be hero (single feature) or dense (tickers)
                // Use max_priority as a heuristic: high priority = hero
                if row.max_priority >= 70 { 0 } else { 2 }
            }
            "pair" | "pair-stack" | "trio" => 1, // Mid zone
            _ => 2, // Dense zone
        }
    }

    rows.sort_by(|a, b| {
        let zone_a = density_zone(a, &variant_map);
        let zone_b = density_zone(b, &variant_map);
        zone_a.cmp(&zone_b).then(b.max_priority.cmp(&a.max_priority))
    });

    // Post-sort: break up consecutive same-variant rows ("harmony within,
    // diversity amongst"). When two adjacent rows share a layout_variant,
    // scan forward for the first row with a different variant and swap it in.
    let len = rows.len();
    for i in 1..len {
        let prev_variant = variant_map
            .get(rows[i - 1].row_template_slug.as_str())
            .copied()
            .unwrap_or("");
        let curr_variant = variant_map
            .get(rows[i].row_template_slug.as_str())
            .copied()
            .unwrap_or("");
        if prev_variant == curr_variant {
            // Find the first non-matching row ahead and swap
            if let Some(swap_idx) = (i + 1..len).find(|&j| {
                variant_map
                    .get(rows[j].row_template_slug.as_str())
                    .copied()
                    .unwrap_or("")
                    != curr_variant
            }) {
                rows.swap(i, swap_idx);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Find the best compatible post template for a post, based on slot weight.
fn find_compatible_post_template(
    post: &LayoutPost,
    slot_weight: &str,
    post_templates: &[PostTemplateConfig],
) -> Option<String> {
    // First: weight match + type compatible
    for pt in post_templates {
        if pt.weight == slot_weight && pt.is_compatible(&post.post_type) {
            return Some(pt.slug.clone());
        }
    }
    // Fallback: any compatible template
    for pt in post_templates {
        if pt.is_compatible(&post.post_type) {
            return Some(pt.slug.clone());
        }
    }
    None
}

/// Load active posts relevant to a county.
async fn load_county_posts(
    county_id: Uuid,
    period_start: NaiveDate,
    pool: &PgPool,
) -> Result<Vec<LayoutPost>> {
    #[derive(Debug, sqlx::FromRow)]
    struct PostRow {
        id: Uuid,
        post_type: Option<String>,
        weight: Option<String>,
        priority: Option<i32>,
    }

    let rows = sqlx::query_as::<_, PostRow>(
        r#"
        SELECT DISTINCT p.id, p.post_type, p.weight, p.priority
        FROM posts p
        LEFT JOIN locationables la
            ON la.locatable_id = p.id
            AND la.locatable_type = 'post'
            AND la.is_primary = true
        LEFT JOIN locations loc ON loc.id = la.location_id
        LEFT JOIN zip_counties zc ON loc.postal_code = zc.zip_code
        WHERE p.status = 'active'
          AND (
            zc.county_id = $1
            OR la.id IS NULL
            OR EXISTS (
              SELECT 1 FROM taggables t
              JOIN tags tg ON t.tag_id = tg.id
              WHERE t.taggable_type = 'post'
                AND t.taggable_id = p.id
                AND tg.value = 'statewide'
            )
          )
          AND (p.published_at IS NULL OR p.published_at >= ($2::date - INTERVAL '7 days'))
        ORDER BY p.priority DESC NULLS LAST
        "#,
    )
    .bind(county_id)
    .bind(period_start)
    .fetch_all(pool)
    .await?;

    let post_ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let topic_tags = load_topic_tags(&post_ids, pool).await?;

    let posts = rows
        .into_iter()
        .map(|r| {
            let topic_slug = topic_tags.get(&r.id).cloned();
            LayoutPost {
                id: r.id,
                post_type: r.post_type.unwrap_or_else(|| "update".to_string()),
                weight: r.weight.unwrap_or_else(|| "medium".to_string()),
                priority: r.priority.unwrap_or(50),
                topic_slug,
            }
        })
        .collect();

    Ok(posts)
}

/// Load topic tags for a batch of post IDs.
async fn load_topic_tags(
    post_ids: &[Uuid],
    pool: &PgPool,
) -> Result<HashMap<Uuid, String>> {
    if post_ids.is_empty() {
        return Ok(HashMap::new());
    }

    #[derive(Debug, sqlx::FromRow)]
    struct TopicRow {
        post_id: Uuid,
        topic_slug: String,
    }

    let rows = sqlx::query_as::<_, TopicRow>(
        r#"
        SELECT t.taggable_id AS post_id, tg.value AS topic_slug
        FROM taggables t
        JOIN tags tg ON t.tag_id = tg.id
        WHERE t.taggable_type = 'post'
          AND tg.kind = 'topic'
          AND t.taggable_id = ANY($1)
        "#,
    )
    .bind(post_ids)
    .fetch_all(pool)
    .await?;

    let mut map = HashMap::new();
    for row in rows {
        map.entry(row.post_id).or_insert(row.topic_slug);
    }

    Ok(map)
}

/// Build topic sections from the placed rows.
fn build_topic_sections(
    rows: &[BroadsheetRow],
    posts: &[LayoutPost],
) -> Vec<BroadsheetSection> {
    let topic_by_id: HashMap<Uuid, &str> = posts
        .iter()
        .filter_map(|p| p.topic_slug.as_ref().map(|t| (p.id, t.as_str())))
        .collect();

    let row_topics: Vec<Option<&str>> = rows
        .iter()
        .map(|row| {
            let mut topic_counts: HashMap<&str, usize> = HashMap::new();
            for slot in &row.slots {
                if let Some(topic) = topic_by_id.get(&slot.post_id) {
                    *topic_counts.entry(topic).or_insert(0) += 1;
                }
            }
            topic_counts
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(topic, _)| topic)
        })
        .collect();

    let mut sections: Vec<BroadsheetSection> = Vec::new();
    let mut current_topic: Option<&str> = None;

    for (i, topic) in row_topics.iter().enumerate() {
        match topic {
            Some(t) => {
                if current_topic == Some(*t) {
                    if let Some(section) = sections.last_mut() {
                        section.row_indices.push(i);
                    }
                } else {
                    let title = humanize_topic(t);
                    sections.push(BroadsheetSection {
                        title,
                        subtitle: None,
                        topic_slug: Some(t.to_string()),
                        row_indices: vec![i],
                    });
                    current_topic = Some(*t);
                }
            }
            None => {
                current_topic = None;
            }
        }
    }

    sections
}

/// Convert a topic slug to a human-readable title.
fn humanize_topic(slug: &str) -> String {
    slug.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// =============================================================================
// Unit tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_post(topic: Option<&str>) -> LayoutPost {
        LayoutPost {
            id: Uuid::new_v4(),
            post_type: "story".to_string(),
            weight: "medium".to_string(),
            priority: 50,
            topic_slug: topic.map(|t| t.to_string()),
        }
    }

    fn make_row(posts: &[&LayoutPost]) -> BroadsheetRow {
        BroadsheetRow {
            row_template_slug: "trio-gazette".to_string(),
            row_template_id: Uuid::new_v4(),
            slots: posts
                .iter()
                .enumerate()
                .map(|(i, p)| BroadsheetSlot {
                    post_id: p.id,
                    post_template_slug: "gazette".to_string(),
                    slot_index: i as i32,
                })
                .collect(),
            max_priority: 50,
        }
    }

    #[test]
    fn humanize_single_word() {
        assert_eq!(humanize_topic("housing"), "Housing");
    }

    #[test]
    fn humanize_hyphenated_slug() {
        assert_eq!(humanize_topic("food-access"), "Food Access");
    }

    #[test]
    fn humanize_three_word_slug() {
        assert_eq!(humanize_topic("mental-health-services"), "Mental Health Services");
    }

    #[test]
    fn build_sections_empty_rows() {
        let sections = build_topic_sections(&[], &[]);
        assert!(sections.is_empty());
    }

    #[test]
    fn build_sections_no_topics() {
        let p1 = make_post(None);
        let p2 = make_post(None);
        let rows = vec![make_row(&[&p1]), make_row(&[&p2])];
        let posts = vec![p1, p2];

        let sections = build_topic_sections(&rows, &posts);
        assert!(sections.is_empty(), "rows without topics should not create sections");
    }

    #[test]
    fn build_sections_single_topic() {
        let p1 = make_post(Some("housing"));
        let p2 = make_post(Some("housing"));
        let rows = vec![make_row(&[&p1]), make_row(&[&p2])];
        let posts = vec![p1, p2];

        let sections = build_topic_sections(&rows, &posts);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].title, "Housing");
        assert_eq!(sections[0].topic_slug.as_deref(), Some("housing"));
        assert_eq!(sections[0].row_indices, vec![0, 1]);
    }

    #[test]
    fn build_sections_two_topics() {
        let p1 = make_post(Some("housing"));
        let p2 = make_post(Some("food-access"));
        let rows = vec![make_row(&[&p1]), make_row(&[&p2])];
        let posts = vec![p1, p2];

        let sections = build_topic_sections(&rows, &posts);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].topic_slug.as_deref(), Some("housing"));
        assert_eq!(sections[0].row_indices, vec![0]);
        assert_eq!(sections[1].topic_slug.as_deref(), Some("food-access"));
        assert_eq!(sections[1].row_indices, vec![1]);
    }

    #[test]
    fn build_sections_ungrouped_between_topics() {
        let p1 = make_post(Some("housing"));
        let p2 = make_post(None);
        let p3 = make_post(Some("housing"));
        let rows = vec![make_row(&[&p1]), make_row(&[&p2]), make_row(&[&p3])];
        let posts = vec![p1, p2, p3];

        let sections = build_topic_sections(&rows, &posts);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].row_indices, vec![0]);
        assert_eq!(sections[1].row_indices, vec![2]);
    }

    #[test]
    fn build_sections_dominant_topic_in_mixed_row() {
        let p1 = make_post(Some("housing"));
        let p2 = make_post(Some("housing"));
        let p3 = make_post(Some("food-access"));
        let rows = vec![make_row(&[&p1, &p2, &p3])];
        let posts = vec![p1, p2, p3];

        let sections = build_topic_sections(&rows, &posts);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].topic_slug.as_deref(), Some("housing"));
    }
}
