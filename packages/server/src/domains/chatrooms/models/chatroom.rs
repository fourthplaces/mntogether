use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::common::{
    ChatroomId, DocumentId, DocumentReferenceId, DocumentTranslationId, MessageId,
};

/// Chatroom - anonymous conversation session
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Chatroom {
    pub id: ChatroomId,
    pub language: String, // language_code from active_languages
    pub created_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
}

/// Message - user or AI assistant message in a chatroom
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Message {
    pub id: MessageId,
    pub chatroom_id: ChatroomId,
    pub role: String, // 'user' or 'assistant'
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub sequence_number: i32,
}

/// Message role enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
        }
    }
}

impl std::str::FromStr for MessageRole {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "user" => Ok(MessageRole::User),
            "assistant" => Ok(MessageRole::Assistant),
            _ => Err(anyhow::anyhow!("Invalid message role: {}", s)),
        }
    }
}

/// ReferralDocument - generated markdown document with references to listings
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ReferralDocument {
    pub id: DocumentId,
    pub chatroom_id: Option<ChatroomId>,

    // Content
    pub source_language: String,
    pub content: String, // Markdown with JSX-like components
    pub title: Option<String>,
    pub status: String, // 'draft', 'published', 'archived'

    // Shareable link
    pub slug: String,
    pub edit_token: Option<String>,

    // Analytics
    pub view_count: i32,
    pub last_viewed_at: Option<DateTime<Utc>>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Document status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatus {
    Draft,
    Published,
    Archived,
}

impl std::fmt::Display for DocumentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentStatus::Draft => write!(f, "draft"),
            DocumentStatus::Published => write!(f, "published"),
            DocumentStatus::Archived => write!(f, "archived"),
        }
    }
}

impl std::str::FromStr for DocumentStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "draft" => Ok(DocumentStatus::Draft),
            "published" => Ok(DocumentStatus::Published),
            "archived" => Ok(DocumentStatus::Archived),
            _ => Err(anyhow::anyhow!("Invalid document status: {}", s)),
        }
    }
}

/// ReferralDocumentTranslation - translated version of a document
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ReferralDocumentTranslation {
    pub id: DocumentTranslationId,
    pub document_id: DocumentId,
    pub language_code: String,
    pub content: String,
    pub title: Option<String>,
    pub translated_at: DateTime<Utc>,
    pub translation_model: Option<String>,
}

/// DocumentReference - tracks entities referenced in a document for staleness detection
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DocumentReference {
    pub id: DocumentReferenceId,
    pub document_id: DocumentId,
    pub reference_kind: String, // 'listing', 'organization', 'contact'
    pub reference_id: String,   // UUID as string
    pub referenced_at: DateTime<Utc>,
    pub display_order: i32,
}

/// Reference kind enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceKind {
    Listing,
    Organization,
    Contact,
}

impl std::fmt::Display for ReferenceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReferenceKind::Listing => write!(f, "listing"),
            ReferenceKind::Organization => write!(f, "organization"),
            ReferenceKind::Contact => write!(f, "contact"),
        }
    }
}

impl std::str::FromStr for ReferenceKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "listing" => Ok(ReferenceKind::Listing),
            "organization" => Ok(ReferenceKind::Organization),
            "contact" => Ok(ReferenceKind::Contact),
            _ => Err(anyhow::anyhow!("Invalid reference kind: {}", s)),
        }
    }
}

// =============================================================================
// Chatroom Queries
// =============================================================================

