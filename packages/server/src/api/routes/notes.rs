use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::auth::AdminUser;
use crate::api::error::{ApiError, ApiResult};
use crate::api::state::AppState;
use crate::common::NoteId;
use crate::domains::notes::models::{Note, Noteable};

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct EmptyRequest {}

#[derive(Debug, Deserialize)]
pub struct CreateNoteRequest {
    pub content: String,
    pub severity: Option<String>,
    pub source_url: Option<String>,
    pub source_id: Option<Uuid>,
    pub source_type: Option<String>,
    pub is_public: Option<bool>,
    pub cta_text: Option<String>,
    pub noteable_type: Option<String>,
    pub noteable_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct GetNoteRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNoteRequest {
    pub id: Uuid,
    pub content: String,
    pub severity: String,
    pub is_public: bool,
    pub cta_text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteNoteRequest {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ListNotesRequest {
    pub severity: Option<String>,
    pub is_public: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ListNotesForEntityRequest {
    pub noteable_type: String,
    pub noteable_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct LinkNoteRequest {
    pub note_id: Uuid,
    pub noteable_type: String,
    pub noteable_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UnlinkNoteRequest {
    pub note_id: Uuid,
    pub noteable_type: String,
    pub noteable_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct GenerateNotesRequest {
    #[allow(dead_code)]
    pub organization_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct AttachNotesRequest {
    pub organization_id: Uuid,
}

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Serialize)]
pub struct LinkedPostResult {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Serialize)]
pub struct LinkedOrgResult {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct NoteResult {
    pub id: String,
    pub content: String,
    pub cta_text: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_orgs: Option<Vec<LinkedOrgResult>>,
}

impl From<Note> for NoteResult {
    fn from(note: Note) -> Self {
        Self {
            id: note.id.to_string(),
            content: note.content,
            cta_text: note.cta_text,
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
            linked_orgs: None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NoteListResult {
    pub notes: Vec<NoteResult>,
}

#[derive(Debug, Serialize)]
pub struct NoteListPaginatedResult {
    pub notes: Vec<NoteResult>,
    pub total_count: i64,
}

#[derive(Debug, Serialize)]
pub struct GenerateNotesResult {
    pub notes_created: i32,
    pub sources_scanned: i32,
    pub posts_attached: i32,
}

#[derive(Debug, Serialize)]
pub struct AttachNotesResult {
    pub notes_count: i32,
    pub posts_count: i32,
    pub noteables_created: i32,
}

// =============================================================================
// Handlers
// =============================================================================

async fn create(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<CreateNoteRequest>,
) -> ApiResult<Json<NoteResult>> {
    let pool = &state.deps.db_pool;

    let note = Note::create(
        &req.content,
        req.severity.as_deref().unwrap_or("info"),
        req.source_url.as_deref(),
        req.source_id,
        req.source_type.as_deref(),
        req.is_public.unwrap_or(false),
        "admin",
        req.cta_text.as_deref(),
        pool,
    )
    .await?;

    // If entity linking info provided, link immediately
    if let (Some(noteable_type), Some(noteable_id)) = (&req.noteable_type, &req.noteable_id) {
        Noteable::create(note.id, noteable_type, *noteable_id, pool).await?;
    }

    Ok(Json(NoteResult::from(note)))
}

async fn get(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<GetNoteRequest>,
) -> ApiResult<Json<NoteResult>> {
    let pool = &state.deps.db_pool;
    let note_id = NoteId::from(req.id);
    let note = Note::find_by_id(note_id, pool).await?;

    // Fetch linked posts
    let linked_posts = Noteable::find_linked_posts_for_notes(&[note_id], pool)
        .await
        .unwrap_or_default();
    let post_results: Vec<LinkedPostResult> = linked_posts
        .into_iter()
        .map(|lp| LinkedPostResult {
            id: lp.post_id.to_string(),
            title: lp.post_title,
        })
        .collect();

    // Fetch linked organizations
    let linked_orgs = Noteable::find_linked_orgs_for_notes(&[note_id], pool)
        .await
        .unwrap_or_default();
    let org_results: Vec<LinkedOrgResult> = linked_orgs
        .into_iter()
        .map(|lo| LinkedOrgResult {
            id: lo.org_id.to_string(),
            name: lo.org_name,
        })
        .collect();

    let mut result = NoteResult::from(note);
    result.linked_posts = Some(post_results);
    result.linked_orgs = Some(org_results);
    Ok(Json(result))
}

async fn update(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<UpdateNoteRequest>,
) -> ApiResult<Json<NoteResult>> {
    let note = Note::update(
        NoteId::from(req.id),
        &req.content,
        &req.severity,
        req.is_public,
        req.cta_text.as_deref(),
        &state.deps.db_pool,
    )
    .await?;

    Ok(Json(NoteResult::from(note)))
}

async fn delete(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<DeleteNoteRequest>,
) -> ApiResult<Json<EmptyRequest>> {
    Note::delete(NoteId::from(req.id), &state.deps.db_pool).await?;
    Ok(Json(EmptyRequest {}))
}

async fn list(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<ListNotesRequest>,
) -> ApiResult<Json<NoteListPaginatedResult>> {
    let pool = &state.deps.db_pool;
    let limit = req.limit.unwrap_or(50).min(200);
    let offset = req.offset.unwrap_or(0);

    let notes = Note::find_all(
        req.severity.as_deref(),
        req.is_public,
        limit,
        offset,
        pool,
    )
    .await?;

    let total_count = Note::count_all(
        req.severity.as_deref(),
        req.is_public,
        pool,
    )
    .await?;

    // Batch-fetch linked posts for all notes
    let note_ids: Vec<_> = notes.iter().map(|n| n.id).collect();
    let linked_posts = Noteable::find_linked_posts_for_notes(&note_ids, pool)
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

    Ok(Json(NoteListPaginatedResult {
        notes: note_results,
        total_count,
    }))
}

async fn list_for_entity(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<ListNotesForEntityRequest>,
) -> ApiResult<Json<NoteListResult>> {
    let pool = &state.deps.db_pool;

    let notes = Note::find_for_entity(&req.noteable_type, req.noteable_id, pool).await?;

    // Batch-fetch linked posts for all notes
    let note_ids: Vec<_> = notes.iter().map(|n| n.id).collect();
    let linked_posts = Noteable::find_linked_posts_for_notes(&note_ids, pool)
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

    Ok(Json(NoteListResult {
        notes: note_results,
    }))
}

async fn link(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<LinkNoteRequest>,
) -> ApiResult<Json<NoteResult>> {
    let pool = &state.deps.db_pool;
    let note_id = NoteId::from(req.note_id);

    Noteable::create(note_id, &req.noteable_type, req.noteable_id, pool).await?;

    let note = Note::find_by_id(note_id, pool).await?;
    Ok(Json(NoteResult::from(note)))
}

async fn unlink(
    State(state): State<AppState>,
    _user: AdminUser,
    Json(req): Json<UnlinkNoteRequest>,
) -> ApiResult<Json<EmptyRequest>> {
    Noteable::delete(
        NoteId::from(req.note_id),
        &req.noteable_type,
        req.noteable_id,
        &state.deps.db_pool,
    )
    .await?;

    Ok(Json(EmptyRequest {}))
}

async fn generate_notes(
    _user: AdminUser,
    Json(_req): Json<GenerateNotesRequest>,
) -> ApiResult<Json<GenerateNotesResult>> {
    Err(ApiError::BadRequest(
        "Note generation from extraction is no longer available".to_string(),
    ))
}

async fn attach_notes(
    _user: AdminUser,
    Json(_req): Json<AttachNotesRequest>,
) -> ApiResult<Json<AttachNotesResult>> {
    Err(ApiError::BadRequest(
        "Automatic note attachment is no longer available. Use manual linking instead.".to_string(),
    ))
}

// =============================================================================
// Router
// =============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/Notes/create", post(create))
        .route("/Notes/get", post(get))
        .route("/Notes/update", post(update))
        .route("/Notes/delete", post(delete))
        .route("/Notes/list", post(list))
        .route("/Notes/list_for_entity", post(list_for_entity))
        .route("/Notes/link", post(link))
        .route("/Notes/unlink", post(unlink))
        .route("/Notes/generate_notes", post(generate_notes))
        .route("/Notes/attach_notes", post(attach_notes))
}
