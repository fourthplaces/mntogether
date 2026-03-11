//! Upcoming events activity - computes next occurrences from schedules

use anyhow::Result;
use std::collections::HashMap;
use uuid::Uuid;

use crate::domains::posts::data::PostType;
use crate::domains::posts::models::Post;
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

    // Build PostType vec in sorted order
    let posts: Vec<PostType> = sorted
        .iter()
        .filter_map(|(id, _)| post_map.get(id).map(|post| PostType::from(post.clone())))
        .collect();

    Ok(posts)
}
