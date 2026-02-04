//! Two-pass post extraction via structured LLM output
//!
//! Pass 1: Extract narrative posts (title + tldr + comprehensive description)
//! Pass 2: Agentic investigation - AI uses tools to find missing information
//!
//! This approach lets the LLM capture all information naturally in prose first,
//! then an agentic step enriches the data by researching missing info.

use anyhow::Result;
use extraction::types::page::CachedPage;
use futures::future::join_all;
use openai_client::OpenAIClient;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::common::{ContactInfo, ExtractedPost, ExtractedPostInformation};
use crate::kernel::{FetchPageTool, OpenAIExtractionService, ServerDeps, WebSearchTool};

//=============================================================================
// PASS 1: Narrative Extraction
//=============================================================================

/// Intermediate type from Pass 1: narrative content only
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NarrativePost {
    /// Clear, descriptive title
    pub title: String,
    /// One-sentence summary (max 100 chars)
    pub tldr: String,
    /// Comprehensive, human-readable description with ALL details
    pub description: String,
}

/// Wrapper for narrative extraction response (for schema generation)
#[derive(Debug, Deserialize, JsonSchema)]
struct NarrativeExtractionResponse {
    posts: Vec<NarrativePost>,
}

/// System prompt for Pass 1: narrative extraction
const NARRATIVE_EXTRACTION_PROMPT: &str = r#"You are extracting community resources from website content.

For each DISTINCT opportunity, service, program, or event you find, provide:

1. **title** - A clear, descriptive title
2. **tldr** - One sentence (max 100 chars) that captures the essence
3. **description** - A well-written description for humans to read

## Writing the Description

Write in clear, organized prose that's easy to scan. Include:
- What this is and who it's for
- Location and address (if mentioned)
- Hours, dates, schedules (if mentioned)
- How to access, apply, or sign up
- Contact information (phone, email, website)
- Eligibility or requirements
- Any other helpful details

Guidelines:
- Write for someone who will actually use this information
- Use short paragraphs, not walls of text
- Be comprehensive but readable - don't omit details, but organize them well
- Natural language, not bullet dumps
- Capture EVERYTHING mentioned - location, hours, contact info, eligibility
- Put it ALL in the description

## What Constitutes a Separate Post

Create separate posts when content describes:
- Different services (e.g., "Food Shelf" vs "Clothing Closet")
- Different audiences for the SAME service (e.g., "Food Shelf - Get Help" vs "Food Shelf - Volunteer")
- Different events (e.g., "Monthly Food Drive" vs "Annual Gala")
- Different programs (e.g., "Senior Services" vs "Youth Services")"#;

/// Pass 1: Extract narrative posts (title + tldr + comprehensive description)
async fn extract_narrative_posts(content: &str, context: Option<&str>) -> Result<Vec<NarrativePost>> {
    if content.trim().is_empty() {
        return Ok(vec![]);
    }

    let system_prompt = match context {
        Some(ctx) => format!("{}\n\n{}", NARRATIVE_EXTRACTION_PROMPT, ctx),
        None => NARRATIVE_EXTRACTION_PROMPT.to_string(),
    };

    let user_prompt = format!("## Content to Extract\n\n{}", content);

    let client = OpenAIClient::from_env()?;
    let response: NarrativeExtractionResponse = client
        .extract("gpt-4o", &system_prompt, &user_prompt)
        .await
        .map_err(|e| anyhow::anyhow!("Narrative extraction failed: {}", e))?;

    Ok(response.posts)
}

//=============================================================================
// PASS 2: Agentic Investigation
//=============================================================================

/// System prompt for Pass 2: agentic investigation
const INVESTIGATION_PROMPT: &str = r#"You are investigating a community resource post to find missing information.

Given the post title and description, extract structured information. If important details are missing (especially contact info), use the available tools to find them.

## Tools Available
- **web_search**: Search for information about the organization
- **fetch_page**: Get content from a specific URL (like a contact page)

## What to Find
1. **Contact Information**: phone, email, website, intake forms, contact person
2. **Location**: Physical address if this is an in-person service
3. **Urgency**: How time-sensitive is this? (low/medium/high/urgent)
4. **Confidence**: How confident are you in the information? (low/medium/high)
5. **Audience**: Who is this for? (recipient/volunteer/donor/participant)

## Guidelines
- First check if the description already contains the information
- Only use tools if something important is missing
- For contact info, try searching "[organization name] contact" or fetching their /contact page
- Be conservative - only include information you're confident about

When you have gathered enough information, respond with your findings."#;

/// System prompt for extracting structured info from investigation findings.
const EXTRACTION_PROMPT: &str = r#"Extract structured information from the investigation findings.

For each field:
- **contact**: Phone, email, website, intake_form_url, contact_name (leave null if not found)
- **location**: Physical address if this is an in-person service (null if virtual/not mentioned)
- **urgency**: "low", "medium", "high", or "urgent" based on time-sensitivity
- **confidence**: "low", "medium", or "high" based on information completeness
- **audience_roles**: Array of who this is for: "recipient", "volunteer", "donor", "participant"

Be conservative - only include information explicitly mentioned."#;

