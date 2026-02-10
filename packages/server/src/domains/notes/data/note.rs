use crate::domains::notes::models::Note;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteData {
    pub id: String,
    pub content: String,
    pub severity: String,
    pub source_url: Option<String>,
    pub source_id: Option<String>,
    pub source_type: Option<String>,
    pub is_public: bool,
    pub created_by: String,
    pub expired_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Note> for NoteData {
    fn from(note: Note) -> Self {
        Self {
            id: note.id.to_string(),
            content: note.content,
            severity: note.severity,
            source_url: note.source_url,
            source_id: note.source_id.map(|id| id.to_string()),
            source_type: note.source_type,
            is_public: note.is_public,
            created_by: note.created_by,
            expired_at: note.expired_at.map(|t| t.to_rfc3339()),
            created_at: note.created_at.to_rfc3339(),
            updated_at: note.updated_at.to_rfc3339(),
        }
    }
}
