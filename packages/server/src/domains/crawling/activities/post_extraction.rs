//! Three-pass post extraction via structured LLM output
//!
//! Pass 1: Batch extract narrative posts (title + summary + comprehensive description)
//! Pass 2: Deduplicate and merge posts across batches
//! Pass 3: Agentic investigation - AI uses tools to find missing information
//!
//! This approach handles large content by batching, deduplicates across batches,
//! then enriches the unique posts with contact information.

use anyhow::Result;
use extraction::types::page::CachedPage;
use futures::future::join_all;
use openai_client::OpenAIClient;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::common::{ExtractedPost, ExtractedPostInformation};
use crate::domains::tag::models::tag_kind_config::build_tag_instructions;
use crate::kernel::{FetchPageTool, ServerDeps, WebSearchTool};

//=============================================================================
// BATCHING
//=============================================================================

/// Maximum content size per batch (~75K tokens worth, leaving room for prompts/response).
/// OpenAI's gpt-4o has 128K token limit. At ~4 chars/token, 300K chars ≈ 75K tokens.
const MAX_CONTENT_CHARS_PER_BATCH: usize = 100_000;

/// Batch pages into groups that fit within the content size limit.
fn batch_pages_by_size(pages: &[CachedPage], max_chars: usize) -> Vec<Vec<&CachedPage>> {
    let mut batches = Vec::new();
    let mut current_batch = Vec::new();
    let mut current_size = 0;

    for page in pages {
        // Account for formatting overhead: "## Source: {url}\n\n{content}\n\n---\n\n"
        let page_size = page.url.len() + page.content.len() + 30;

        // If adding this page would exceed limit and batch isn't empty, start new batch
        if current_size + page_size > max_chars && !current_batch.is_empty() {
            batches.push(current_batch);
            current_batch = Vec::new();
            current_size = 0;
        }

        current_batch.push(page);
        current_size += page_size;
    }

    if !current_batch.is_empty() {
        batches.push(current_batch);
    }

    batches
}

//=============================================================================
// PASS 1: Narrative Extraction
//=============================================================================

/// Intermediate type from Pass 1: narrative content only
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NarrativePost {
    /// Clear, descriptive title
    pub title: String,
    /// 2-3 sentence summary (~250 chars) with key details for card previews
    pub summary: String,
    /// Comprehensive, human-readable description with ALL details
    pub description: String,
    /// The source URL where this post was found
    pub source_url: String,
    /// Primary audience: "recipient", "volunteer", "donor", or "participant"
    pub audience: String,
}

/// Wrapper for narrative extraction response (for schema generation)
#[derive(Debug, Deserialize, JsonSchema)]
struct NarrativeExtractionResponse {
    posts: Vec<NarrativePost>,
}

/// System prompt for Pass 1: narrative extraction
const NARRATIVE_EXTRACTION_PROMPT: &str = r#"You are extracting community resources from website content.

For each DISTINCT opportunity, service, program, or event you find, provide:

1. **title** - An action-focused title that tells people exactly what they can DO. Lead with the action, not the organization. (e.g., "Get Free Hot Meals Every Tuesday", "Sort and Pack Food Boxes", "Donate Food or Funds"). Never include organization names in titles - that info is captured elsewhere.
2. **summary** - 2-3 sentence summary (~250 chars) that includes the most important actionable details: when/where it happens, key requirements, and how to get involved. Don't just say what it is — include the details someone needs to decide whether to click.
3. **description** - A rich markdown description for humans to read
4. **source_url** - The URL where this content was found (look at the Source header above the content)
5. **audience** - Who this post is for: "recipient" (people who receive help), "volunteer" (people who give time), "donor" (people who give money/goods), or "participant" (general participants)

## Writing the Description

Write in well-formatted markdown that's easy to scan. Use:
- **Bold** for key terms (eligibility, deadlines, requirements)
- Bullet lists for multiple items (hours, services offered, eligibility criteria)
- Short paragraphs for narrative context

Include all relevant details:
- What this is and who it's for
- Location and address — REQUIRED for in-person services (full street address, city, state, zip). Skip for virtual-only.
- Schedule — REQUIRED for events and recurring programs (day, time, frequency). Skip for always-available services.
- Contact information — phone, email, website, or signup form. Note the gap explicitly if missing.
- Eligibility or requirements
- How to access, apply, or sign up