/// Investigate a single post to find missing information.
///
/// Uses AI agent with tools to research, then structured extraction for the result.
async fn investigate_post(
    narrative: &NarrativePost,
    domain: &str,
    deps: &ServerDeps,
) -> Result<ExtractedPostInformation> {
    let user_message = format!(
        "Source domain: {}\n\nTitle: {}\n\nDescription:\n{}",
        domain, narrative.title, narrative.description
    );

    debug!(title = %narrative.title, domain = %domain, "Investigating post");

    let client = OpenAIClient::from_env()?;

    // Step 1: Agent investigates with tools
    let response = client
        .agent("gpt-4o")
        .system(INVESTIGATION_PROMPT)
        .tool(WebSearchTool::new(deps.web_searcher.clone()))
        .tool(FetchPageTool::new(deps.ingestor.clone()))
        .max_iterations(5)
        .build()
        .chat(&user_message)
        .await?;

    debug!(
        title = %narrative.title,
        tool_calls = ?response.tool_calls_made,
        iterations = response.iterations,
        "Investigation complete"
    );

    // Step 2: Extract structured info from findings
    let extraction_input = format!(
        "Post Title: {}\n\nOriginal Description:\n{}\n\nInvestigation Findings:\n{}",
        narrative.title, narrative.description, response.content
    );

    client
        .extract::<ExtractedPostInformation>("gpt-4o", EXTRACTION_PROMPT, &extraction_input)
        .await
        .map_err(|e| anyhow::anyhow!("Structured extraction failed: {}", e))
}

//=============================================================================
// Main Entry Points
//=============================================================================

/// Simple search query for finding pages with community listings.
/// Used for semantic search - keep it short and topical.
pub const POST_SEARCH_QUERY: &str =
    "volunteer opportunities services programs events donations community resources help";

/// Extract structured posts from markdown content using two-pass extraction.
///
/// Pass 1: Extract narrative posts (title + tldr + comprehensive description)
/// Pass 2: Agentic investigation to find missing information
///
/// # Arguments
/// * `content` - Combined markdown content from extraction library
/// * `domain` - Source domain for context and investigation
/// * `deps` - Server dependencies
///
/// # Returns
/// Vector of extracted posts, empty if extraction fails or produces no results.
pub async fn extract_posts_from_content(
    content: &str,
    domain: &str,
    deps: &ServerDeps,
) -> Result<Vec<ExtractedPost>> {
    if content.trim().is_empty() {
        return Ok(vec![]);
    }

    let context = format!("Organization: {}\nSource URL: https://{}", domain, domain);

    // Pass 1: Extract narrative posts (title + tldr + description)
    let narratives = extract_narrative_posts(content, Some(&context)).await?;

    if narratives.is_empty() {
        return Ok(vec![]);
    }

    info!(
        narratives_count = narratives.len(),
        domain = %domain,
        "Pass 1 complete: extracted narrative posts"
    );

    // Pass 2: Investigate each post in parallel
    let investigation_futures: Vec<_> = narratives
        .iter()
        .map(|n| investigate_post(n, domain, deps))
        .collect();

    let investigation_results = join_all(investigation_futures).await;

    // Combine narratives with investigation results
    let mut posts = Vec::new();
    for (narrative, info_result) in narratives.into_iter().zip(investigation_results) {
        let info = match info_result {
            Ok(i) => i,
            Err(e) => {
                warn!(
                    title = %narrative.title,
                    error = %e,
                    "Investigation failed, using defaults"
                );
                ExtractedPostInformation {
                    contact: ContactInfo::default(),
                    location: None,
                    urgency: "medium".to_string(),
                    confidence: "low".to_string(),
                    audience_roles: vec!["recipient".to_string()],
                }
            }
        };

        posts.push(ExtractedPost {
            title: narrative.title,
            tldr: narrative.tldr,
            description: narrative.description,
            contact: info.contact_or_none(),
            location: info.location,
            urgency: Some(info.urgency),
            confidence: Some(info.confidence),
            audience_roles: info.audience_roles,
            source_page_snapshot_id: None,
        });
    }

    info!(
        posts_count = posts.len(),
        domain = %domain,
        "Pass 2 complete: investigation finished"
    );

    Ok(posts)
}

/// Result of extracting posts for a domain.
#[derive(Debug)]
pub struct DomainExtractionResult {
    /// Extracted posts
    pub posts: Vec<ExtractedPost>,
    /// URLs of pages that were searched
    pub page_urls: Vec<String>,
}

/// Search for pages and extract posts for a domain with agentic investigation.
///
/// This function:
/// 1. Searches for relevant pages using semantic search
/// 2. Combines raw page content
/// 3. Extracts structured posts via LLM
/// 4. Uses AI tools to investigate and fill in missing information
pub async fn extract_posts_for_domain(
    domain: &str,
    extraction: &OpenAIExtractionService,
    deps: &ServerDeps,
) -> Result<DomainExtractionResult> {
    // Search for relevant pages
    let pages = extraction
        .search_and_get_pages(POST_SEARCH_QUERY, Some(domain), 50)
        .await?;

    info!(
        domain = %domain,
        pages_found = pages.len(),
        page_urls = ?pages.iter().map(|p| &p.url).collect::<Vec<_>>(),
        "Search results (with investigation)"
    );

    if pages.is_empty() {
        return Ok(DomainExtractionResult {
            posts: vec![],
            page_urls: vec![],
        });
    }

    let page_urls: Vec<String> = pages.iter().map(|p| p.url.clone()).collect();

    // Combine and extract with investigation
    let posts = extract_posts_from_pages(&pages, domain, deps).await?;

    Ok(DomainExtractionResult { posts, page_urls })
}

/// Extract posts from a set of pages with agentic investigation.
pub async fn extract_posts_from_pages(
    pages: &[CachedPage],
    domain: &str,
    deps: &ServerDeps,
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
        domain = %domain,
        "Extracting structured posts from raw content"
    );

    extract_posts_from_content(&combined_content, domain, deps).await
}
