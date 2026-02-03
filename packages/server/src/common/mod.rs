// Common types and utilities shared across the application

pub mod app_state;
pub mod auth;
pub mod embedding;
pub mod entity_ids;
pub mod id;
pub mod nats;
pub mod nats_tap;
pub mod pagination;
pub mod pii;
pub mod read_result;
pub mod readable;
pub mod types;
pub mod utils;

pub use app_state::AppState;
pub use auth::{Actor, AdminCapability, AuthError, HasAuthContext};
pub use embedding::Embeddable;
pub use entity_ids::*;
pub use id::{Id, V4, V7};
pub use nats::IntoNatsPayload;
pub use pagination::{
    build_page_info, trim_results, Cursor, PageInfo, PaginationArgs, PaginationDirection,
    ValidatedPaginationArgs,
};
pub use read_result::ReadResult;
pub use readable::Readable;
pub use types::*;