impl Chatroom {
    /// Find chatroom by ID
    pub async fn find_by_id(id: ChatroomId, pool: &PgPool) -> Result<Self> {
        let chatroom = sqlx::query_as::<_, Chatroom>("SELECT * FROM chatrooms WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await?;
        Ok(chatroom)
    }

    /// Create a new chatroom
    pub async fn create(language: String, pool: &PgPool) -> Result<Self> {
        let chatroom = sqlx::query_as::<_, Chatroom>(
            r#"
            INSERT INTO chatrooms (language)
            VALUES ($1)
            RETURNING *
            "#,
        )
        .bind(language)
        .fetch_one(pool)
        .await?;
        Ok(chatroom)
    }

    /// Update last activity timestamp
    pub async fn touch_activity(id: ChatroomId, pool: &PgPool) -> Result<Self> {
        let chatroom = sqlx::query_as::<_, Chatroom>(
            r#"
            UPDATE chatrooms
            SET last_activity_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(chatroom)
    }

    /// Find recent chatrooms
    pub async fn find_recent(limit: i64, pool: &PgPool) -> Result<Vec<Self>> {
        let chatrooms = sqlx::query_as::<_, Chatroom>(
            "SELECT * FROM chatrooms ORDER BY last_activity_at DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;
        Ok(chatrooms)
    }

    /// Delete a chatroom
    pub async fn delete(id: ChatroomId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM chatrooms WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

// =============================================================================
// Message Queries
// =============================================================================

impl Message {
    /// Find message by ID
    pub async fn find_by_id(id: MessageId, pool: &PgPool) -> Result<Self> {
        let message = sqlx::query_as::<_, Message>("SELECT * FROM messages WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await?;
        Ok(message)
    }

    /// Find messages for a chatroom
    pub async fn find_by_chatroom(chatroom_id: ChatroomId, pool: &PgPool) -> Result<Vec<Self>> {
        let messages = sqlx::query_as::<_, Message>(
            "SELECT * FROM messages WHERE chatroom_id = $1 ORDER BY sequence_number",
        )
        .bind(chatroom_id)
        .fetch_all(pool)
        .await?;
        Ok(messages)
    }

    /// Create a new message
    pub async fn create(
        chatroom_id: ChatroomId,
        role: String,
        content: String,
        sequence_number: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        let message = sqlx::query_as::<_, Message>(
            r#"
            INSERT INTO messages (chatroom_id, role, content, sequence_number)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(chatroom_id)
        .bind(role)
        .bind(content)
        .bind(sequence_number)
        .fetch_one(pool)
        .await?;
        Ok(message)
    }

    /// Get next sequence number for a chatroom
    pub async fn next_sequence_number(chatroom_id: ChatroomId, pool: &PgPool) -> Result<i32> {
        let max: Option<i32> = sqlx::query_scalar(
            "SELECT MAX(sequence_number) FROM messages WHERE chatroom_id = $1",
        )
        .bind(chatroom_id)
        .fetch_one(pool)
        .await?;
        Ok(max.unwrap_or(0) + 1)
    }

    /// Delete all messages in a chatroom
    pub async fn delete_all_for_chatroom(chatroom_id: ChatroomId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM messages WHERE chatroom_id = $1")
            .bind(chatroom_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

// =============================================================================
// ReferralDocument Queries
// =============================================================================

impl ReferralDocument {
    /// Find document by ID
    pub async fn find_by_id(id: DocumentId, pool: &PgPool) -> Result<Self> {
        let document =
            sqlx::query_as::<_, ReferralDocument>("SELECT * FROM referral_documents WHERE id = $1")
                .bind(id)
                .fetch_one(pool)
                .await?;
        Ok(document)
    }

    /// Find document by slug
    pub async fn find_by_slug(slug: &str, pool: &PgPool) -> Result<Option<Self>> {
        let document = sqlx::query_as::<_, ReferralDocument>(
            "SELECT * FROM referral_documents WHERE slug = $1",
        )
        .bind(slug)
        .fetch_optional(pool)
        .await?;
        Ok(document)
    }

    /// Find document by edit token
    pub async fn find_by_edit_token(edit_token: &str, pool: &PgPool) -> Result<Option<Self>> {
        let document = sqlx::query_as::<_, ReferralDocument>(
            "SELECT * FROM referral_documents WHERE edit_token = $1",
        )
        .bind(edit_token)
        .fetch_optional(pool)
        .await?;
        Ok(document)
    }

    /// Create a new document
    pub async fn create(
        chatroom_id: Option<ChatroomId>,
        source_language: String,
        content: String,
        title: Option<String>,
        slug: String,
        edit_token: Option<String>,
        status: String,
        pool: &PgPool,
    ) -> Result<Self> {
        let document = sqlx::query_as::<_, ReferralDocument>(
            r#"
            INSERT INTO referral_documents (
                chatroom_id, source_language, content, title, slug, edit_token, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(chatroom_id)
        .bind(source_language)
        .bind(content)
        .bind(title)
        .bind(slug)
        .bind(edit_token)
        .bind(status)
        .fetch_one(pool)
        .await?;
        Ok(document)
    }

    /// Update document content
    pub async fn update_content(
        id: DocumentId,
        content: String,
        title: Option<String>,
        pool: &PgPool,
    ) -> Result<Self> {
        let document = sqlx::query_as::<_, ReferralDocument>(
            r#"
            UPDATE referral_documents
            SET content = $2, title = COALESCE($3, title), updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(content)
        .bind(title)
        .fetch_one(pool)
        .await?;
        Ok(document)
    }

    /// Update document status
    pub async fn update_status(id: DocumentId, status: String, pool: &PgPool) -> Result<Self> {
        let document = sqlx::query_as::<_, ReferralDocument>(
            r#"
            UPDATE referral_documents
            SET status = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .fetch_one(pool)
        .await?;
        Ok(document)
    }

    /// Increment view count
    pub async fn increment_view_count(id: DocumentId, pool: &PgPool) -> Result<Self> {
        let document = sqlx::query_as::<_, ReferralDocument>(
            r#"
            UPDATE referral_documents
            SET view_count = view_count + 1, last_viewed_at = NOW(), updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(document)
    }

    /// Find documents by chatroom
    pub async fn find_by_chatroom(chatroom_id: ChatroomId, pool: &PgPool) -> Result<Vec<Self>> {
        let documents = sqlx::query_as::<_, ReferralDocument>(
            "SELECT * FROM referral_documents WHERE chatroom_id = $1 ORDER BY created_at DESC",
        )
        .bind(chatroom_id)
        .fetch_all(pool)
        .await?;
        Ok(documents)
    }

    /// Find published documents
    pub async fn find_published(limit: i64, offset: i64, pool: &PgPool) -> Result<Vec<Self>> {
        let documents = sqlx::query_as::<_, ReferralDocument>(
            "SELECT * FROM referral_documents WHERE status = 'published' ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
        Ok(documents)
    }

    /// Delete a document
    pub async fn delete(id: DocumentId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM referral_documents WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

// =============================================================================
// ReferralDocumentTranslation Queries
// =============================================================================

impl ReferralDocumentTranslation {
    /// Find translation by document ID and language
    pub async fn find_by_document_and_language(
        document_id: DocumentId,
        language_code: &str,
        pool: &PgPool,
    ) -> Result<Option<Self>> {
        let translation = sqlx::query_as::<_, ReferralDocumentTranslation>(
            "SELECT * FROM referral_document_translations WHERE document_id = $1 AND language_code = $2",
        )
        .bind(document_id)
        .bind(language_code)
        .fetch_optional(pool)
        .await?;
        Ok(translation)
    }

    /// Find all translations for a document
    pub async fn find_by_document(document_id: DocumentId, pool: &PgPool) -> Result<Vec<Self>> {
        let translations = sqlx::query_as::<_, ReferralDocumentTranslation>(
            "SELECT * FROM referral_document_translations WHERE document_id = $1",
        )
        .bind(document_id)
        .fetch_all(pool)
        .await?;
        Ok(translations)
    }

    /// Create or update translation
    pub async fn create_or_update(
        document_id: DocumentId,
        language_code: String,
        content: String,
        title: Option<String>,
        translation_model: Option<String>,
        pool: &PgPool,
    ) -> Result<Self> {
        let translation = sqlx::query_as::<_, ReferralDocumentTranslation>(
            r#"
            INSERT INTO referral_document_translations (
                document_id, language_code, content, title, translation_model
            )
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (document_id, language_code) DO UPDATE
            SET content = EXCLUDED.content, title = EXCLUDED.title, translated_at = NOW(), translation_model = EXCLUDED.translation_model
            RETURNING *
            "#,
        )
        .bind(document_id)
        .bind(language_code)
        .bind(content)
        .bind(title)
        .bind(translation_model)
        .fetch_one(pool)
        .await?;
        Ok(translation)
    }

    /// Delete a translation
    pub async fn delete(id: DocumentTranslationId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM referral_document_translations WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

// =============================================================================
// DocumentReference Queries
// =============================================================================

impl DocumentReference {
    /// Find references for a document
    pub async fn find_by_document(document_id: DocumentId, pool: &PgPool) -> Result<Vec<Self>> {
        let references = sqlx::query_as::<_, DocumentReference>(
            "SELECT * FROM document_references WHERE document_id = $1 ORDER BY display_order",
        )
        .bind(document_id)
        .fetch_all(pool)
        .await?;
        Ok(references)
    }

    /// Create a reference
    pub async fn create(
        document_id: DocumentId,
        reference_kind: String,
        reference_id: String,
        display_order: i32,
        pool: &PgPool,
    ) -> Result<Self> {
        let reference = sqlx::query_as::<_, DocumentReference>(
            r#"
            INSERT INTO document_references (document_id, reference_kind, reference_id, display_order)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (document_id, reference_kind, reference_id) DO UPDATE
            SET display_order = EXCLUDED.display_order, referenced_at = NOW()
            RETURNING *
            "#,
        )
        .bind(document_id)
        .bind(reference_kind)
        .bind(reference_id)
        .bind(display_order)
        .fetch_one(pool)
        .await?;
        Ok(reference)
    }

    /// Delete all references for a document
    pub async fn delete_all_for_document(document_id: DocumentId, pool: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM document_references WHERE document_id = $1")
            .bind(document_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Find documents that reference a specific entity
    pub async fn find_documents_referencing(
        reference_kind: &str,
        reference_id: &str,
        pool: &PgPool,
    ) -> Result<Vec<DocumentId>> {
        let document_ids = sqlx::query_scalar::<_, DocumentId>(
            "SELECT DISTINCT document_id FROM document_references WHERE reference_kind = $1 AND reference_id = $2",
        )
        .bind(reference_kind)
        .bind(reference_id)
        .fetch_all(pool)
        .await?;
        Ok(document_ids)
    }
}
