//! Layout engine — the "deliberately dumb" placement algorithm from CMS_SYSTEM_SPEC.md §7.
//!
//! Takes a county_id and date range, finds eligible posts, then greedily assigns them
//! to row template slots based on weight matching and priority ordering.

use anyhow::Result;
use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::editions::data::types::{
    BroadsheetDraft, BroadsheetRow, BroadsheetSlot, LayoutPost,
};
use crate::domains::editions::models::post_template_config::PostTemplateConfig;
use crate::domains::editions::models::row_template_config::RowTemplateWithSlots;
use crate::domains::editions::models::row_template_config::RowTemplateConfig;
use crate::kernel::ServerDeps;

/// Generate a broadsheet draft for a county and date range.
///
/// Algorithm (from CMS_SYSTEM_SPEC.md §7):
/// 1. Load eligible posts for the county (via zip→county + statewide)
/// 2. Sort by priority (descending)
/// 3. Load available row templates + post templates
/// 4. Greedily assign posts to slots (weight match + type compatibility)
/// 5. Order rows by highest-priority post in each row
/// 6. Return the broadsheet draft
pub async fn generate_broadsheet(
    county_id: Uuid,
    _period_start: NaiveDate,
    _period_end: NaiveDate,
    deps: &ServerDeps,
) -> Result<BroadsheetDraft> {
    let pool = &deps.db_pool;

    // Step 1: Load eligible posts for this county
    // Note: period filtering will be added when posts gain published_at-based windowing.
    // For now, all active posts are eligible.
    let posts = load_county_posts(county_id, pool).await?;

    // Step 2: Already sorted by priority DESC in the query

    // Step 3: Load config
    let templates = RowTemplateConfig::find_all_with_slots(pool).await?;
    let post_templates = PostTemplateConfig::find_all(pool).await?;

    // Steps 4-5: Place posts into rows
    let draft = place_posts(posts, &templates, &post_templates);

    Ok(draft)
}

/// Load active posts relevant to a county.
///
/// A post is relevant if:
/// - Its primary location's postal_code maps to this county via zip_counties
///   (join path: posts → locationables → locations → zip_counties)
/// - It has no location (treated as statewide)
/// - It is tagged 'statewide'
async fn load_county_posts(
    county_id: Uuid,
    pool: &PgPool,
) -> Result<Vec<LayoutPost>> {
    // Lightweight struct for layout engine — only needs id, type, weight, priority
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
        ORDER BY p.priority DESC NULLS LAST
        "#,
    )
    .bind(county_id)
    .fetch_all(pool)
    .await?;

    let posts = rows
        .into_iter()
        .map(|r| LayoutPost {
            id: r.id,
            post_type: r.post_type.unwrap_or_else(|| "notice".to_string()),
            weight: r.weight.unwrap_or_else(|| "medium".to_string()),
            priority: r.priority.unwrap_or(50),
        })
        .collect();

    Ok(posts)
}