Guidelines:
- Use markdown formatting liberally - bold, bullets, headers if appropriate
- Be comprehensive and well-organized
- Capture EVERYTHING mentioned - location, hours, contact info, eligibility
- ALWAYS include the source_url from the Source header above each content section

## CRITICAL: Only Extract SPECIFIC Opportunities

Only create posts for CONCRETE, SPECIFIC opportunities that someone can actually act on.

**DO extract:**
- "Free Tax Preparation Help - Saturdays in February" (specific service with timing)
- "Community Meal - Every Wednesday 5:30pm" (specific recurring event)
- "Emergency Shelter Beds Available" (specific service)
- "Youth Soccer League Registration Open" (specific program)

**DO NOT extract:**
- "Explore Our Events" (too vague - no specific event)
- "Learn About Our Programs" (meta-content, not a program itself)
- "Visit Our Website" (not actionable)
- "Check Our Calendar" (pointer to content, not content itself)
- "Contact Us For More Information" (generic, not a specific opportunity)

If a page only contains navigation or generic "learn more" content without specific details, extract NOTHING from that page. It's better to have fewer, high-quality posts than many vague ones.

## CRITICAL: Split by Audience

**ALWAYS create separate posts for each audience type.** A single page often describes multiple ways to engage:

- **Recipients**: People who RECEIVE help (get food, get assistance, access services)
- **Volunteers**: People who GIVE time (sort food, deliver boxes, help at events)
- **Donors**: People who GIVE money or goods (donate food, contribute funds)

If a page says "Get food here" AND "Volunteer to help" AND "Donate to support us" - that is THREE separate posts:
1. "Get Free Food Boxes" (audience: recipient)
2. "Sort and Pack Food Boxes" (audience: volunteer)
3. "Donate Food or Funds" (audience: donor)

Each post should have:
- An action-focused title (what can I DO?) - no organization names
- Description focused on THAT audience's needs and actions
- The specific contact info for THAT action (e.g., volunteer signup form, donation link, food registration)

## Other Reasons to Split Posts

Also create separate posts for:
- Different services (e.g., Food Shelf vs Clothing Closet)
- Different events (e.g., Monthly Food Drive vs Annual Gala)
- Different programs (e.g., Senior Services vs Youth Services)"#;

