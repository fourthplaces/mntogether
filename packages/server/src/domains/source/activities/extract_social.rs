//! Metadata-only LLM extraction for social media posts.
//!
//! Unlike the 3-pass website extraction pipeline (narrative → dedup → investigation),
//! social posts preserve the original caption verbatim as the description. The LLM
//! only extracts metadata: title, summary, contacts, schedule, location, urgency, tags.

use anyhow::Result;
use futures::future::join_all;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::common::extraction_types::ContactInfo;
use crate::common::types::{ExtractedPost, ExtractedSchedule, TagEntry};
use crate::kernel::ServerDeps;

use super::scrape_social::ScrapedSocialPost;

/// Metadata extracted by the LLM from a social media caption.
/// The caption itself is preserved verbatim — this struct only holds derived metadata.
///
/// Contact fields are inlined (not nested as ContactInfo) to avoid `allOf` in the
/// JSON schema, which OpenAI strict mode rejects.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SocialPostMetadata {
    /// Action-focused title (e.g., "Get Free Hot Meals Every Tuesday")
    pub title: String,
    /// 2-3 sentence summary (~250 chars) for card previews
    pub summary: String,
    // -- Contact fields (inlined to avoid allOf schema issue) --
    /// Phone number found in the caption
    pub phone: Option<String>,
    /// Email address found in the caption
    pub email: Option<String>,
    /// Website URL found in the caption
    pub website: Option<String>,
    /// Intake/signup form URL found in the caption
    pub intake_form_url: Option<String>,
    /// Contact person name found in the caption
    pub contact_name: Option<String>,
    /// Physical address if mentioned
    pub location: Option<String>,
    pub zip_code: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    /// "low", "medium", "high", "urgent"
    pub urgency: String,
    /// "low", "medium", "high"
    pub confidence: String,
    /// Tag classifications
    #[serde(default)]
    pub tags: Vec<TagEntry>,
    /// Schedule entries
    #[serde(default)]
    pub schedule: Vec<ExtractedSchedule>,
}

impl SocialPostMetadata {
    fn to_contact_info(&self) -> ContactInfo {
        ContactInfo {
            phone: self.phone.clone(),
            email: self.email.clone(),
            website: self.website.clone(),
            intake_form_url: self.intake_form_url.clone(),
            contact_name: self.contact_name.clone(),
            other: vec![],
        }
    }
}

fn build_social_extraction_prompt(tag_instructions: &str) -> String {
    let tag_section = if tag_instructions.is_empty() {
        String::new()
    } else {
        format!(
            "\n- **tags**: Tag classifications:\n{}",
            tag_instructions
        )
    };

    format!(
        r#"You are extracting metadata from a social media post caption. The original caption will be preserved verbatim as the post description — do NOT rewrite it.

Your job is to extract ONLY metadata:

- **title**: An action-focused title describing what someone can DO (e.g., "Get Free Hot Meals Every Tuesday", "Sign Up for Job Training Program"). Do NOT include the organization name in the title. Keep it under 80 characters.
- **summary**: A 2-3 sentence summary (~250 chars) with the key actionable details. What is offered, who can access it, when/where.
- **phone**: Phone number found in caption (null if not found)
- **email**: Email address found in caption (null if not found)
- **website**: Website URL found in caption (null if not found)
- **intake_form_url**: Intake/signup form URL found in caption (null if not found)
- **contact_name**: Contact person name found in caption (null if not found)
- **location**: Physical address if mentioned (null if virtual/not mentioned)
- **zip_code**: 5-digit zip code (null if unknown)
- **city**: City name (e.g., "Minneapolis")
- **state**: 2-letter state abbreviation (e.g., "MN")
- **urgency**: "low" (ongoing service), "medium" (upcoming event), "high" (deadline soon), or "urgent" (immediate need/today)
- **confidence**: "low", "medium", or "high" based on how clearly the caption describes a community resource or service
- **schedule**: Array of schedule entries. For each recurring or one-off schedule mentioned:
  - **frequency**: "weekly", "biweekly", "monthly", or "one_time"
  - **day_of_week**: Lowercase day name ("monday", "tuesday", etc.)
  - **start_time**: Start time in 24h "HH:MM" format
  - **end_time**: End time in 24h "HH:MM" format
  - **date**: Specific date "YYYY-MM-DD" — for one_time events only
  - **notes**: Freeform notes (e.g., "1st and 3rd week only")
  Only include schedule entries with specific day/time info. Empty array if none.{}

Be conservative — only extract information explicitly present in the caption."#,
        tag_section
    )
}

fn build_user_prompt(post: &ScrapedSocialPost) -> String {
    let mut prompt = format!("**Platform**: {}\n", post.platform);

    if let Some(author) = &post.author {
        prompt.push_str(&format!("**Author**: {}\n", author));
    }
    if let Some(location) = &post.location {
        prompt.push_str(&format!("**Tagged Location**: {}\n", location));
    }
    if let Some(posted_at) = &post.posted_at {
        prompt.push_str(&format!("**Posted**: {}\n", posted_at));
    }

    prompt.push_str(&format!("\n**Caption**:\n{}", post.caption));

    prompt
}

/// Extract metadata from social media posts using a single-pass LLM extraction.
///
/// The original caption is preserved verbatim as the post description.
/// The LLM only generates title, summary, contacts, schedule, location, urgency, and tags.
pub async fn extract_posts_from_social(
    posts: &[ScrapedSocialPost],
    tag_instructions: &str,
    deps: &ServerDeps,
) -> Result<Vec<ExtractedPost>> {
    if posts.is_empty() {
        return Ok(vec![]);
    }

    let system_prompt = build_social_extraction_prompt(tag_instructions);

    info!(
        post_count = posts.len(),
        "Extracting metadata from social posts"
    );

    let futures: Vec<_> = posts
        .iter()
        .enumerate()
        .map(|(idx, post)| {
            let system = system_prompt.clone();
            let user = build_user_prompt(post);
            let caption = post.caption.clone();
            let source_url = post.source_url.clone();
            let ai = deps.ai.clone();

            async move {
                let result = ai
                    .extract::<SocialPostMetadata>("gpt-4o", &system, &user)
                    .await;

                match result {
                    Ok(metadata) => {
                        info!(
                            idx = idx,
                            title = %metadata.title,
                            "Social post metadata extracted"
                        );

                        let contact = metadata.to_contact_info();
                        let tags = TagEntry::to_map(&metadata.tags);

                        Some(ExtractedPost {
                            title: metadata.title,
                            summary: metadata.summary,
                            description: caption,
                            contact: Some(contact),
                            location: metadata.location,
                            urgency: Some(metadata.urgency),
                            confidence: Some(metadata.confidence),
                            source_page_snapshot_id: None,
                            source_url: Some(source_url),
                            zip_code: metadata.zip_code,
                            city: metadata.city,
                            state: metadata.state,
                            tags,
                            schedule: metadata.schedule,
                        })
                    }
                    Err(e) => {
                        warn!(
                            idx = idx,
                            error = %e,
                            "Failed to extract metadata from social post, skipping"
                        );
                        None
                    }
                }
            }
        })
        .collect();

    let results = join_all(futures).await;
    let extracted: Vec<ExtractedPost> = results.into_iter().flatten().collect();

    info!(
        input_count = posts.len(),
        extracted_count = extracted.len(),
        "Social post metadata extraction complete"
    );

    Ok(extracted)
}