/// The core placement algorithm. Pure function — no I/O.
///
/// Strategy:
/// 1. Count weight distribution of available posts
/// 2. Select row templates to match the distribution
/// 3. Greedily fill slots with highest-priority posts
/// 4. Sort rows by highest-priority post in each
fn place_posts(
    posts: Vec<LayoutPost>,
    templates: &[RowTemplateWithSlots],
    post_templates: &[PostTemplateConfig],
) -> BroadsheetDraft {
    if posts.is_empty() {
        return BroadsheetDraft { rows: vec![] };
    }

    // Count weight distribution
    let heavy_count = posts.iter().filter(|p| p.weight == "heavy").count();
    let medium_count = posts.iter().filter(|p| p.weight == "medium").count();
    let light_count = posts.iter().filter(|p| p.weight == "light").count();

    // Select row templates to match weight distribution
    let selected_rows = select_row_templates(heavy_count, medium_count, light_count, templates);

    // Track which posts have been placed
    let mut placed: Vec<bool> = vec![false; posts.len()];
    let mut broadsheet_rows: Vec<BroadsheetRow> = Vec::new();

    // Global slot counter — each post gets a unique slot_index within its row
    for (template, template_with_slots) in &selected_rows {
        let mut row_slots: Vec<BroadsheetSlot> = Vec::new();
        let mut row_max_priority: i32 = 0;
        let mut next_slot_index: i32 = 0;

        for slot_def in &template_with_slots.slots {
            // Fill this slot group with `count` posts of matching weight
            let mut filled = 0;
            for (i, post) in posts.iter().enumerate() {
                if filled >= slot_def.count {
                    break;
                }
                if placed[i] {
                    continue;
                }
                if post.weight != slot_def.weight {
                    continue;
                }
                if !slot_def.accepts_type(&post.post_type) {
                    continue;
                }

                // Find a compatible post template
                if let Some(pt_slug) = find_compatible_post_template(post, post_templates) {
                    row_slots.push(BroadsheetSlot {
                        post_id: post.id,
                        post_template_slug: pt_slug,
                        slot_index: next_slot_index,
                    });
                    next_slot_index += 1;
                    placed[i] = true;
                    filled += 1;
                    if post.priority > row_max_priority {
                        row_max_priority = post.priority;
                    }
                }
            }
        }

        // Only add the row if at least one slot was filled
        if !row_slots.is_empty() {
            broadsheet_rows.push(BroadsheetRow {
                row_template_slug: template.slug.clone(),
                row_template_id: template.id,
                slots: row_slots,
                max_priority: row_max_priority,
            });
        }
    }

    // Sort rows by highest-priority post (descending)
    broadsheet_rows.sort_by(|a, b| b.max_priority.cmp(&a.max_priority));

    BroadsheetDraft {
        rows: broadsheet_rows,
    }
}

/// Select row templates to accommodate the weight distribution of posts.
///
/// Greedy heuristic: pick templates that consume the most remaining posts.
fn select_row_templates<'a>(
    mut heavy: usize,
    mut medium: usize,
    mut light: usize,
    templates: &'a [RowTemplateWithSlots],
) -> Vec<(&'a RowTemplateConfig, &'a RowTemplateWithSlots)> {
    let mut selected: Vec<(&RowTemplateConfig, &RowTemplateWithSlots)> = Vec::new();

    // Keep selecting templates while we have posts to place
    let max_rows = 12; // reasonable max rows per broadsheet
    for _ in 0..max_rows {
        if heavy == 0 && medium == 0 && light == 0 {
            break;
        }

        // Score each template by how many posts it would consume
        let mut best: Option<(&RowTemplateWithSlots, usize)> = None;

        for tws in templates {
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

            if score > 0 {
                if best.is_none() || score > best.unwrap().1 {
                    best = Some((tws, score));
                }
            }
        }

        match best {
            Some((tws, _)) => {
                // Consume the posts this template uses
                for slot in &tws.slots {
                    let available = match slot.weight.as_str() {
                        "heavy" => &mut heavy,
                        "medium" => &mut medium,
                        "light" => &mut light,
                        _ => continue,
                    };
                    let consume = (slot.count as usize).min(*available);
                    *available -= consume;
                }
                selected.push((&tws.config, tws));
            }
            None => break, // No template can consume any remaining posts
        }
    }

    selected
}

/// Find the first compatible post template for a post.
fn find_compatible_post_template(
    post: &LayoutPost,
    post_templates: &[PostTemplateConfig],
) -> Option<String> {
    // Prefer 'gazette' as the default template (most versatile)
    // Then fall back to any compatible template
    for pt in post_templates {
        if pt.slug == "gazette" && pt.is_compatible(&post.post_type) {
            return Some(pt.slug.clone());
        }
    }
    for pt in post_templates {
        if pt.is_compatible(&post.post_type) {
            return Some(pt.slug.clone());
        }
    }
    None
}
