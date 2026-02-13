//! Post relevance scoring — standalone LLM pass for human review triage.
//!
//! Evaluates posts on three factors:
//! - Immigration relevance (50%): connected to immigrant communities / the crisis
//! - Actionability (30%): specific event, drive, or action someone can participate in
//! - Completeness (20%): has date, location, contact info, clear next steps
//!
//! Returns a composite 1-10 score + human-readable breakdown.
//! Works on both newly extracted posts and existing posts (batch scoring).

use ai_client::OpenAi;
use anyhow::Result;
use schemars::JsonSchema;
use serde::Deserialize;
use tracing::{info, warn};

use crate::kernel::GPT_5_MINI;

/// LLM response type for structured scoring output.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct RelevanceScoreResponse {
    /// Immigration relevance score (1-10). How connected is this to immigrant communities and the current crisis?
    pub immigration_relevance: i32,
    /// Brief reasoning for the immigration relevance score
    pub immigration_relevance_reasoning: String,
    /// Actionability score (1-10). Is this a specific event, drive, or action someone can show up to or participate in?
    pub actionability: i32,
    /// Brief reasoning for the actionability score
    pub actionability_reasoning: String,
    /// Completeness score (1-10). Does this have date, location, contact info, and clear next steps?
    pub completeness: i32,
    /// Brief reasoning for the completeness score
    pub completeness_reasoning: String,
}

/// Computed relevance score with human-readable breakdown.
pub struct RelevanceScore {
    /// Weighted composite score (1-10)
    pub score: i32,
    /// Human-readable breakdown of per-factor scores and reasoning
    pub breakdown: String,
}

const SCORING_PROMPT: &str = r#"You are evaluating a community resource post for MN Together, a platform connecting communities around the immigration crisis in Minnesota.

Score this post on three factors, each on a scale of 1-10:

## 1. Immigration Relevance (weight: 50%)
How connected is this post to immigrant communities and the current immigration crisis?

- **9-10**: Directly about ICE enforcement, deportation defense, sanctuary, immigrant family support
- **7-8**: Clearly serves immigrant communities (know-your-rights, accompaniment, legal aid for immigrants)
- **5-6**: Related to immigrant communities but not crisis-specific (general community event at an immigrant-serving org)
- **3-4**: Tangentially related (general social service that immigrants might use)
- **1-2**: Not related to immigrant communities or the immigration crisis

## 2. Actionability (weight: 30%)
Is this a specific event, drive, or action that someone can show up to or participate in?

- **9-10**: Specific event with date, time, and clear way to participate
- **7-8**: Ongoing drive or program with clear signup/participation method
- **5-6**: Has some actionable details but missing key info (no date, vague location)
- **3-4**: Vague call to action without specifics
- **1-2**: Purely informational, no way to participate

## 3. Completeness (weight: 20%)
Does this post have the details someone needs to take action?

- **9-10**: Has date/time, location, contact info, and clear next steps
- **7-8**: Has most key details, missing one element
- **5-6**: Has some details but missing several important ones
- **3-4**: Minimal details, mostly vague
- **1-2**: Almost no actionable details

## Instructions
- Be honest and critical — lower scores help humans focus their review time
- Score based on the post content, not what you imagine it could be
- The organization name provides context but the post content determines the score"#;

/// Score a single post's relevance.
///
/// LLM returns three sub-scores (1-10 each) + reasoning.
/// Code computes the weighted composite: relevance*0.5 + actionability*0.3 + completeness*0.2
pub async fn score_post_relevance(
    title: &str,
    summary: Option<&str>,
    description: &str,
    org_name: &str,
    ai: &OpenAi,
) -> Result<RelevanceScore> {
    let user_prompt = format!(
        "Organization: {}\n\nTitle: {}\n\nSummary: {}\n\nDescription:\n{}",
        org_name,
        title,
        summary.unwrap_or("(none)"),
        description,
    );

    let response: RelevanceScoreResponse = ai
        .extract(GPT_5_MINI, SCORING_PROMPT, &user_prompt)
        .await
        .map_err(|e| anyhow::anyhow!("Relevance scoring failed: {}", e))?;

    // Clamp sub-scores to 1-10
    let relevance = response.immigration_relevance.clamp(1, 10);
    let actionability = response.actionability.clamp(1, 10);
    let completeness = response.completeness.clamp(1, 10);

    // Compute weighted composite
    let composite =
        (relevance as f64 * 0.5 + actionability as f64 * 0.3 + completeness as f64 * 0.2).round()
            as i32;
    let composite = composite.clamp(1, 10);

    let breakdown = format!(
        "Relevance: {}/10 — {}\nActionability: {}/10 — {}\nCompleteness: {}/10 — {}\nComposite: {}/10",
        relevance, response.immigration_relevance_reasoning,
        actionability, response.actionability_reasoning,
        completeness, response.completeness_reasoning,
        composite,
    );

    info!(
        title = %title,
        relevance = relevance,
        actionability = actionability,
        completeness = completeness,
        composite = composite,
        "Post scored"
    );

    Ok(RelevanceScore {
        score: composite,
        breakdown,
    })
}

/// Score a post by ID, looking up content and org name from the database.
/// Best-effort: returns None if scoring fails.
pub async fn score_post_by_id(
    post_id: crate::common::PostId,
    ai: &OpenAi,
    pool: &sqlx::PgPool,
) -> Option<RelevanceScore> {
    use crate::domains::posts::models::Post;

    let post = match Post::find_by_id(post_id, pool).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            warn!(post_id = %post_id, "Post not found for scoring");
            return None;
        }
        Err(e) => {
            warn!(post_id = %post_id, error = %e, "Failed to load post for scoring");
            return None;
        }
    };

    let org_name = Post::find_org_name(post_id, pool)
        .await
        .unwrap_or(None)
        .unwrap_or_else(|| "Unknown Organization".to_string());

    match score_post_relevance(
        &post.title,
        post.summary.as_deref(),
        &post.description,
        &org_name,
        ai,
    )
    .await
    {
        Ok(score) => {
            if let Err(e) =
                Post::update_relevance_score(post_id, score.score, &score.breakdown, pool).await
            {
                warn!(post_id = %post_id, error = %e, "Failed to store relevance score");
            }
            Some(score)
        }
        Err(e) => {
            warn!(post_id = %post_id, error = %e, "Relevance scoring failed");
            None
        }
    }
}
