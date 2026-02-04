//! Post extraction via structured LLM output
//!
//! Provides shared extraction logic for converting markdown content
//! to structured `ExtractedPost` items. Used by both the crawling
//! effect handlers and the page regeneration actions.

use anyhow::Result;
use extraction::types::page::CachedPage;
use serde_json::json;
use tracing::{info, warn};

use crate::common::ExtractedPost;
use crate::kernel::{BaseAI, OpenAIExtractionService};

/// System prompt for structured post extraction.
pub const POST_EXTRACTION_PROMPT: &str = r#"You are an expert at extracting structured information from website content.

## Your Task

Extract ALL distinct posts/listings/opportunities from the content. Each post should represent a SEPARATE offering that someone could engage with.

## What Constitutes a Separate Post

Create separate posts when content describes:
- Different services (e.g., "Food Shelf" vs "Clothing Closet")
- Different audiences for the SAME service (e.g., "Food Shelf - Get Help" vs "Food Shelf - Volunteer")
- Different events (e.g., "Monthly Food Drive" vs "Annual Gala")
- Different programs (e.g., "Senior Services" vs "Youth Services")

## Audience Roles

Assign audience_roles based on WHO should engage with this post:
- "recipient" - People who would receive the service/help
- "volunteer" - People who would donate time
- "donor" - People who would donate money/goods
- "participant" - People who would attend/participate

A single service can target multiple audiences (e.g., a food shelf might have separate posts for recipients and volunteers).

## Contact Information

Extract ALL contact methods mentioned:
- phone: Phone numbers
- email: Email addresses
- website: URLs (signup forms, registration pages, etc.)
- intake_form_url: Specific intake/application forms
- contact_name: Contact person name if mentioned

## Confidence Levels

- "high" - All key information clearly stated
- "medium" - Some information inferred or partially stated
- "low" - Information is unclear or may be outdated

## Urgency Levels

- "urgent" - Immediate need, time-sensitive
- "high" - Important, should act soon
- "medium" - Normal priority
- "low" - Ongoing, no time pressure"#;

/// JSON schema for structured post extraction.
/// Note: OpenAI structured output requires top-level "type": "object"
pub fn post_extraction_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "posts": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string", "description": "Clear, descriptive title" },
                        "tldr": { "type": "string", "description": "One-sentence summary (max 100 chars)" },
                        "description": { "type": "string", "description": "Comprehensive description with all relevant details" },
                        "contact": {
                            "type": "object",
                            "properties": {
                                "phone": { "type": ["string", "null"] },
                                "email": { "type": ["string", "null"] },
                                "website": { "type": ["string", "null"] },
                                "intake_form_url": { "type": ["string", "null"] },
                                "contact_name": { "type": ["string", "null"] }
                            },
                            "required": ["phone", "email", "website", "intake_form_url", "contact_name"],
                            "additionalProperties": false
                        },
                        "location": { "type": ["string", "null"], "description": "Physical location if relevant" },
                        "urgency": { "type": "string", "enum": ["low", "medium", "high", "urgent"] },
                        "confidence": { "type": "string", "enum": ["low", "medium", "high"] },
                        "audience_roles": {
                            "type": "array",
                            "items": { "type": "string", "enum": ["recipient", "donor", "volunteer", "participant"] }
                        }
                    },
                    "required": ["title", "tldr", "description", "contact", "location", "urgency", "confidence", "audience_roles"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["posts"],
        "additionalProperties": false
    })
}

/// Simple search query for finding pages with community listings.
/// Used for semantic search - keep it short and topical.
pub const POST_SEARCH_QUERY: &str =
    "volunteer opportunities services programs events donations community resources help";

/// Detailed extraction query (used with the extraction library's .extract() if needed).
/// NOT for semantic search - this is an instruction, not a topic.
#[deprecated(note = "Use POST_SEARCH_QUERY for search, POST_EXTRACTION_PROMPT for extraction")]
pub const POST_EXTRACTION_QUERY: &str = POST_SEARCH_QUERY;

