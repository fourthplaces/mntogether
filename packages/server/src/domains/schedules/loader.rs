use dataloader::BatchFn;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::domains::schedules::models::Schedule;

/// Batches schedule lookups by post ID.
pub struct PostSchedulesLoader {
    pub db: Arc<PgPool>,
}

impl PostSchedulesLoader {
    pub fn new(db: Arc<PgPool>) -> Self {
        Self { db }
    }
}

impl BatchFn<Uuid, Vec<Schedule>> for PostSchedulesLoader {
    fn load(
        &mut self,
        keys: &[Uuid],
    ) -> impl std::future::Future<Output = HashMap<Uuid, Vec<Schedule>>> {
        let db = self.db.clone();
        let keys = keys.to_vec();
        async move {
            let fetched = Schedule::find_for_post_ids(&keys, db.as_ref())
                .await
                .unwrap_or_default();
            let mut schedules_by_post: HashMap<Uuid, Vec<Schedule>> = HashMap::new();
            for schedule in fetched {
                schedules_by_post
                    .entry(schedule.schedulable_id)
                    .or_default()
                    .push(schedule);
            }
            // Ensure every requested key has an entry
            for id in &keys {
                schedules_by_post.entry(*id).or_default();
            }
            schedules_by_post
        }
    }
}
