//! Upcoming events activity - computes next occurrences from schedules

use anyhow::Result;
use std::collections::HashMap;
use uuid::Uuid;

use crate::domains::posts::data::types::BusinessInfo;
use crate::domains::posts::data::PostType;
use crate::domains::posts::models::{BusinessPost, Post};
use crate::kernel::ServerDeps;

/// Get upcoming events: posts tagged `post_type: event` with schedules,
/// ordered by next occurrence (computed from rrule).
pub async fn get_upcoming_events(limit: usize, deps: &ServerDeps) -> Result<Vec<PostType>> {
    let pool = &deps.db_pool;

    // Load all schedules attached to event-tagged posts
    let schedules = Post::find_event_schedules(pool).await?;

    // Group by post, find earliest next occurrence per post
    let mut post_next: HashMap<Uuid, chrono::DateTime<chrono::Utc>> = HashMap::new();

    for schedule in &schedules {
        if let Some(next) = schedule.next_occurrences(1).into_iter().next() {
            let entry = post_next.entry(schedule.schedulable_id).or_insert(next);
            if next < *entry {
                *entry = next;
            }
        }
    }

    // Sort by next occurrence
    let mut sorted: Vec<_> = post_next.into_iter().collect();
    sorted.sort_by_key(|(_, next)| *next);
    sorted.truncate(limit);

    // Batch-load all posts in one query
    let post_ids: Vec<Uuid> = sorted.iter().map(|(id, _)| *id).collect();
    let loaded = Post::find_by_ids(&post_ids, pool).await?;

    // Index by ID to preserve sort order
    let post_map: HashMap<Uuid, Post> = loaded.into_iter().map(|p| (p.id.into_uuid(), p)).collect();

    // Batch-load business info for business-type posts
    let business_post_ids: Vec<Uuid> = post_map
        .values()
        .filter(|p| p.post_type == "business")
        .map(|p| p.id.into_uuid())
        .collect();
    let business_map: HashMap<Uuid, BusinessPost> = if business_post_ids.is_empty() {
        HashMap::new()
    } else {
        BusinessPost::find_by_post_ids(&business_post_ids, pool)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|b| (b.post_id.into_uuid(), b))
            .collect()
    };

    // Build PostType vec in sorted order
    let posts: Vec<PostType> = sorted
        .iter()
        .filter_map(|(id, _)| {
            post_map.get(id).map(|post| {
                let mut pt = PostType::from(post.clone());
                if let Some(business) = business_map.get(id) {
                    pt.business_info = Some(BusinessInfo {
                        accepts_donations: business.accepts_donations,
                        donation_link: business.donation_link.clone(),
                        gift_cards_available: business.gift_cards_available,
                        gift_card_link: business.gift_card_link.clone(),
                        online_ordering_link: business.online_ordering_link.clone(),
                        delivery_available: business.delivery_available,
                        proceeds_percentage: business.proceeds_percentage,
                        proceeds_beneficiary_id: business.proceeds_beneficiary_id,
                        proceeds_description: business.proceeds_description.clone(),
                        impact_statement: business.impact_statement.clone(),
                    });
                }
                pt
            })
        })
        .collect();

    Ok(posts)
}
