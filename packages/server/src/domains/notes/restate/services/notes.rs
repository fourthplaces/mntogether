use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use tracing::warn;

use crate::common::auth::restate_auth::require_admin;
use crate::common::{EmptyRequest, NoteId, OrganizationId};
use crate::domains::notes::activities;
use crate::domains::notes::models::{Note, Noteable};
use crate::domains::organization::models::Organization;
use crate::impl_restate_serde;
use crate::kernel::ServerDeps;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNoteRequest {
    pub content: String,
    pub severity: Option<String>,
    pub source_url: Option<String>,
    pub source_id: Option<Uuid>,
    pub source_type: Option<String>,
    pub is_public: Option<bool>,
    /// Optionally link to an entity on creation.
    pub noteable_type: Option<String>,
    pub noteable_id: Option<Uuid>,
}

impl_restate_serde!(CreateNoteRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetNoteRequest {
    pub id: Uuid,
}

impl_restate_serde!(GetNoteRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNoteRequest {
    pub id: Uuid,
    pub content: String,
    pub severity: String,
    pub is_public: bool,
}

impl_restate_serde!(UpdateNoteRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteNoteRequest {
    pub id: Uuid,
}

impl_restate_serde!(DeleteNoteRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListNotesForEntityRequest {
    pub noteable_type: String,
    pub noteable_id: Uuid,
}

impl_restate_serde!(ListNotesForEntityRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkNoteRequest {
    pub note_id: Uuid,
    pub noteable_type: String,
    pub noteable_id: Uuid,
}

impl_restate_serde!(LinkNoteRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlinkNoteRequest {
    pub note_id: Uuid,
    pub noteable_type: String,
    pub noteable_id: Uuid,
}

impl_restate_serde!(UnlinkNoteRequest);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateNotesRequest {
    pub organization_id: Uuid,
}

impl_restate_serde!(GenerateNotesRequest);

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedPostResult {
    pub id: String,
    pub title: String,
}

impl_restate_serde!(LinkedPostResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteResult {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_posts: Option<Vec<LinkedPostResult>>,
}

impl_restate_serde!(NoteResult);

impl From<Note> for NoteResult {
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
            linked_posts: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteListResult {
    pub notes: Vec<NoteResult>,
}

impl_restate_serde!(NoteListResult);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateNotesResult {
    pub notes_created: i32,
    pub sources_scanned: i32,
    pub posts_attached: i32,
}

impl_restate_serde!(GenerateNotesResult);

// =============================================================================
// Service definition
// =============================================================================

#[restate_sdk::service]
#[name = "Notes"]
pub trait NotesService {
    async fn create(req: CreateNoteRequest) -> Result<NoteResult, HandlerError>;
    async fn get(req: GetNoteRequest) -> Result<NoteResult, HandlerError>;
    async fn update(req: UpdateNoteRequest) -> Result<NoteResult, HandlerError>;
    async fn delete(req: DeleteNoteRequest) -> Result<EmptyRequest, HandlerError>;
    async fn list_for_entity(
        req: ListNotesForEntityRequest,
    ) -> Result<NoteListResult, HandlerError>;
    async fn link(req: LinkNoteRequest) -> Result<NoteResult, HandlerError>;
    async fn unlink(req: UnlinkNoteRequest) -> Result<EmptyRequest, HandlerError>;
    async fn generate_notes(
        req: GenerateNotesRequest,
    ) -> Result<GenerateNotesResult, HandlerError>;
}

pub struct NotesServiceImpl {
    deps: Arc<ServerDeps>,
}

impl NotesServiceImpl {
    pub fn with_deps(deps: Arc<ServerDeps>) -> Self {
        Self { deps }
    }
}

impl NotesService for NotesServiceImpl {
    async fn create(
        &self,
        ctx: Context<'_>,
        req: CreateNoteRequest,
    ) -> Result<NoteResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let note = Note::create(
            &req.content,
            req.severity.as_deref().unwrap_or("info"),
            req.source_url.as_deref(),
            req.source_id,
            req.source_type.as_deref(),
            req.is_public.unwrap_or(false),
            "admin",
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        // Generate embedding for semantic matching against posts
        if let Ok(emb) = self.deps.embedding_service.generate(&req.content).await {
            let _ = Note::update_embedding(note.id, &emb, &self.deps.db_pool).await;
        }

        // If entity linking info provided, link immediately
        if let (Some(noteable_type), Some(noteable_id)) = (&req.noteable_type, &req.noteable_id) {
            Noteable::create(note.id, noteable_type, *noteable_id, &self.deps.db_pool)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;
        }

        Ok(NoteResult::from(note))
    }

    async fn get(
        &self,
        ctx: Context<'_>,
        req: GetNoteRequest,
    ) -> Result<NoteResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let note = Note::find_by_id(NoteId::from(req.id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(NoteResult::from(note))
    }

    async fn update(
        &self,
        ctx: Context<'_>,
        req: UpdateNoteRequest,
    ) -> Result<NoteResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let note = Note::update(
            NoteId::from(req.id),
            &req.content,
            &req.severity,
            req.is_public,
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(NoteResult::from(note))
    }

    async fn delete(
        &self,
        ctx: Context<'_>,
        req: DeleteNoteRequest,
    ) -> Result<EmptyRequest, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        Note::delete(NoteId::from(req.id), &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(EmptyRequest {})
    }

    async fn list_for_entity(
        &self,
        ctx: Context<'_>,
        req: ListNotesForEntityRequest,
    ) -> Result<NoteListResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let notes = Note::find_for_entity(
            &req.noteable_type,
            req.noteable_id,
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        // Batch-fetch linked posts for all notes
        let note_ids: Vec<_> = notes.iter().map(|n| n.id).collect();
        let linked_posts = Noteable::find_linked_posts_for_notes(&note_ids, &self.deps.db_pool)
            .await
            .unwrap_or_default();

        // Group linked posts by note_id
        let mut posts_by_note: std::collections::HashMap<String, Vec<LinkedPostResult>> =
            std::collections::HashMap::new();
        for lp in linked_posts {
            posts_by_note
                .entry(lp.note_id.to_string())
                .or_default()
                .push(LinkedPostResult {
                    id: lp.post_id.to_string(),
                    title: lp.post_title,
                });
        }

        let note_results: Vec<NoteResult> = notes
            .into_iter()
            .map(|n| {
                let id_str = n.id.to_string();
                let mut result = NoteResult::from(n);
                result.linked_posts = Some(posts_by_note.remove(&id_str).unwrap_or_default());
                result
            })
            .collect();

        Ok(NoteListResult {
            notes: note_results,
        })
    }

    async fn link(
        &self,
        ctx: Context<'_>,
        req: LinkNoteRequest,
    ) -> Result<NoteResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let note_id = NoteId::from(req.note_id);
        Noteable::create(note_id, &req.noteable_type, req.noteable_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let note = Note::find_by_id(note_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(NoteResult::from(note))
    }

    async fn unlink(
        &self,
        ctx: Context<'_>,
        req: UnlinkNoteRequest,
    ) -> Result<EmptyRequest, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        Noteable::delete(
            NoteId::from(req.note_id),
            &req.noteable_type,
            req.noteable_id,
            &self.deps.db_pool,
        )
        .await
        .map_err(|e| TerminalError::new(e.to_string()))?;

        Ok(EmptyRequest {})
    }

    async fn generate_notes(
        &self,
        ctx: Context<'_>,
        req: GenerateNotesRequest,
    ) -> Result<GenerateNotesResult, HandlerError> {
        let _user = require_admin(ctx.headers(), &self.deps.jwt_service)?;

        let org_id = OrganizationId::from(req.organization_id);
        let org = Organization::find_by_id(org_id, &self.deps.db_pool)
            .await
            .map_err(|e| TerminalError::new(e.to_string()))?;

        let result =
            activities::generate_notes_for_organization(org_id, &org.name, &self.deps)
                .await
                .map_err(|e| TerminalError::new(e.to_string()))?;

        // Attach all org notes to all org posts (best-effort)
        let posts_attached = match activities::attach_notes_to_org_posts(org_id, &self.deps).await {
            Ok(r) => r.noteables_created,
            Err(e) => {
                warn!(org_id = %org_id, error = %e, "Failed to attach notes to posts");
                0
            }
        };

        Ok(GenerateNotesResult {
            notes_created: result.notes_created,
            sources_scanned: result.sources_scanned,
            posts_attached,
        })
    }
}
