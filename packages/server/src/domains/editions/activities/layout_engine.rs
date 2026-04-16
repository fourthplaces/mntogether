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
    BroadsheetWidgetSlot, LayoutPost,
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

    // Load the per-county editorial weight target. This determines broadsheet
    // length: the layout engine aims to fill rows until the target is met,
    // flexing up to 1.3× on busy weeks and down to whatever the pool can supply.
    let target_content_weight = sqlx::query_scalar::<_, i32>(
        "SELECT target_content_weight FROM counties WHERE id = $1",
    )
    .bind(county_id)
    .fetch_one(pool)
    .await?;

    let posts = load_county_posts(county_id, period_start, pool).await?;

    let heavy_count = posts.iter().filter(|p| p.weight == "heavy").count();
    let medium_count = posts.iter().filter(|p| p.weight == "medium").count();
    let light_count = posts.iter().filter(|p| p.weight == "light").count();
    let pool_weight = (heavy_count * 3 + medium_count * 2 + light_count) as i32;
    tracing::info!(
        county_id = %county_id,
        total = posts.len(),
        heavy = heavy_count,
        medium = medium_count,
        light = light_count,
        pool_weight = pool_weight,
        target_content_weight = target_content_weight,
        "Layout engine: loaded posts by weight tier"
    );

    let templates = RowTemplateConfig::find_all_with_slots(pool).await?;
    let post_templates = PostTemplateConfig::find_all(pool).await?;

    // Build height_units lookup from post templates
    let height_map: HashMap<String, i32> = post_templates
        .iter()
        .map(|pt| (pt.slug.clone(), pt.height_units))
        .collect();

    let mut draft = place_posts(
        posts,
        &templates,
        &post_templates,
        &height_map,
        target_content_weight,
    );

    // Load evergreen widgets available for this county+date and interleave
    // widget-standalone rows into the draft at rule-based positions.
    let available_widgets = Widget::find_available(county_id, period_start, pool).await?;

    // Look up widget row template IDs by slug
    let widget_template_ids: HashMap<String, Uuid> = sqlx::query_as::<_, (String, Uuid)>(
        "SELECT slug, id FROM row_template_configs WHERE slug LIKE 'widget%'"
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .collect();

    if !widget_template_ids.is_empty() {
        draft.widget_rows = place_widgets(&draft.rows, &available_widgets, &widget_template_ids);
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
    template_ids: &HashMap<String, Uuid>,
) -> Vec<BroadsheetWidgetRow> {
    if rows.is_empty() || widgets.is_empty() {
        return Vec::new();
    }

    let standalone_id = match template_ids.get("widget-standalone") {
        Some(id) => *id,
        None => return Vec::new(),
    };
    let trio_id = template_ids.get("widget-trio").copied().unwrap_or(standalone_id);
    let pair_id = template_ids.get("widget-pair").copied().unwrap_or(standalone_id);

    // Group widgets by type for round-robin selection
    let mut by_type: HashMap<&str, Vec<&Widget>> = HashMap::new();
    for w in widgets {
        by_type.entry(w.widget_type.as_str()).or_default().push(w);
    }

    // Widget placement rules.
    // "count" = how many widgets in the row (1=standalone, 2=pair, 3=trio).
    // Never two widget rows back-to-back (enforced after rule application).
    struct Rule { after: usize, wtype: &'static str, count: usize }
    // Rules use 0-based post row indices. The last rule must leave at least
    // 1 post row after it (no dangling section_sep with nothing below).
    let last_row = rows.len().saturating_sub(1);
    let rules = [
        Rule { after: 2, wtype: "section_sep",  count: 1 },
        Rule { after: 4, wtype: "number",       count: 3 },  // stat-card trio
        Rule { after: 5, wtype: "section_sep",  count: 1 },
        Rule { after: 6, wtype: "photo",        count: 1 },
        Rule { after: 7, wtype: "pull_quote",   count: 1 },
        Rule { after: 8, wtype: "number",       count: 2 },  // number-block pair
        Rule { after: 9, wtype: "resource_bar", count: 1 },
    ];

    let mut type_cursor: HashMap<&str, usize> = HashMap::new();
    let mut result: Vec<BroadsheetWidgetRow> = Vec::new();

    for rule in &rules {
        if rule.after >= rows.len() { continue; }
        // Section seps must have at least one post row after them
        if rule.wtype == "section_sep" && rule.after >= last_row { continue; }

        let pool = match by_type.get(rule.wtype) {
            Some(p) if p.len() >= rule.count => p,
            _ => continue,
        };

        // No back-to-back: skip if the previous widget row inserts at the
        // SAME position (both after the same post row). Widgets at adjacent
        // positions (e.g., after row 6 and after row 7) are fine — there's
        // a post row between them.
        if let Some(last) = result.last() {
            if last.insert_after == rule.after {
                continue; // Would be truly adjacent — skip this rule
            }
        }

        let cursor = type_cursor.entry(rule.wtype).or_insert(0);

        // For multi-widget rows (count > 1), all widgets must share the same
        // visual variant ("harmony within"). Filter to widgets matching the
        // first pick's widget_template.
        let first_pick = pool[*cursor % pool.len()];
        let first_template = first_pick.data.get("widget_template")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let filtered: Vec<&&Widget> = if rule.count > 1 {
            pool.iter()
                .filter(|w| {
                    w.data.get("widget_template")
                        .and_then(|v| v.as_str())
                        .unwrap_or("") == first_template
                })
                .collect()
        } else {
            pool.iter().collect()
        };
        if filtered.len() < rule.count { continue; }

        // Pick widget(s) round-robin from the filtered (same-variant) pool
        let mut slots: Vec<BroadsheetWidgetSlot> = Vec::new();
        let mut local_cursor = 0usize;
        for i in 0..rule.count {
            let picked = filtered[local_cursor % filtered.len()];
            local_cursor += 1;

            let widget_template = if rule.wtype == "number" {
                picked.data.get("widget_template")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            } else if rule.wtype == "section_sep" {
                let next_row = rows.get(rule.after + 1);
                let next_is_ticker = next_row
                    .map(|r| r.row_template_slug == "ticker" || r.row_template_slug == "ticker-updates")
                    .unwrap_or(false);
                if next_is_ticker { None } else { Some("ledger".to_string()) }
            } else {
                None
            };

            slots.push(BroadsheetWidgetSlot {
                widget_id: picked.id,
                widget_template,
                slot_index: i as i32,
            });
        }

        let row_template_id = match rule.count {
            3 => trio_id,
            2 => pair_id,
            _ => standalone_id,
        };

        // Advance global cursor so the next pick of this type starts fresh
        *cursor += rule.count;

        result.push(BroadsheetWidgetRow {
            widgets: slots,
            insert_after: rule.after,
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
    target_content_weight: i32,
) -> BroadsheetDraft {
    if posts.is_empty() {
        return BroadsheetDraft { rows: vec![], sections: vec![], widget_rows: vec![] };
    }

    let heavy_count = posts.iter().filter(|p| p.weight == "heavy").count();
    let medium_count = posts.iter().filter(|p| p.weight == "medium").count();
    let light_count = posts.iter().filter(|p| p.weight == "light").count();

    let mut selected_rows = select_row_templates(
        heavy_count,
        medium_count,
        light_count,
        templates,
        target_content_weight,
        &posts,
        post_templates,
    );

    // Reorder: specialty templates fill BEFORE generic ones so they get
    // first pick of type-restricted posts (alert-notice needs update/action,
    // generous-exchange needs need/aid, etc.). Generic templates (gazette,
    // bulletin, digest) accept any type and fill whatever's left.
    let generic_templates = ["three-column", "classifieds", "classifieds-ledger",
        "classifieds-ledger-alt", "hero-with-sidebar", "hero-feature-digest",
        "hero-feature-ledger", "lead-feature-gazette", "pair-stack-gazette"];
    selected_rows.sort_by(|(a, _), (b, _)| {
        let a_generic = generic_templates.contains(&a.slug.as_str());
        let b_generic = generic_templates.contains(&b.slug.as_str());
        a_generic.cmp(&b_generic) // false (specialty) before true (generic)
    });

    tracing::info!(
        selected = selected_rows.len(),
        templates = ?selected_rows.iter().map(|(cfg, _)| &cfg.slug).collect::<Vec<_>>(),
        "Layout engine: selected row templates (specialty-first order)"
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

    // Multi-cell layouts should keep side-by-side cells visually aligned.
    // In any layout where cells sit next to each other (lead-stack, pair-stack,
    // trio, pair, classifieds), non-anchor cells height-balance against the
    // anchor cell so one column doesn't tower over its siblings.
    let is_multi_cell = matches!(
        template.layout_variant.as_str(),
        "lead-stack" | "pair-stack" | "trio" | "pair" | "classifieds"
    );

    for slot_def in slots {
        let is_anchor = slot_def.slot_index == 0;

        // For multi-cell layouts, non-anchor cells target the anchor's height
        // when they stack (count > 1). A count=1 cell can't be balanced —
        // whatever single post fits goes in.
        let target_height = if is_multi_cell && !is_anchor && slot_def.count > 1 {
            anchor_height
        } else {
            0
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
                    .map(|s| {
                        let post_type = posts
                            .iter()
                            .find(|p| p.id == s.post_id)
                            .map(|p| p.post_type.as_str())
                            .unwrap_or("");
                        effective_height(&s.post_template_slug, post_type, height_map)
                    })
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

        let h = effective_height(&pt_slug, &posts[*i].post_type, height_map);

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

/// Slot-weight score for a single "heavy"/"medium"/"light" slot.
/// Effective rendered height for a (post_template, post_type) pairing.
///
/// The stored `height_units` is a single scalar per template, but several
/// templates render substantially different components depending on the
/// post's type. Reference posts rendered via `ledger` or `bulletin` show a
/// full items list (LedgerReference, BulletinReference) that's ~2× the
/// height of the same template rendering an update or exchange. Without
/// this adjustment, stacking two reference posts in a narrow cell produces
/// a visually unbalanced column that doesn't match its siblings.
///
/// Returns the template's base height plus a known-outlier bonus.
fn effective_height(
    template_slug: &str,
    post_type: &str,
    height_map: &HashMap<String, i32>,
) -> i32 {
    let base = height_map.get(template_slug).copied().unwrap_or(4);
    let bonus = match (template_slug, post_type) {
        // Reference posts render an items list; in the compact ledger/bulletin
        // templates this roughly doubles the rendered height.
        ("ledger", "reference") => 3,
        ("bulletin", "reference") => 3,
        _ => 0,
    };
    base + bonus
}

fn weight_score(w: &str) -> i32 {
    match w {
        "heavy" => 3,
        "medium" => 2,
        "light" => 1,
        _ => 0,
    }
}

/// Total editorial weight produced by a single row template (sum of slot weights).
fn row_weight(tws: &RowTemplateWithSlots) -> i32 {
    tws.slots
        .iter()
        .map(|s| weight_score(&s.weight) * s.count as i32)
        .sum()
}

/// Select row templates to accommodate the weight distribution of posts,
/// sized by the county's editorial weight target.
///
/// Weight-budget approach (heavy=3, medium=2, light=1):
/// - Phase 1 (Hero zone):  20% of target — templates with heavy slots
/// - Phase 2 (Mid zone):   50% of target — medium-focused templates
/// - Phase 3 (Dense zone): 30% of target — light-focused templates
///
/// Each phase stops when its weight budget is filled OR the matching post pool
/// runs out, whichever comes first. A soft overshoot of 1.3× the total target
/// is permitted — once we've reached it, generation stops even if posts remain.
/// This lets the broadsheet flex naturally: short on slow news weeks, longer
/// on busy ones, never runaway.
fn select_row_templates<'a>(
    mut heavy: usize,
    mut medium: usize,
    mut light: usize,
    templates: &'a [RowTemplateWithSlots],
    target_content_weight: i32,
    posts: &[LayoutPost],
    post_templates: &[PostTemplateConfig],
) -> Vec<(&'a RowTemplateConfig, &'a RowTemplateWithSlots)> {
    let mut selected: Vec<(&RowTemplateConfig, &RowTemplateWithSlots)> = Vec::new();
    let mut variant_counts: HashMap<&str, usize> = HashMap::new();

    // Track per-(weight,type) counts of unconsumed posts so we can reject
    // templates whose slots can't actually be filled. Without this, the engine
    // picks templates whose slot weights match the pool but whose `accepts`
    // arrays don't — the row later fails to fill, but the raw weight counts
    // have already been deducted, starving Phase 3.
    let mut pool_by_type: HashMap<(String, String), usize> = HashMap::new();
    for p in posts {
        *pool_by_type
            .entry((p.weight.clone(), p.post_type.clone()))
            .or_insert(0) += 1;
    }

    // Weight budgets per phase. These add up to 100% of target; the 1.3× soft
    // ceiling is applied over the total accumulated weight (not per-phase).
    let hero_budget = (target_content_weight as f64 * 0.20).round() as i32;
    let mid_budget = (target_content_weight as f64 * 0.50).round() as i32;
    let dense_budget = (target_content_weight as f64 * 0.30).round() as i32;
    let overshoot_ceiling = (target_content_weight as f64 * 1.30).round() as i32;

    let mut accumulated: i32 = 0;

    // Phase 1: Hero rows (templates with heavy slots)
    let mut hero_accum: i32 = 0;
    while hero_accum < hero_budget && heavy > 0 && accumulated < overshoot_ceiling {
        let last_slug = selected.last().map(|(cfg, _)| cfg.slug.as_str());
        let best = pick_best_template(
            templates, heavy, medium, light, last_slug, &variant_counts, &selected,
            &pool_by_type, post_templates,
            |tws| tws.slots.iter().any(|s| s.weight == "heavy"),
        );

        if let Some(tws) = best {
            let rw = row_weight(tws);
            deduct_slots(tws, &mut heavy, &mut medium, &mut light);
            deduct_from_pool_by_type(tws, &mut pool_by_type, post_templates);
            *variant_counts.entry(tws.config.layout_variant.as_str()).or_insert(0) += 1;
            selected.push((&tws.config, tws));
            hero_accum += rw;
            accumulated += rw;
        } else {
            break;
        }
    }

    // Phase 2: Medium-focused rows
    let mut mid_accum: i32 = 0;
    while mid_accum < mid_budget && medium > 0 && accumulated < overshoot_ceiling {
        let last_slug = selected.last().map(|(cfg, _)| cfg.slug.as_str());
        let best = pick_best_template(
            templates, heavy, medium, light, last_slug, &variant_counts, &selected,
            &pool_by_type, post_templates,
            |tws| {
                tws.slots.iter().any(|s| s.weight == "medium")
                    && !tws.slots.iter().any(|s| s.weight == "heavy")
            },
        );

        if let Some(tws) = best {
            let rw = row_weight(tws);
            deduct_slots(tws, &mut heavy, &mut medium, &mut light);
            deduct_from_pool_by_type(tws, &mut pool_by_type, post_templates);
            *variant_counts.entry(tws.config.layout_variant.as_str()).or_insert(0) += 1;
            selected.push((&tws.config, tws));
            mid_accum += rw;
            accumulated += rw;
        } else {
            break;
        }
    }

    // Phase 3: Light/dense rows (tickers, classifieds, digests)
    let mut dense_accum: i32 = 0;
    while dense_accum < dense_budget && light > 0 && accumulated < overshoot_ceiling {
        let last_slug = selected.last().map(|(cfg, _)| cfg.slug.as_str());
        let best = pick_best_template(
            templates, heavy, medium, light, last_slug, &variant_counts, &selected,
            &pool_by_type, post_templates,
            |tws| tws.slots.iter().all(|s| s.weight == "light"),
        );

        if let Some(tws) = best {
            let rw = row_weight(tws);
            deduct_slots(tws, &mut heavy, &mut medium, &mut light);
            deduct_from_pool_by_type(tws, &mut pool_by_type, post_templates);
            *variant_counts.entry(tws.config.layout_variant.as_str()).or_insert(0) += 1;
            selected.push((&tws.config, tws));
            dense_accum += rw;
            accumulated += rw;
        } else {
            break;
        }
    }

    tracing::info!(
        target = target_content_weight,
        hero_budget = hero_budget,
        mid_budget = mid_budget,
        dense_budget = dense_budget,
        hero_filled = hero_accum,
        mid_filled = mid_accum,
        dense_filled = dense_accum,
        total_filled = accumulated,
        rows = selected.len(),
        "Layout engine: weight budget filled"
    );

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
    pool_by_type: &HashMap<(String, String), usize>,
    post_templates: &[PostTemplateConfig],
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

        // Type-compatibility pre-check: ensure each slot can find enough posts
        // whose post_type is accepted by the slot AND compatible with the
        // slot's required post_template (if any). Without this, we'd pick
        // templates like `pair-exchange` (needs need/aid) when the pool only
        // has action/event/reference mediums, then fail to fill later — but
        // raw weight counts would already be deducted, starving later phases.
        if !can_fill_template(tws, pool_by_type, post_templates) {
            continue;
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

        // Strongly boost templates not yet used — ensures dormant templates
        // (pair-exchange, trio-pinboard, pair-spotlight, etc.) surface
        let already = already_selected.iter().any(|(cfg, _)| cfg.slug == tws.config.slug);
        if !already {
            adj_score *= 2.5;
        }

        // Extra boost for specialty templates that use restricted post templates.
        // Without this, generic templates (gazette in 3 slots = score 7.5) always
        // outscore specialty templates (generous-exchange in 2 slots = score 5.0).
        let has_specialty_template = tws.slots.iter().any(|s| {
            matches!(s.post_template_slug.as_deref(), Some(
                "alert-notice" | "card-event" | "directory-ref" | "generous-exchange" |
                "pinboard-exchange" | "whisper-notice" | "spotlight-local" | "feature-reversed" |
                "quick-ref"
            ))
        });
        if has_specialty_template && !already {
            adj_score *= 2.0; // Stacks with the novelty boost above
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

/// Check if a row template's slots can be filled by the current pool,
/// respecting both weight AND post_type/template compatibility. Simulates
/// consumption greedily on a cloned pool — does not mutate the input.
///
/// Returns true iff every slot has enough matching posts available.
fn can_fill_template(
    tws: &RowTemplateWithSlots,
    pool_by_type: &HashMap<(String, String), usize>,
    post_templates: &[PostTemplateConfig],
) -> bool {
    let mut remaining = pool_by_type.clone();
    for slot in &tws.slots {
        let mut need = slot.count as usize;
        let keys: Vec<(String, String)> = remaining.keys().cloned().collect();
        for key in keys {
            if need == 0 {
                break;
            }
            let (w, t) = &key;
            if w != &slot.weight {
                continue;
            }
            if !slot.accepts_type(t) {
                continue;
            }
            // Verify a compatible post_template exists for this (slot,type)
            let pt_compat = if let Some(ref tmpl_slug) = slot.post_template_slug {
                post_templates
                    .iter()
                    .any(|pt| &pt.slug == tmpl_slug && pt.is_compatible(t))
            } else {
                post_templates.iter().any(|pt| pt.is_compatible(t))
            };
            if !pt_compat {
                continue;
            }
            let avail = *remaining.get(&key).unwrap_or(&0);
            let take = avail.min(need);
            if take > 0 {
                *remaining.get_mut(&key).unwrap() -= take;
                need -= take;
            }
        }
        if need > 0 {
            return false;
        }
    }
    true
}

/// Deduct the posts consumed by a row template from the per-(weight,type)
/// pool. Mirrors `can_fill_template`'s greedy matching so the pool stays
/// in sync with the raw heavy/medium/light counts.
fn deduct_from_pool_by_type(
    tws: &RowTemplateWithSlots,
    pool_by_type: &mut HashMap<(String, String), usize>,
    post_templates: &[PostTemplateConfig],
) {
    for slot in &tws.slots {
        let mut need = slot.count as usize;
        let keys: Vec<(String, String)> = pool_by_type.keys().cloned().collect();
        for key in keys {
            if need == 0 {
                break;
            }
            let (w, t) = &key;
            if w != &slot.weight {
                continue;
            }
            if !slot.accepts_type(t) {
                continue;
            }
            let pt_compat = if let Some(ref tmpl_slug) = slot.post_template_slug {
                post_templates
                    .iter()
                    .any(|pt| &pt.slug == tmpl_slug && pt.is_compatible(t))
            } else {
                post_templates.iter().any(|pt| pt.is_compatible(t))
            };
            if !pt_compat {
                continue;
            }
            let avail = *pool_by_type.get(&key).unwrap_or(&0);
            let take = avail.min(need);
            if take > 0 {
                *pool_by_type.get_mut(&key).unwrap() -= take;
                need -= take;
            }
        }
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

    // Post-sort: break up visual monotony ("harmony within, diversity amongst").
    //
    // Two adjacent rows feel monotonous when they share either:
    //   1. The same layout_variant (e.g., two lead-stack rows)
    //   2. The same anchor template (e.g., three rows all led by `feature`)
    //   3. The same row_template_slug (exact same recipe back-to-back)
    //
    // When detected, scan forward for the first row with a different visual
    // signature and swap it in. This interleaves features with gazettes,
    // trios with pairs, etc.
    let len = rows.len();

    // Derive the "visual signature" of a row: (layout_variant, anchor_template)
    let signature = |row: &BroadsheetRow| -> (String, String) {
        let variant = variant_map
            .get(row.row_template_slug.as_str())
            .copied()
            .unwrap_or("")
            .to_string();
        let anchor = row
            .slots
            .first()
            .map(|s| s.post_template_slug.clone())
            .unwrap_or_default();
        (variant, anchor)
    };

    for i in 1..len {
        let prev_sig = signature(&rows[i - 1]);
        let curr_sig = signature(&rows[i]);
        // Monotonous if EITHER variant or anchor template matches
        let monotonous = prev_sig.0 == curr_sig.0 || prev_sig.1 == curr_sig.1;
        if monotonous {
            // Find the first non-monotonous row ahead and swap
            if let Some(swap_idx) = (i + 1..len).find(|&j| {
                let j_sig = signature(&rows[j]);
                j_sig.0 != curr_sig.0 && j_sig.1 != curr_sig.1
                    && j_sig.0 != prev_sig.0 && j_sig.1 != prev_sig.1
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

/// Convert a county name like "Scott" or "Lac qui Parle" to its service_area
/// tag slug: "scott-county" / "lac-qui-parle-county".
fn county_service_area_slug(county_name: &str) -> String {
    let kebab = county_name
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-");
    // Replace other common separators
    let kebab = kebab.replace('.', "").replace(',', "");
    format!("{}-county", kebab)
}

/// Load active posts relevant to a county.
///
/// A post is eligible when any of the following holds:
///   - Its locationable's zip_code maps to this county.
///   - Its direct `posts.zip_code` maps to this county (legacy path).
///   - It has a `service_area` tag matching this county (e.g. `scott-county`).
///   - It has the explicit `statewide` tag.
///   - It has NO location at all AND no `service_area` tag (truly statewide).
///
/// The critical rule: posts with an explicit `service_area` tag are locked
/// to that county — they won't fall through the "no location = statewide"
/// fallback. Without this, county-specific references (e.g. "Scott County
/// Food Shelves") leak into every county's broadsheet.
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

    // Fetch the county's service_area slug so we can match `service_area` tags.
    let county_name: String =
        sqlx::query_scalar("SELECT name FROM counties WHERE id = $1")
            .bind(county_id)
            .fetch_one(pool)
            .await?;
    let service_area = county_service_area_slug(&county_name);

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
            -- Explicit county match via locationable
            zc.county_id = $1
            -- Explicit match via this county's service_area tag
            OR EXISTS (
              SELECT 1 FROM taggables t
              JOIN tags tg ON t.tag_id = tg.id
              WHERE t.taggable_type = 'post'
                AND t.taggable_id = p.id
                AND tg.kind = 'service_area'
                AND tg.value = $3
            )
            -- Explicit statewide tag
            OR EXISTS (
              SELECT 1 FROM taggables t
              JOIN tags tg ON t.tag_id = tg.id
              WHERE t.taggable_type = 'post'
                AND t.taggable_id = p.id
                AND tg.value = 'statewide'
            )
            -- No location at all AND no service_area tag pinning it elsewhere
            OR (
              la.id IS NULL
              AND NOT EXISTS (
                SELECT 1 FROM taggables t
                JOIN tags tg ON t.tag_id = tg.id
                WHERE t.taggable_type = 'post'
                  AND t.taggable_id = p.id
                  AND tg.kind = 'service_area'
              )
            )
          )
          AND (p.published_at IS NULL OR p.published_at >= ($2::date - INTERVAL '7 days'))
        ORDER BY p.priority DESC NULLS LAST
        "#,
    )
    .bind(county_id)
    .bind(period_start)
    .bind(&service_area)
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