/// Pass 1: Extract narrative posts (title + summary + comprehensive description)
async fn extract_narrative_posts(
    content: &str,
    context: Option<&str>,
) -> Result<Vec<NarrativePost>> {
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
// PASS 2: Deduplication & Merge
//=============================================================================

/// System prompt for deduplicating and merging posts
const DEDUPE_PROMPT: &str = r#"You are consolidating a list of community resource posts that may contain duplicates.

Posts are duplicates if they describe the SAME opportunity, service, or program - even if:
- Titles are worded differently ("Get Free Food" vs "Food Assistance Program")
- Descriptions have different levels of detail
- They came from different pages on the same website

## Your Task

1. Identify posts that describe the same thing
2. Merge duplicates into a single, best version:
   - Use the clearest, most action-focused title
   - Combine information from all versions into the most complete description
   - Keep all unique source_urls (comma-separate if multiple)
3. Keep distinct posts separate (different services, audiences, programs)

## Output

Return the deduplicated list of posts. Each post should have:
- title: The best title (action-focused, no org names)
- summary: 2-3 sentence summary (~250 chars) with key actionable details (when, where, requirements, how to access)
- description: Merged description with ALL details from duplicates
- source_url: The primary source URL (or comma-separated if merged from multiple)

## CRITICAL: Preserve Markdown Formatting

The input descriptions contain rich markdown formatting. You MUST preserve this formatting:
- **Bold text** for key terms
- Bullet lists for multiple items
- Short paragraphs for readability
- Any links, headers, or other markdown

Do NOT strip formatting or convert to plain text. The output descriptions should be as well-formatted as the inputs.

Be aggressive about merging duplicates, but never merge posts that serve different audiences (recipient vs volunteer vs donor) or different services."#;

/// Deduplicate and merge posts using LLM.
async fn dedupe_and_merge_posts(
    posts: Vec<NarrativePost>,
    domain: &str,
) -> Result<Vec<NarrativePost>> {
    if posts.len() <= 1 {
        return Ok(posts);
    }

    // Format posts for the LLM
    let posts_json = serde_json::to_string_pretty(&posts)?;
    let user_prompt = format!(
        "Organization: {}\n\n## Posts to Deduplicate\n\n{}",
        domain, posts_json
    );

    info!(
        input_count = posts.len(),
        domain = %domain,
        "Deduplicating posts"
    );

    let client = OpenAIClient::from_env()?;
    let response: NarrativeExtractionResponse = client
        .extract("gpt-4o", DEDUPE_PROMPT, &user_prompt)
        .await
        .map_err(|e| anyhow::anyhow!("Deduplication failed: {}", e))?;

    info!(
        input_count = posts.len(),
        output_count = response.posts.len(),
        domain = %domain,
        "Deduplication complete"
    );

    Ok(response.posts)
}

//=============================================================================
// PASS 3: Agentic Investigation
//=============================================================================

/// System prompt for Pass 3: agentic investigation
const INVESTIGATION_PROMPT: &str = r#"You are investigating a community resource post to find contact information so people can take action.

## What Counts as Contact Information

Contact information is ANY way for someone to reach out or take action:
- **Signup/intake forms** (volunteer forms, application forms, registration links)
- **Email addresses**
- **Phone numbers**
- **Physical addresses** (for in-person services)
- **Website URLs** with clear next steps

A signup form URL IS valid contact information. If the description contains a form link, that's the primary contact method.

## Your Task (REQUIRED - follow this order)

1. **FIRST**: Check if the description already contains contact info (forms, emails, phones, addresses)
2. **THEN**: Use fetch_page on the SOURCE URL to explore that page for contact links
3. **NEXT**: Try fetch_page on common contact pages:
   - Replace the path with /contact, /contact-us, /about, /get-involved
4. **IF STILL MISSING**: Use web_search for "{organization name} contact phone email address"

## Tools Available
- **fetch_page**: Get content from a URL - USE THIS to explore the source website
- **web_search**: Search the web for organization information

## What to Extract
1. **Contact Information** (REQUIRED): The PRIMARY way to take action - form URL, email, phone, or website
2. **Location**: Physical address if this is an in-person service
3. **Urgency**: How time-sensitive (low/medium/high/urgent)
4. **Confidence**: high if form/email/phone found, medium if only website, low if nothing found
5. **Audience**: Who is this for (recipient/volunteer/donor/participant)
6. **Schedule**: For events/recurring programs: dates, times, and frequency.

## Guidelines
- A signup form link in the description IS the contact method - report it!
- ALWAYS try fetch_page on the source URL first - this is the most reliable source
- Do NOT give up after one failed attempt - try multiple strategies
- Set confidence based on how actionable the contact info is

Respond with your findings including all contact information you found."#;

/// Build the structured extraction prompt with dynamic tag instructions.
fn build_extraction_prompt(tag_instructions: &str) -> String {
    let tag_section = if tag_instructions.is_empty() {
        String::new()
    } else {
        format!(
            "\n- **tags**: Object with tag classifications:\n{}",
            tag_instructions
        )
    };

    format!(
        r#"Extract structured information from the investigation findings.

For each field:
- **contact**: Phone, email, website, intake_form_url, contact_name (leave null if not found)
- **location**: Physical address if this is an in-person service (null if virtual/not mentioned)
- **zip_code**: 5-digit zip code for in-person services (null if virtual/unknown)
- **city**: City name (e.g., "Minneapolis")
- **state**: 2-letter state abbreviation (e.g., "MN")
- **urgency**: "low", "medium", "high", or "urgent" based on time-sensitivity
- **confidence**: "low", "medium", or "high" based on information completeness
- **schedule**: Array of schedule entries. For each recurring or one-off schedule mentioned, extract:
  - **frequency**: "weekly", "biweekly", "monthly", or "one_time"
  - **day_of_week**: Lowercase day name ("monday", "tuesday", etc.) — required for weekly/biweekly/monthly
  - **start_time**: Start time in 24h "HH:MM" format (e.g., "17:00")
  - **end_time**: End time in 24h "HH:MM" format (e.g., "19:00")
  - **date**: Specific date "YYYY-MM-DD" — for one_time events only
  - **notes**: Freeform notes (e.g., "1st and 3rd week only", "by appointment")
  Only include schedule entries for events/programs with specific day/time info. If no schedule is mentioned, return an empty array.{}

Be conservative - only include information explicitly mentioned."#,
        tag_section
    )
}

/// Investigate a single post to find missing information.
///
/// Uses AI agent with tools to research, then structured extraction for the result.
pub async fn investigate_post(
    narrative: &NarrativePost,
    tag_instructions: &str,
    deps: &ServerDeps,
) -> Result<ExtractedPostInformation> {
    let user_message = format!(
        "Source URL: {}\n\nTitle: {}\n\nDescription:\n{}",
        narrative.source_url, narrative.title, narrative.description
    );

    info!(
        title = %narrative.title,
        source_url = %narrative.source_url,
        description_len = narrative.description.len(),
        "Starting post investigation"
    );

    let client = OpenAIClient::from_env()?;

    // Step 1: Agent investigates with tools
    info!(title = %narrative.title, "Running agent with tools (web_search, fetch_page)");

    let response = client
        .agent("gpt-4o")
        .system(INVESTIGATION_PROMPT)
        .tool(WebSearchTool::new(deps.web_searcher.clone()))
        .tool(FetchPageTool::new(deps.ingestor.clone(), deps.db_pool.clone()))
        .max_iterations(5)
        .build()
        .chat(&user_message)
        .await?;

    info!(
        title = %narrative.title,
        tool_calls_made = ?response.tool_calls_made,
        iterations = response.iterations,
        findings_len = response.content.len(),
        "Agent investigation complete"
    );

    debug!(
        title = %narrative.title,
        findings = %response.content,
        "Full investigation findings"
    );

    // Step 2: Extract structured info from findings
    let extraction_input = format!(
        "Post Title: {}\n\nOriginal Description:\n{}\n\nInvestigation Findings:\n{}",
        narrative.title, narrative.description, response.content
    );

    info!(
        title = %narrative.title,
        extraction_input_len = extraction_input.len(),
        "Extracting structured info from findings"
    );

    let extraction_prompt = build_extraction_prompt(tag_instructions);
    let result = client
        .extract::<ExtractedPostInformation>("gpt-4o", &extraction_prompt, &extraction_input)
        .await
        .map_err(|e| anyhow::anyhow!("Structured extraction failed: {}", e))?;

    info!(
        title = %narrative.title,
        has_phone = result.contact.phone.is_some(),
        has_email = result.contact.email.is_some(),
        has_website = result.contact.website.is_some(),
        has_contact_name = result.contact.contact_name.is_some(),
        has_location = result.location.is_some(),
        urgency = %result.urgency,
        confidence = %result.confidence,
        "Structured extraction complete"
    );

    Ok(result)
}

//=============================================================================
// Main Entry Points
//=============================================================================

/// Extract structured posts from markdown content using three-pass extraction.
///
/// Pass 1: Extract narrative posts (title + summary + comprehensive description)
/// Pass 2: Deduplicate and merge posts
/// Pass 3: Agentic investigation to find missing information
///
/// Note: For multiple pages, prefer `extract_posts_from_pages` which handles batching.
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

    // Build dynamic tag instructions once for all investigations
    let tag_instructions = build_tag_instructions(&deps.db_pool)
        .await
        .unwrap_or_default();

    let context = format!("Organization: {}\nSource URL: https://{}", domain, domain);

    // Pass 1: Extract narrative posts (title + summary + description)
    let narratives = extract_narrative_posts(content, Some(&context)).await?;

    if narratives.is_empty() {
        return Ok(vec![]);
    }

    info!(
        narratives_count = narratives.len(),
        domain = %domain,
        "Pass 1 complete: extracted narrative posts"
    );

    // Pass 2: Deduplicate and merge
    let deduplicated = dedupe_and_merge_posts(narratives, domain).await?;

    info!(
        deduplicated_count = deduplicated.len(),
        domain = %domain,
        "Pass 2 complete: deduplication finished"
    );

    // Pass 3: Investigate each post in parallel
    let investigation_futures: Vec<_> = deduplicated
        .iter()
        .map(|n| investigate_post(n, &tag_instructions, deps))
        .collect();

    let investigation_results = join_all(investigation_futures).await;

    // Combine narratives with investigation results
    let mut posts = Vec::new();
    for (narrative, info_result) in deduplicated.into_iter().zip(investigation_results) {
        let info = match info_result {
            Ok(i) => i,
            Err(e) => {
                warn!(
                    title = %narrative.title,
                    error = %e,
                    "Investigation failed, using defaults"
                );
                ExtractedPostInformation::default()
            }
        };

        posts.push(ExtractedPost::from_narrative_and_info(narrative, info));
    }

    info!(
        posts_count = posts.len(),
        domain = %domain,
        "Pass 3 complete: investigation finished"
    );

    Ok(posts)
}

