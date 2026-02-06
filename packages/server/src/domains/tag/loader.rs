use dataloader::BatchFn;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::domains::tag::models::Tag;

/// Batches tag lookups by post ID.
pub struct PostTagsLoader {
    pub db: Arc<PgPool>,
}

impl PostTagsLoader {
    pub fn new(db: Arc<PgPool>) -> Self {
        Self { db }
    }
}

impl BatchFn<Uuid, Vec<Tag>> for PostTagsLoader {
    fn load(
        &mut self,
        keys: &[Uuid],
    ) -> impl std::future::Future<Output = HashMap<Uuid, Vec<Tag>>> {
        let db = self.db.clone();
        let keys = keys.to_vec();
        async move {
            let fetched = Tag::find_for_post_ids(&keys, db.as_ref())
                .await
                .unwrap_or_default();
            let mut tags_by_post: HashMap<Uuid, Vec<Tag>> = HashMap::new();
            for row in fetched {
                tags_by_post
                    .entry(row.taggable_id)
                    .or_default()
                    .push(row.tag);
            }
            // Ensure every requested key has an entry
            for id in &keys {
                tags_by_post.entry(*id).or_default();
            }
            tags_by_post
        }
    }
}
