//! Edition-specific data transfer types used by activities and HTTP handlers.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A draft broadsheet produced by the layout engine, before persisting to DB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadsheetDraft {
    pub rows: Vec<BroadsheetRow>,
    pub sections: Vec<BroadsheetSection>,
    /// Widget-standalone rows to interleave with the post rows at persistence time.
    /// Each entry specifies which row index it should be inserted AFTER.
    pub widget_rows: Vec<BroadsheetWidgetRow>,
}

/// A widget row to be inserted into the broadsheet at a specific position.
/// Can hold 1 widget (standalone), 2 (pair), or 3 (trio).
/// Persistence rewrites sort orders so these interleave cleanly with post rows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadsheetWidgetRow {
    /// One or more widget slots in this row.
    /// Single = widget-standalone, 2 = widget-pair, 3 = widget-trio.
    pub widgets: Vec<BroadsheetWidgetSlot>,
    /// Insert this widget row AFTER the row at this index in `BroadsheetDraft.rows`.
    pub insert_after: usize,
    /// The row_template_config_id (widget-standalone, widget-pair, or widget-trio).
    pub row_template_id: Uuid,
}

/// A single widget placement within a widget row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadsheetWidgetSlot {
    pub widget_id: Uuid,
    pub widget_template: Option<String>,
    pub slot_index: i32,
}

/// A topic section in the broadsheet draft (created from Root Signal topic data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadsheetSection {
    pub title: String,
    pub subtitle: Option<String>,
    pub topic_slug: Option<String>,
    /// Indices into BroadsheetDraft.rows that belong to this section.
    pub row_indices: Vec<usize>,
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
    /// Topic slug from Root Signal (via tags with kind='topic').
    pub topic_slug: Option<String>,
}

/// Result of batch edition generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchGenerateResult {
    pub created: i32,
    pub regenerated: i32,
    pub skipped: i32,
    pub failed: i32,
    pub total_counties: i32,
}

/// Kanban stats: counts of editions by status for a given period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditionKanbanStats {
    pub draft: i32,
    pub in_review: i32,
    pub approved: i32,
    pub published: i32,
}
