use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

use crate::common::{NoteId, NoteableId};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Note {
    pub id: NoteId,
    pub content: String,
    pub severity: String,
    pub source_url: Option<String>,
    pub source_id: Option<Uuid>,
    pub source_type: Option<String>,
    pub is_public: bool,
    pub created_by: String,
    pub expired_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub embedding: Option<pgvector::Vector>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Noteable {
    pub id: NoteableId,
    pub note_id: NoteId,
    pub noteable_type: String,
    pub noteable_id: Uuid,
    pub added_at: DateTime<Utc>,
}

// =============================================================================
// Note Queries
// =============================================================================

impl Note {
    pub async fn create(
        content: &str,
        severity: &str,
        source_url: Option<&str>,
        source_id: Option<Uuid>,
        source_type: Option<&str>,
        is_public: bool,
        created_by: &str,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO notes (content, severity, source_url, source_id, source_type, is_public, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(content)
        .bind(severity)
        .bind(source_url)
        .bind(source_id)
        .bind(source_type)
        .bind(is_public)
        .bind(created_by)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn find_by_id(id: NoteId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>("SELECT * FROM notes WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn update(
        id: NoteId,
        content: &str,
        severity: &str,
        is_public: bool,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE notes SET content = $2, severity = $3, is_public = $4, updated_at = now() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(content)
        .bind(severity)
        .bind(is_public)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn delete(id: NoteId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM notes WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn expire(id: NoteId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE notes SET expired_at = now(), updated_at = now() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    pub async fn unexpire(id: NoteId, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "UPDATE notes SET expired_at = NULL, updated_at = now() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Update embedding for a note (for semantic matching against posts).
    pub async fn update_embedding(id: NoteId, embedding: &[f32], pool: &PgPool) -> Result<()> {
        use pgvector::Vector;
        let vector = Vector::from(embedding.to_vec());
        sqlx::query("UPDATE notes SET embedding = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(vector)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Find all notes linked to an entity (including expired).
    pub async fn find_for_entity(
        noteable_type: &str,
        noteable_id: Uuid,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT n.*
            FROM notes n
            INNER JOIN noteables nb ON nb.note_id = n.id
            WHERE nb.noteable_type = $1 AND nb.noteable_id = $2
            ORDER BY
                CASE n.severity WHEN 'urgent' THEN 0 WHEN 'notice' THEN 1 ELSE 2 END,
                n.created_at DESC
            "#,
        )
        .bind(noteable_type)
        .bind(noteable_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find active (non-expired) notes for an entity.
    pub async fn find_active_for_entity(
        noteable_type: &str,
        noteable_id: Uuid,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT n.*
            FROM notes n
            INNER JOIN noteables nb ON nb.note_id = n.id
            WHERE nb.noteable_type = $1 AND nb.noteable_id = $2
              AND n.expired_at IS NULL
            ORDER BY
                CASE n.severity WHEN 'urgent' THEN 0 WHEN 'notice' THEN 1 ELSE 2 END,
                n.created_at DESC
            "#,
        )
        .bind(noteable_type)
        .bind(noteable_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find public, active notes for an entity (for public display).
    pub async fn find_public_for_entity(
        noteable_type: &str,
        noteable_id: Uuid,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT n.*
            FROM notes n
            INNER JOIN noteables nb ON nb.note_id = n.id
            WHERE nb.noteable_type = $1 AND nb.noteable_id = $2
              AND n.is_public = true
              AND n.expired_at IS NULL
            ORDER BY
                CASE n.severity WHEN 'urgent' THEN 0 WHEN 'notice' THEN 1 ELSE 2 END,
                n.created_at DESC
            "#,
        )
        .bind(noteable_type)
        .bind(noteable_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find post IDs that have active, public, urgent notes attached.
    pub async fn find_post_ids_with_urgent_notes(
        post_ids: &[Uuid],
        pool: &PgPool,
    ) -> Result<HashSet<Uuid>> {
        if post_ids.is_empty() {
            return Ok(HashSet::new());
        }
        let rows = sqlx::query_as::<_, (Uuid,)>(
            r#"
            SELECT DISTINCT nb.noteable_id
            FROM noteables nb
            JOIN notes n ON n.id = nb.note_id
            WHERE nb.noteable_type = 'post'
              AND nb.noteable_id = ANY($1)
              AND n.severity = 'urgent'
              AND n.is_public = true
              AND n.expired_at IS NULL
            "#,
        )
        .bind(post_ids)
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Find notes by source (for deduplication and refresh).
    pub async fn find_by_source(
        source_type: &str,
        source_id: Uuid,
        pool: &PgPool,
    ) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM notes WHERE source_type = $1 AND source_id = $2 ORDER BY created_at DESC",
        )
        .bind(source_type)
        .bind(source_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}

// =============================================================================
// Noteable Queries
// =============================================================================

impl Noteable {
    /// Link a note to an entity. Idempotent (ON CONFLICT DO NOTHING).
    pub async fn create(
        note_id: NoteId,
        noteable_type: &str,
        noteable_id: Uuid,
        pool: &PgPool,
    ) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO noteables (note_id, noteable_type, noteable_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (note_id, noteable_type, noteable_id) DO UPDATE
            SET note_id = EXCLUDED.note_id
            RETURNING *
            "#,
        )
        .bind(note_id)
        .bind(noteable_type)
        .bind(noteable_id)
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }

    /// Unlink a note from an entity.
    pub async fn delete(
        note_id: NoteId,
        noteable_type: &str,
        noteable_id: Uuid,
        pool: &PgPool,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM noteables WHERE note_id = $1 AND noteable_type = $2 AND noteable_id = $3",
        )
        .bind(note_id)
        .bind(noteable_type)
        .bind(noteable_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Find all linked entities for a note.
    pub async fn find_for_note(note_id: NoteId, pool: &PgPool) -> Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM noteables WHERE note_id = $1 ORDER BY added_at DESC",
        )
        .bind(note_id)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }

    /// Find all linked posts for a batch of notes (avoids N+1).
    /// Returns (note_id, post_id, post_title) tuples.
    pub async fn find_linked_posts_for_notes(
        note_ids: &[NoteId],
        pool: &PgPool,
    ) -> Result<Vec<NoteLinkedPost>> {
        if note_ids.is_empty() {
            return Ok(Vec::new());
        }
        let uuids: Vec<Uuid> = note_ids.iter().map(|id| (*id).into()).collect();
        sqlx::query_as::<_, NoteLinkedPost>(
            r#"
            SELECT nb.note_id, p.id AS post_id, p.title AS post_title
            FROM noteables nb
            INNER JOIN posts p ON p.id = nb.noteable_id
            WHERE nb.noteable_type = 'post'
              AND nb.note_id = ANY($1)
              AND p.deleted_at IS NULL
            ORDER BY p.created_at DESC
            "#,
        )
        .bind(&uuids)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NoteLinkedPost {
    pub note_id: NoteId,
    pub post_id: Uuid,
    pub post_title: String,
}
