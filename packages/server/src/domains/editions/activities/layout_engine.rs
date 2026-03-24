//! Layout engine — the "deliberately dumb" placement algorithm from CMS_SYSTEM_SPEC.md §7.
//!
//! Takes a county_id and date range, finds eligible posts, then greedily assigns them
//! to row template slots based on weight matching and priority ordering.

use std::collections::HashMap;

use anyhow::Result;
use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::editions::data::types::{
    BroadsheetDraft, BroadsheetRow, BroadsheetSection, BroadsheetSlot, LayoutPost,
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
    period_start: NaiveDate,
    _period_end: NaiveDate,
    deps: &ServerDeps,
) -> Result<BroadsheetDraft> {
    let pool = &deps.db_pool;

    // Step 1: Load eligible posts for this county within the edition's time window
    let posts = load_county_posts(county_id, period_start, pool).await?;

    // Step 2: Already sorted by priority DESC in the query
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

    // Step 3: Load config
    let templates = RowTemplateConfig::find_all_with_slots(pool).await?;
    let post_templates = PostTemplateConfig::find_all(pool).await?;

    tracing::info!(
        row_templates = templates.len(),
        post_templates = post_templates.len(),
        "Layout engine: loaded template configs"
    );

    // Steps 4-5: Place posts into rows
    let draft = place_posts(posts, &templates, &post_templates);

    tracing::info!(
        rows = draft.rows.len(),
        total_slots = draft.rows.iter().map(|r| r.slots.len()).sum::<usize>(),
        sections = draft.sections.len(),
        "Layout engine: placement complete"
    );

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
    period_start: NaiveDate,
    pool: &PgPool,
) -> Result<Vec<LayoutPost>> {
    // Lightweight struct for layout engine — only needs id, type, weight, priority, topic
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

    // Load topic tags for all posts in one query
    let post_ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let topic_tags = load_topic_tags(&post_ids, pool).await?;

    let posts = rows
        .into_iter()
        .map(|r| {
            let topic_slug = topic_tags.get(&r.id).cloned();
            LayoutPost {
                id: r.id,
                post_type: r.post_type.unwrap_or_else(|| "notice".to_string()),
                weight: r.weight.unwrap_or_else(|| "medium".to_string()),
                priority: r.priority.unwrap_or(50),
                topic_slug,
            }
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
        return BroadsheetDraft { rows: vec![], sections: vec![] };
    }

    // Count weight distribution
    let heavy_count = posts.iter().filter(|p| p.weight == "heavy").count();
    let medium_count = posts.iter().filter(|p| p.weight == "medium").count();
    let light_count = posts.iter().filter(|p| p.weight == "light").count();

    // Select row templates to match weight distribution
    let selected_rows = select_row_templates(heavy_count, medium_count, light_count, templates);

    tracing::info!(
        selected = selected_rows.len(),
        templates = ?selected_rows.iter().map(|(cfg, _)| &cfg.slug).collect::<Vec<_>>(),
        "Layout engine: selected row templates"
    );

    // Track which posts have been placed
    let mut placed: Vec<bool> = vec![false; posts.len()];
    let mut broadsheet_rows: Vec<BroadsheetRow> = Vec::new();

    // Place posts into row slots — slot_index matches the template slot definition
    // so the admin UI can group edition slots by their template slot correctly.
    for (template, template_with_slots) in &selected_rows {
        let mut row_slots: Vec<BroadsheetSlot> = Vec::new();
        let mut row_max_priority: i32 = 0;

        for slot_def in &template_with_slots.slots {
            // Fill this slot group with `count` posts of matching weight.
            // All posts in this group share the same slot_index (the template slot's index).
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

                // Use the slot's recipe template if set and compatible, else fall back
                let pt_slug = slot_def
                    .post_template_slug
                    .as_ref()
                    .filter(|slug| {
                        // Verify the recipe's template is compatible with this post type
                        post_templates
                            .iter()
                            .any(|pt| &pt.slug == *slug && pt.is_compatible(&post.post_type))
                    })
                    .cloned()
                    .or_else(|| {
                        find_compatible_post_template(post, &slot_def.weight, post_templates)
                    });

                if let Some(pt_slug) = pt_slug {
                    row_slots.push(BroadsheetSlot {
                        post_id: post.id,
                        post_template_slug: pt_slug,
                        slot_index: slot_def.slot_index,
                    });
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
            let expected_slots: i32 = template_with_slots.slots.iter().map(|s| s.count).sum();
            tracing::debug!(
                template = %template.slug,
                filled = row_slots.len(),
                expected = expected_slots,
                "Layout engine: filled row"
            );
            broadsheet_rows.push(BroadsheetRow {
                row_template_slug: template.slug.clone(),
                row_template_id: template.id,
                slots: row_slots,
                max_priority: row_max_priority,
            });
        } else {
            tracing::debug!(
                template = %template.slug,
                "Layout engine: skipped empty row (no posts matched)"
            );
        }
    }

    // Sort rows by highest-priority post (descending)
    broadsheet_rows.sort_by(|a, b| b.max_priority.cmp(&a.max_priority));

    let total_placed = placed.iter().filter(|&&p| p).count();
    tracing::info!(
        placed = total_placed,
        unplaced = posts.len() - total_placed,
        rows = broadsheet_rows.len(),
        "Layout engine: slot filling summary"
    );

    // Build sections from topic slugs on placed posts
    let sections = build_topic_sections(&broadsheet_rows, &posts);

    BroadsheetDraft {
        rows: broadsheet_rows,
        sections,
    }
}

/// Select row templates to accommodate the weight distribution of posts.
///
/// Strategy: allocate rows proportionally across weight tiers.
/// 1. Reserve 2-3 rows for hero/lead templates (heavy posts)
/// 2. Fill remaining rows with medium-only and light-only templates
/// 3. Avoid repeating the same template consecutively
fn select_row_templates<'a>(
    mut heavy: usize,
    mut medium: usize,
    mut light: usize,
    templates: &'a [RowTemplateWithSlots],
) -> Vec<(&'a RowTemplateConfig, &'a RowTemplateWithSlots)> {
    let mut selected: Vec<(&RowTemplateConfig, &RowTemplateWithSlots)> = Vec::new();
    let max_rows = 12;

    // Track how many times each layout_variant has been used
    let mut variant_counts: HashMap<&str, usize> = HashMap::new();

    // Phase 1: Select hero/lead rows (templates using heavy slots).
    // Cap at 3 to leave room for medium/light rows.
    let max_hero_rows = 3.min(heavy);
    for _ in 0..max_hero_rows {
        if heavy == 0 {
            break;
        }

        let last_slug = selected.last().map(|(cfg, _)| cfg.slug.as_str());
        let mut best: Option<(&RowTemplateWithSlots, usize)> = None;

        for tws in templates {
            // Only consider templates with heavy slots
            if !tws.slots.iter().any(|s| s.weight == "heavy") {
                continue;
            }

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

            // Avoid repeating the same slug
            if let Some(last) = last_slug {
                if tws.config.slug == last {
                    continue;
                }
            }

            if best.is_none() || score > best.unwrap().1 {
                best = Some((tws, score));
            }
        }

        if let Some((tws, _)) = best {
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
            *variant_counts
                .entry(tws.config.layout_variant.as_str())
                .or_insert(0) += 1;
            selected.push((&tws.config, tws));
        } else {
            break;
        }
    }

    // Phase 2: Fill remaining rows with medium/light templates.
    // Prefer variety in layout_variant.
    let remaining_rows = max_rows - selected.len();
    for _ in 0..remaining_rows {
        if medium == 0 && light == 0 {
            break;
        }

        let last_slug = selected.last().map(|(cfg, _)| cfg.slug.as_str());
        let mut best: Option<(&RowTemplateWithSlots, f64)> = None;

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

            if score == 0 {
                continue;
            }

            // Skip heavy-only templates in phase 2 if we already have hero rows
            let needs_heavy = tws.slots.iter().any(|s| s.weight == "heavy");
            if needs_heavy && !selected.is_empty() {
                continue;
            }

            let mut adj_score = score as f64;

            // Avoid repeating the same slug consecutively
            if let Some(last) = last_slug {
                if tws.config.slug == last {
                    adj_score *= 0.4;
                }
            }

            // Penalize overused layout variants (max 2 of any variant)
            let variant_count = variant_counts
                .get(tws.config.layout_variant.as_str())
                .copied()
                .unwrap_or(0);
            if variant_count >= 2 {
                adj_score *= 0.3;
            }

            // Boost templates not yet used in this edition
            let already_selected = selected.iter().any(|(cfg, _)| cfg.slug == tws.config.slug);
            if !already_selected {
                adj_score *= 1.5;
            }

            if best.is_none() || adj_score > best.unwrap().1 {
                best = Some((tws, adj_score));
            }
        }

        if let Some((tws, _)) = best {
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
            *variant_counts
                .entry(tws.config.layout_variant.as_str())
                .or_insert(0) += 1;
            selected.push((&tws.config, tws));
        } else {
            break;
        }
    }

    selected
}

