use dataloader::non_cached::Loader;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::domains::schedules::loader::PostSchedulesLoader;
use crate::domains::schedules::models::Schedule;
use crate::domains::posts::loader::{PostBusinessInfoLoader, PostLoader};
use crate::domains::posts::models::{BusinessPost, Post};
use crate::domains::tag::loader::PostTagsLoader;
use crate::domains::tag::models::Tag;

pub struct DataLoaders {
    pub post: Loader<Uuid, Option<Post>, PostLoader>,
    pub business_info: Loader<Uuid, Option<BusinessPost>, PostBusinessInfoLoader>,
    pub post_tags: Loader<Uuid, Vec<Tag>, PostTagsLoader>,
    pub post_schedules: Loader<Uuid, Vec<Schedule>, PostSchedulesLoader>,
}

impl DataLoaders {
    pub fn new(db: Arc<PgPool>) -> Self {
        Self {
            post: Loader::new(PostLoader::new(db.clone())),
            business_info: Loader::new(PostBusinessInfoLoader::new(db.clone())),
            post_tags: Loader::new(PostTagsLoader::new(db.clone())),
            post_schedules: Loader::new(PostSchedulesLoader::new(db)),
        }
    }
}
