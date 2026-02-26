//! Edition-specific data transfer types used by activities and Restate services.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A draft broadsheet produced by the layout engine, before persisting to DB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadsheetDraft {
    pub rows: Vec<BroadsheetRow>,
}

/// A single row in the broadsheet draft.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadsheetRow {
    pub row_template_slug: String,
    pub row_template_id: Uuid,
    pub slots: Vec<BroadsheetSlot>,
    /// Priority of highest-priority post in this row (for ordering).
    pub max_priority: i32,
}

/// A post placement within a broadsheet row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadsheetSlot {
    pub post_id: Uuid,
    pub post_template_slug: String,
    pub slot_index: i32,
}

/// A lightweight post representation used by the layout engine.
/// Contains only the fields needed for placement decisions.
#[derive(Debug, Clone)]
pub struct LayoutPost {
    pub id: Uuid,
    pub post_type: String,
    pub weight: String,
    pub priority: i32,
}

/// Result of batch edition generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchGenerateResult {
    pub created: i32,
    pub failed: i32,
    pub total_counties: i32,
}