/// Extract posts from a set of pages with batching, deduplication, and agentic investigation.
///
/// Flow: batch extract → dedupe & merge → enrich
pub async fn extract_posts_from_pages(
    pages: &[CachedPage],
    domain: &str,
    deps: &ServerDeps,
) -> Result<Vec<ExtractedPost>> {
    // Build dynamic tag instructions from all post-applicable tag kinds
    let tag_instructions = build_tag_instructions(&deps.db_pool)
        .await
        .unwrap_or_default();

    extract_posts_from_pages_with_tags(pages, domain, &tag_instructions, deps).await
}

/// Extract posts from pages with custom tag instructions.
///
/// Use this when you have specific tag kinds instead of all tag kinds.
/// Pass empty string for tag_instructions to skip tag extraction entirely.
pub async fn extract_posts_from_pages_with_tags(
    pages: &[CachedPage],
    domain: &str,
    tag_instructions: &str,
    deps: &ServerDeps,
) -> Result<Vec<ExtractedPost>> {
    if pages.is_empty() {
        return Ok(vec![]);
    }

    let tag_instructions = tag_instructions.to_string();

    let context = format!("Organization: {}\nSource URL: https://{}", domain, domain);

    // Step 1: Batch pages by content size
    let batches = batch_pages_by_size(pages, MAX_CONTENT_CHARS_PER_BATCH);

    info!(
        pages_count = pages.len(),
        batch_count = batches.len(),
        domain = %domain,
        "Processing pages in batches"
    );

    // Step 2: Extract narratives from each batch (in parallel)
    let batch_futures: Vec<_> = batches
        .iter()
        .enumerate()
        .map(|(batch_idx, batch)| {
            let combined_content: String = batch
                .iter()
                .map(|p| format!("## Source: {}\n\n{}", p.url, p.content))
                .collect::<Vec<_>>()
                .join("\n\n---\n\n");

            let context = context.clone();
            let content_len = combined_content.len();
            let batch_pages = batch.len();

            async move {
                info!(
                    batch = batch_idx + 1,
                    batch_pages = batch_pages,
                    content_len = content_len,
                    "Extracting narratives from batch"
                );

                let result = extract_narrative_posts(&combined_content, Some(&context)).await;

                match &result {
                    Ok(narratives) => {
                        info!(
                            batch = batch_idx + 1,
                            narratives_count = narratives.len(),
                            "Batch extraction complete"
                        );
                    }
                    Err(e) => {
                        warn!(
                            batch = batch_idx + 1,
                            error = %e,
                            "Batch extraction failed"
                        );
                    }
                }

                result
            }
        })
        .collect();

    let batch_results = join_all(batch_futures).await;

    let all_narratives: Vec<NarrativePost> = batch_results
        .into_iter()
        .filter_map(|r| r.ok())
        .flatten()
        .collect();

    if all_narratives.is_empty() {
        return Ok(vec![]);
    }

    info!(
        total_narratives = all_narratives.len(),
        domain = %domain,
        "Pass 1 complete: all batches extracted"
    );

    // Step 3: Deduplicate and merge posts
    let deduplicated = dedupe_and_merge_posts(all_narratives, domain).await?;

    info!(
        deduplicated_count = deduplicated.len(),
        domain = %domain,
        "Pass 2 complete: deduplication finished"
    );

    // Step 4: Enrich each unique post with investigation
    let investigation_futures: Vec<_> = deduplicated
        .iter()
        .map(|n| investigate_post(n, &tag_instructions, deps))
        .collect();

    let investigation_results = join_all(investigation_futures).await;

    // Combine narratives with investigation results
    let mut posts = Vec::new();
    for (narrative, info_result) in deduplicated.into_iter().zip(investigation_results) {
        let info = match info_result {
            Ok(i) => i,
            Err(e) => {
                warn!(
                    title = %narrative.title,
                    error = %e,
                    "Investigation failed, using defaults"
                );
                ExtractedPostInformation::default()
            }
        };

        posts.push(ExtractedPost::from_narrative_and_info(narrative, info));
    }

    info!(
        posts_count = posts.len(),
        domain = %domain,
        "Pass 3 complete: investigation finished"
    );

    Ok(posts)
}
