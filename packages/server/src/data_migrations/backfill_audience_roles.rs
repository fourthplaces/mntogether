//! Backfill audience roles for existing listings using AI classification
//!
//! This migration:
//! 1. Finds listings without audience_role tags
//! 2. Uses AI to classify the audience role based on title/description
//! 3. Tags each listing with the appropriate audience roles

use super::{DataMigration, MigrationContext, MigrationResult, VerifyResult};
use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

/// Migration to backfill audience roles for existing listings
pub struct BackfillAudienceRolesMigration;

/// Audience role classification from AI
#[derive(Debug, Clone, serde::Deserialize)]
struct AudienceClassification {
    roles: Vec<String>,
}

/// Classify a listing's audience roles using AI
async fn classify_audience_roles(
    title: &str,
    description: &str,
    openai_api_key: &str,
) -> Result<Vec<String>> {
    let client = reqwest::Client::new();

    let prompt = format!(
        r#"Classify this listing's target audience. Return a JSON object with a "roles" array containing one or more of:
- "recipient": People receiving services/benefits (food, housing, healthcare assistance, etc.)
- "donor": People giving money, food, goods, or other resources
- "volunteer": People giving their time to help
- "participant": People attending events, classes, groups, or programs

Listing Title: {title}
Description: {description}

Return ONLY valid JSON like: {{"roles": ["recipient", "volunteer"]}}
Do not include markdown or explanation. Just the JSON object."#,
        title = title,
        description = description
    );

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", openai_api_key))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": "gpt-4o-mini",
            "messages": [
                {"role": "system", "content": "You are a classifier that categorizes community listings by their target audience. Always respond with valid JSON only."},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.1,
            "max_tokens": 100
        }))
        .send()
        .await
        .context("Failed to call OpenAI API")?;

    let response_body: serde_json::Value = response.json().await?;

    let content = response_body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("{}");

    // Parse the response
    let classification: AudienceClassification =
        serde_json::from_str(content).unwrap_or(AudienceClassification { roles: vec![] });

    // Validate roles
    let valid_roles: Vec<String> = classification
        .roles
        .into_iter()
        .filter(|r| ["recipient", "donor", "volunteer", "participant"].contains(&r.as_str()))
        .collect();

    Ok(valid_roles)
}

