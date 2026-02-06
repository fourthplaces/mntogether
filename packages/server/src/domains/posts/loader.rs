use dataloader::BatchFn;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::domains::posts::models::{BusinessPost, Post};

/// Batches Post lookups by ID.
pub struct PostLoader {
    pub db: Arc<PgPool>,
}

impl PostLoader {
    pub fn new(db: Arc<PgPool>) -> Self {
        Self { db }
    }
}

impl BatchFn<Uuid, Option<Post>> for PostLoader {
    fn load(
        &mut self,
        keys: &[Uuid],
    ) -> impl std::future::Future<Output = HashMap<Uuid, Option<Post>>> {
        let db = self.db.clone();
        let keys = keys.to_vec();
        async move {
            let posts = Post::find_by_ids(&keys, db.as_ref())
                .await
                .unwrap_or_default();
            let mut map: HashMap<Uuid, Option<Post>> = posts
                .into_iter()
                .map(|p| (p.id.into_uuid(), Some(p)))
                .collect();
            for id in &keys {
                map.entry(*id).or_insert(None);
            }
            map
        }
    }
}

/// Batches BusinessPost lookups by post ID.
pub struct PostBusinessInfoLoader {
    pub db: Arc<PgPool>,
}

impl PostBusinessInfoLoader {
    pub fn new(db: Arc<PgPool>) -> Self {
        Self { db }
    }
}

impl BatchFn<Uuid, Option<BusinessPost>> for PostBusinessInfoLoader {
    fn load(
        &mut self,
        keys: &[Uuid],
    ) -> impl std::future::Future<Output = HashMap<Uuid, Option<BusinessPost>>> {
        let db = self.db.clone();
        let keys = keys.to_vec();
        async move {
            let businesses = BusinessPost::find_by_post_ids(&keys, db.as_ref())
                .await
                .unwrap_or_default();
            let mut map: HashMap<Uuid, Option<BusinessPost>> = businesses
                .into_iter()
                .map(|b| (b.post_id.into_uuid(), Some(b)))
                .collect();
            for id in &keys {
                map.entry(*id).or_insert(None);
            }
            map
        }
    }
}