/// Find the best compatible post template for a post, based on slot weight.
///
/// Uses the `weight` column from `post_template_configs` to match templates
/// to slot weights. Falls back to any compatible template if no weight-matched
/// template is found.
fn find_compatible_post_template(
    post: &LayoutPost,
    slot_weight: &str,
    post_templates: &[PostTemplateConfig],
) -> Option<String> {
    // First pass: find templates whose DB weight matches the slot weight AND are type-compatible
    for pt in post_templates {
        if pt.weight == slot_weight && pt.is_compatible(&post.post_type) {
            return Some(pt.slug.clone());
        }
    }

    // Fallback: any compatible template regardless of weight
    for pt in post_templates {
        if pt.is_compatible(&post.post_type) {
            return Some(pt.slug.clone());
        }
    }

    None
}

/// Load topic tags for a batch of post IDs.
/// Returns a map of post_id → topic_slug for posts that have a topic tag (kind='topic').
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
        // If a post has multiple topic tags, take the first one
        map.entry(row.post_id).or_insert(row.topic_slug);
    }

    Ok(map)
}

/// Build topic sections from the placed rows.
///
/// For each row, determine its dominant topic (most common topic among its posts).
/// Group consecutive rows with the same topic into sections. Rows without topics
/// are left ungrouped (above the fold).
fn build_topic_sections(
    rows: &[BroadsheetRow],
    posts: &[LayoutPost],
) -> Vec<BroadsheetSection> {
    // Build post_id → topic lookup
    let topic_by_id: HashMap<Uuid, &str> = posts
        .iter()
        .filter_map(|p| p.topic_slug.as_ref().map(|t| (p.id, t.as_str())))
        .collect();

    // For each row, find the dominant topic
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

    // Group rows by topic into sections (skip ungrouped rows)
    let mut sections: Vec<BroadsheetSection> = Vec::new();
    let mut current_topic: Option<&str> = None;

    for (i, topic) in row_topics.iter().enumerate() {
        match topic {
            Some(t) => {
                if current_topic == Some(*t) {
                    // Extend current section
                    if let Some(section) = sections.last_mut() {
                        section.row_indices.push(i);
                    }
                } else {
                    // Start new section
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
        // Ungrouped row between two topic rows → breaks the section continuity
        let p1 = make_post(Some("housing"));
        let p2 = make_post(None);
        let p3 = make_post(Some("housing"));
        let rows = vec![make_row(&[&p1]), make_row(&[&p2]), make_row(&[&p3])];
        let posts = vec![p1, p2, p3];

        let sections = build_topic_sections(&rows, &posts);
        // Should create two separate housing sections (interrupted by ungrouped row)
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].row_indices, vec![0]);
        assert_eq!(sections[1].row_indices, vec![2]);
    }

    #[test]
    fn build_sections_dominant_topic_in_mixed_row() {
        // Row with 2 housing posts and 1 food post → housing dominates
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