#[async_trait]
impl DataMigration for BackfillAudienceRolesMigration {
    fn name(&self) -> &'static str {
        "backfill_audience_roles"
    }

    fn description(&self) -> &'static str {
        "Classify and tag listings with audience roles using AI"
    }

    async fn estimate(&self, db: &PgPool) -> Result<i64> {
        // Count listings without audience_role tags
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(DISTINCT l.id)
            FROM listings l
            WHERE l.status IN ('active', 'pending_approval')
              AND NOT EXISTS (
                SELECT 1 FROM taggables tg
                JOIN tags t ON t.id = tg.tag_id
                WHERE tg.taggable_type = 'listing'
                  AND tg.taggable_id = l.id
                  AND t.kind = 'audience_role'
              )
            "#,
        )
        .fetch_one(db)
        .await?;

        Ok(count.0)
    }

    async fn find_work(&self, cursor: Option<Uuid>, limit: i64, db: &PgPool) -> Result<Vec<Uuid>> {
        let ids: Vec<(Uuid,)> = match cursor {
            Some(c) => {
                sqlx::query_as(
                    r#"
                    SELECT l.id
                    FROM listings l
                    WHERE l.status IN ('active', 'pending_approval')
                      AND l.id > $1
                      AND NOT EXISTS (
                        SELECT 1 FROM taggables tg
                        JOIN tags t ON t.id = tg.tag_id
                        WHERE tg.taggable_type = 'listing'
                          AND tg.taggable_id = l.id
                          AND t.kind = 'audience_role'
                      )
                    ORDER BY l.id
                    LIMIT $2
                    "#,
                )
                .bind(c)
                .bind(limit)
                .fetch_all(db)
                .await?
            }
            None => {
                sqlx::query_as(
                    r#"
                    SELECT l.id
                    FROM listings l
                    WHERE l.status IN ('active', 'pending_approval')
                      AND NOT EXISTS (
                        SELECT 1 FROM taggables tg
                        JOIN tags t ON t.id = tg.tag_id
                        WHERE tg.taggable_type = 'listing'
                          AND tg.taggable_id = l.id
                          AND t.kind = 'audience_role'
                      )
                    ORDER BY l.id
                    LIMIT $1
                    "#,
                )
                .bind(limit)
                .fetch_all(db)
                .await?
            }
        };

        Ok(ids.into_iter().map(|(id,)| id).collect())
    }

    async fn execute_one(&self, id: Uuid, ctx: &MigrationContext) -> Result<MigrationResult> {
        // Fetch listing details
        let listing: Option<(String, String)> = sqlx::query_as(
            r#"
            SELECT title, description
            FROM listings
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&ctx.db_pool)
        .await?;

        let (title, description) = match listing {
            Some(l) => l,
            None => return Ok(MigrationResult::Skipped),
        };

        // Check if already has audience_role tags (idempotency)
        let has_tags: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM taggables tg
            JOIN tags t ON t.id = tg.tag_id
            WHERE tg.taggable_type = 'listing'
              AND tg.taggable_id = $1
              AND t.kind = 'audience_role'
            "#,
        )
        .bind(id)
        .fetch_one(&ctx.db_pool)
        .await?;

        if has_tags.0 > 0 {
            return Ok(MigrationResult::Skipped);
        }

        // Dry-run mode
        if ctx.dry_run {
            // Still call AI to validate classification works
            let openai_key = std::env::var("OPENAI_API_KEY")
                .context("OPENAI_API_KEY environment variable not set")?;
            let roles = classify_audience_roles(&title, &description, &openai_key).await?;
            tracing::info!(
                listing_id = %id,
                title = %title,
                roles = ?roles,
                "Would tag listing with audience roles"
            );
            return Ok(MigrationResult::WouldMigrate);
        }

        // Classify using AI
        let openai_key = std::env::var("OPENAI_API_KEY")
            .context("OPENAI_API_KEY environment variable not set")?;
        let roles = classify_audience_roles(&title, &description, &openai_key).await?;

        if roles.is_empty() {
            tracing::warn!(
                listing_id = %id,
                title = %title,
                "AI could not classify audience roles"
            );
            return Ok(MigrationResult::Skipped);
        }

        // Tag the listing with each role
        for role in &roles {
            // Get or create the tag
            let tag: (Uuid,) = sqlx::query_as(
                r#"
                INSERT INTO tags (kind, value, display_name)
                VALUES ('audience_role', $1, $2)
                ON CONFLICT (kind, value) DO UPDATE SET display_name = EXCLUDED.display_name
                RETURNING id
                "#,
            )
            .bind(role)
            .bind(capitalize(role))
            .fetch_one(&ctx.db_pool)
            .await?;

            // Create the taggable association
            sqlx::query(
                r#"
                INSERT INTO taggables (tag_id, taggable_type, taggable_id)
                VALUES ($1, 'listing', $2)
                ON CONFLICT (tag_id, taggable_type, taggable_id) DO NOTHING
                "#,
            )
            .bind(tag.0)
            .bind(id)
            .execute(&ctx.db_pool)
            .await?;
        }

        tracing::info!(
            listing_id = %id,
            title = %title,
            roles = ?roles,
            "Tagged listing with audience roles"
        );

        Ok(MigrationResult::Migrated)
    }

    async fn verify(&self, db: &PgPool) -> Result<VerifyResult> {
        // Check if any active/pending listings still lack audience_role tags
        let remaining: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(DISTINCT l.id)
            FROM listings l
            WHERE l.status IN ('active', 'pending_approval')
              AND NOT EXISTS (
                SELECT 1 FROM taggables tg
                JOIN tags t ON t.id = tg.tag_id
                WHERE tg.taggable_type = 'listing'
                  AND tg.taggable_id = l.id
                  AND t.kind = 'audience_role'
              )
            "#,
        )
        .fetch_one(db)
        .await?;

        if remaining.0 == 0 {
            Ok(VerifyResult::Passed)
        } else {
            Ok(VerifyResult::Incomplete {
                remaining: remaining.0,
            })
        }
    }

    fn batch_size(&self) -> i64 {
        // Small batch size due to AI API calls
        10
    }

    fn error_budget(&self) -> f64 {
        0.05 // 5% error rate allowed (AI might fail on some)
    }
}

/// Capitalize first letter
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
