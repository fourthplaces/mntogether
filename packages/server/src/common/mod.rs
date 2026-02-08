// Common types and utilities shared across the application

pub mod auth;
pub mod embedding;
pub mod entity_ids;
pub mod extraction_types;
pub mod id;
pub mod nats;
pub mod nats_tap;
pub mod pagination;
pub mod pii;
pub mod read_result;
pub mod readable;
pub mod restate_serde;
pub mod restate_types;
pub mod types;
pub mod utils;

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
pub use restate_types::EmptyRequest;
pub use types::*;

// Unified extraction types - use these instead of domain-specific definitions
pub use extraction_types::{
    CallToAction, ContactInfo, DayHours, EligibilityInfo, ExtractionType, LocationInfo,
    ScheduleInfo,
};
