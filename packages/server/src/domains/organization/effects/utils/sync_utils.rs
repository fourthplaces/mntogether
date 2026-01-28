use crate::common::utils::generate_content_hash;
use crate::domains::organization::models::{NeedStatus, OrganizationNeed};
use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

/// Sync result showing what changed
#[derive(Debug)]
pub struct SyncResult {
    pub new_needs: Vec<Uuid>,
    pub unchanged_needs: Vec<Uuid>,
    pub changed_needs: Vec<Uuid>,
    pub disappeared_needs: Vec<Uuid>,
}

/// Extracted need input (from AI)
#[derive(Debug, Clone)]
pub struct ExtractedNeedInput {
    pub organization_name: String,
    pub title: String,
    pub description: String,
    pub description_markdown: Option<String>,
    pub tldr: Option<String>,
    pub contact: Option<serde_json::Value>,
    pub urgency: Option<String>,
    pub confidence: Option<String>,
}

/// Synchronize extracted needs with database
///
/// Algorithm:
/// 1. Calculate content hash for each extracted need
/// 2. Find existing needs from same source
/// 3. Compare hashes:
///    - Same hash = unchanged (update last_seen_at)
///    - Different hash = changed (create new pending_approval)
///    - Not found = new (create pending_approval)
/// 4. Mark needs not in extracted set as disappeared
pub async fn sync_needs(
    pool: &PgPool,
    source_id: Uuid,
    extracted_needs: Vec<ExtractedNeedInput>,
) -> Result<SyncResult> {
    let now = Utc::now();

    // Calculate content hashes for extracted needs
    let extracted_with_hashes: Vec<_> = extracted_needs
        .into_iter()
        .map(|need| {
            let content_hash = generate_content_hash(&format!(
                "{} {} {}",
                need.title, need.description, need.organization_name
            ));
            (need, content_hash)
        })
        .collect();

    // Fetch existing active needs from this source
    let existing_needs = sqlx::query_as::<_, ExistingNeed>(
        r#"
        SELECT id, content_hash
        FROM organization_needs
        WHERE source_id = $1
          AND status IN ('pending_approval', 'active')
          AND disappeared_at IS NULL
        "#,
    )
    .bind(source_id)
    .fetch_all(pool)
    .await?;

    let mut new_needs = Vec::new();
    let mut unchanged_needs = Vec::new();
    let mut changed_needs = Vec::new();

    // Process each extracted need
    for (need, content_hash) in extracted_with_hashes {
        // Check if this content hash exists in database
        if let Some(existing) = existing_needs
            .iter()
            .find(|n| n.content_hash.as_ref() == Some(&content_hash))
        {
            // Unchanged - just update last_seen_at
            sqlx::query!(
                r#"
                UPDATE organization_needs
                SET last_seen_at = $1
                WHERE id = $2
                "#,
                now,
                existing.id
            )
            .execute(pool)
            .await?;

            unchanged_needs.push(existing.id);
        } else {
            // Check if there's an existing need with same title (might be changed)
            let maybe_changed = sqlx::query_as::<_, ExistingNeed>(
                r#"
                SELECT id, content_hash
                FROM organization_needs
                WHERE source_id = $1
                  AND title = $2
                  AND status IN ('pending_approval', 'active')
                  AND disappeared_at IS NULL
                LIMIT 1
                "#,
            )
            .bind(source_id)
            .bind(&need.title)
            .fetch_optional(pool)
            .await?;

            if maybe_changed.is_some() {
                // Content changed - create new pending_approval need
                let new_id = create_pending_need(pool, source_id, &need, &content_hash).await?;
                changed_needs.push(new_id);
            } else {
                // New need - create pending_approval
                let new_id = create_pending_need(pool, source_id, &need, &content_hash).await?;
                new_needs.push(new_id);
            }
        }
    }

    // Find needs that disappeared (weren't in extracted set)
    let extracted_hashes: Vec<_> = extracted_with_hashes
        .iter()
        .map(|(_, hash)| hash.as_str())
        .collect();

    let disappeared_needs = sqlx::query_scalar::<_, Uuid>(
        r#"
        UPDATE organization_needs
        SET disappeared_at = $1
        WHERE source_id = $2
          AND status IN ('pending_approval', 'active')
          AND disappeared_at IS NULL
          AND content_hash NOT IN (SELECT * FROM UNNEST($3::text[]))
        RETURNING id
        "#,
    )
    .bind(now)
    .bind(source_id)
    .bind(&extracted_hashes)
    .fetch_all(pool)
    .await?;

    Ok(SyncResult {
        new_needs,
        unchanged_needs,
        changed_needs,
        disappeared_needs,
    })
}

/// Create a new pending need
async fn create_pending_need(
    pool: &PgPool,
    source_id: Uuid,
    need: &ExtractedNeedInput,
    content_hash: &str,
) -> Result<Uuid> {
    let contact_json = serde_json::to_value(&need.contact)?;

    let row = sqlx::query!(
        r#"
        INSERT INTO organization_needs (
            organization_name,
            title,
            description,
            description_markdown,
            tldr,
            contact_info,
            urgency,
            status,
            content_hash,
            source_id
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id
        "#,
        need.organization_name,
        need.title,
        need.description,
        need.description_markdown,
        need.tldr,
        contact_json,
        need.urgency,
        NeedStatus::PendingApproval.to_string(),
        content_hash,
        source_id
    )
    .fetch_one(pool)
    .await?;

    Ok(row.id)
}

#[derive(Debug, sqlx::FromRow)]
struct ExistingNeed {
    id: Uuid,
    content_hash: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash_generation() {
        let need = ExtractedNeedInput {
            organization_name: "Test Org".to_string(),
            title: "Help Needed".to_string(),
            description: "We need volunteers".to_string(),
            description_markdown: None,
            tldr: None,
            contact: None,
            urgency: None,
            confidence: None,
        };

        let hash1 = generate_content_hash(&format!(
            "{} {} {}",
            need.title, need.description, need.organization_name
        ));

        // Same content should produce same hash
        let hash2 = generate_content_hash(&format!(
            "{} {} {}",
            need.title, need.description, need.organization_name
        ));

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 is 64 hex chars
    }
}
