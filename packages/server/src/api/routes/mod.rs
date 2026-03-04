pub mod auth;
pub mod editions;
pub mod heat_map;
pub mod media;
pub mod member_object;
pub mod members;
pub mod notes;
pub mod organizations;
pub mod posts;
pub mod tags;

use axum::Router;

use super::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(auth::router())
        .merge(editions::router())
        .merge(heat_map::router())
        .merge(media::router())
        .merge(member_object::router())
        .merge(members::router())
        .merge(notes::router())
        .merge(organizations::router())
        .merge(posts::router())
        .merge(tags::router())
}