/// Extract structured posts from markdown content.
///
/// # Arguments
/// * `content` - Combined markdown content from extraction library
/// * `context` - Optional context string (e.g., "Organization: Example\nSource URL: https://...")
/// * `ai` - AI service for structured generation
///
/// # Returns
/// Vector of extracted posts, empty if extraction fails or produces no results.
pub async fn extract_posts_from_content(
    content: &str,
    context: Option<&str>,
    ai: &dyn BaseAI,
) -> Result<Vec<ExtractedPost>> {
    if content.trim().is_empty() {
        return Ok(vec![]);
    }

    // Build system prompt with optional context
    let system_prompt = match context {
        Some(ctx) => format!("{}\n\n{}", POST_EXTRACTION_PROMPT, ctx),
        None => POST_EXTRACTION_PROMPT.to_string(),
    };

    let user_prompt = format!("## Content to Extract\n\n{}", content);

    // Extract structured posts directly
    let json_response = ai
        .generate_structured(&system_prompt, &user_prompt, post_extraction_schema())
        .await?;

    // Response is wrapped in { "posts": [...] } object
    #[derive(serde::Deserialize)]
    struct PostsWrapper {
        posts: Vec<ExtractedPost>,
    }

    let wrapper: PostsWrapper = match serde_json::from_str(&json_response) {
        Ok(w) => w,
        Err(e) => {
            warn!(
                error = %e,
                response_preview = %json_response.chars().take(500).collect::<String>(),
                "Failed to parse structured extraction response"
            );
            return Ok(vec![]);
        }
    };

    Ok(wrapper.posts)
}

/// Result of extracting posts for a domain.
#[derive(Debug)]
pub struct DomainExtractionResult {
    /// Extracted posts
    pub posts: Vec<ExtractedPost>,
    /// URLs of pages that were searched
    pub page_urls: Vec<String>,
}

/// Search for pages and extract posts for a domain.
///
/// This is the main entry point for post extraction. It:
/// 1. Searches for relevant pages using semantic search
/// 2. Combines raw page content
/// 3. Extracts structured posts via LLM
///
/// # Arguments
/// * `domain` - Website domain to search (e.g., "redcross.org")
/// * `extraction` - Extraction service for page search
/// * `ai` - AI service for structured extraction
pub async fn extract_posts_for_domain(
    domain: &str,
    extraction: &OpenAIExtractionService,
    ai: &dyn BaseAI,
) -> Result<DomainExtractionResult> {
    // Search for relevant pages
    let pages = extraction
        .search_and_get_pages(POST_SEARCH_QUERY, Some(domain), 50)
        .await?;

    info!(
        domain = %domain,
        pages_found = pages.len(),
        page_urls = ?pages.iter().map(|p| &p.url).collect::<Vec<_>>(),
        "Search results"
    );

    if pages.is_empty() {
        return Ok(DomainExtractionResult {
            posts: vec![],
            page_urls: vec![],
        });
    }

    let page_urls: Vec<String> = pages.iter().map(|p| p.url.clone()).collect();

    // Combine and extract
    let posts = extract_posts_from_pages(&pages, Some(domain), ai).await?;

    Ok(DomainExtractionResult { posts, page_urls })
}

/// Extract posts from a set of pages.
///
/// Lower-level function that takes already-fetched pages.
pub async fn extract_posts_from_pages(
    pages: &[CachedPage],
    domain: Option<&str>,
    ai: &dyn BaseAI,
) -> Result<Vec<ExtractedPost>> {
    if pages.is_empty() {
        return Ok(vec![]);
    }

    // Combine raw page content
    let combined_content: String = pages
        .iter()
        .map(|p| format!("## Source: {}\n\n{}", p.url, p.content))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    info!(
        pages_count = pages.len(),
        content_len = combined_content.len(),
        "Extracting structured posts from raw content"
    );

    // Build context
    let context = domain.map(|d| format!("Organization: {}\nSource URL: https://{}", d, d));

    extract_posts_from_content(&combined_content, context.as_deref(), ai).await
}
