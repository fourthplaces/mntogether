//! Three-pass post extraction via structured LLM output
//!
//! Pass 1: Batch extract narrative posts (title + summary + comprehensive description)
//! Pass 2: Deduplicate and merge posts across batches
//! Pass 3: Agentic investigation - AI uses tools to find missing information
//!
//! This approach handles large content by batching, deduplicates across batches,
//! then enriches the unique posts with contact information.

use ai_client::{Agent, OpenAi, PromptBuilder};
use anyhow::Result;
use extraction::types::page::CachedPage;
use futures::future::join_all;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::common::{ExtractedPost, ExtractedPostInformation};
use crate::domains::tag::models::tag_kind_config::build_tag_instructions;
use crate::kernel::{FetchPageTool, ServerDeps, WebSearchTool, GPT_5_MINI};

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
    /// Primary audience: "participant", "volunteer", or "donor"
    pub audience: String,
}

/// Wrapper for narrative extraction response (for schema generation)
#[derive(Debug, Deserialize, JsonSchema)]
struct NarrativeExtractionResponse {
    posts: Vec<NarrativePost>,
}

/// System prompt for Pass 1: narrative extraction
const NARRATIVE_EXTRACTION_PROMPT: &str = r#"You are extracting posts for MN Together, a platform connecting communities around the immigration crisis in Minnesota.

## The Litmus Test

Only extract content that passes BOTH tests:
1. "Is this connected to immigrant communities and the current crisis?"
2. "Is this something someone can show up to, participate in, or contribute to?"

If BOTH yes → extract. Otherwise → skip.

## What to Extract

For each DISTINCT event, drive, or action you find, provide:

1. **title** - Action-focused, 5-10 words. Lead with the need or the action, not the org name. (e.g., "Deliver Groceries to Families Afraid to Leave Home", "Learn Your Rights Before ICE Comes to Your Door", "Keep Families Housed While They Figure Out What's Next"). Never include organization names in titles.
2. **summary** - 2-3 sentences (~250 chars). Lead with the human need or the moment, then the action. Make someone feel why this matters before telling them what to do.
   - Instead of: "Donate online to the emergency family support fund to provide food, rent assistance, and essential supplies for families affected by the immigration crisis in Minnesota."
   - Write: "Families are skipping meals and falling behind on rent because they're afraid to go to work. Your donation keeps them housed and fed while they navigate what's next."
   - Instead of: "Bring food and household supplies to support families in crisis. Drop off at 13798 Parkwood Drive, Burnsville during Mon/Tue 12–7, Fri 12–5, Sat 10–4."
   - Write: "Families can't risk a grocery run right now. Drop off rice, beans, diapers, or toiletries at 13798 Parkwood Drive in Burnsville—Mon/Tue 12–7, Fri 12–5, Sat 10–4."
   - Instead of: "Sign up to pack, load, or drive home deliveries for families in crisis."
   - Write: "Volunteers are packing and delivering groceries to families who can't leave home safely. Grab a shift at the Burnsville hub—no experience needed, just a photo ID."
3. **description** - A rich markdown description for someone who's ready to act but needs the details (see Writing the Description below)
4. **source_url** - The URL where this content was found (from the Source header above the content)
5. **audience** - Who this post is for: "participant" (attend/join events), "volunteer" (give time to help immigrants), or "donor" (give money/goods)

## What Qualifies

### Community Support Events
- Know-your-rights workshops and trainings
- Community meetings about immigration response
- Vigils, gatherings, and community support events connected to immigration
- Rallies and marches connected to immigration
- ICE rapid response trainings
- Sanctuary-related events
- Legal clinics or legal aid events (not standing office hours)

### Volunteer Opportunities
- Grocery/supply delivery to immigrant families afraid to leave home
- Accompaniment (escorting people to appointments, court, etc.)
- Supply packing or sorting for immigrant communities
- Rapid response team signups and trainings
- Sanctuary hosting
- Translation/interpretation at events

### Donation Drives
- Fundraisers for legal defense, bail funds, or family support
- Supply drives (food, clothing, hygiene items for immigrant families)
- Rent/housing emergency funds for families in crisis

## DO NOT Extract

- **Regular worship services** (Sunday mass, Bible study, prayer groups)
- **Standing services with regular hours** (food shelf open Mon-Fri, legal clinic every Tuesday) — UNLESS explicitly serving immigrant families in crisis (e.g., "delivery for families afraid to leave home")
- **Staff job postings** (hiring for the org)
- **Board governance** (meeting minutes, bylaws, annual reports)
- **"About Us" pages** (org history, mission statements, leadership bios)
- **Past event recaps** (only extract upcoming or ongoing events/drives)
- **Press releases** that aren't actionable events
- **Donor thank-yous / impact reports** (unless they contain a current donation drive)
- **Generic navigation content** ("Explore Our Events", "Learn About Our Programs")
- **General community programs** not connected to immigrant communities (after-school programs, senior fitness, arts classes, etc.)
- **Political events unrelated to immigration** (environmental protests, general labor actions, non-immigration policy issues)

If a page has no content connected to immigrant communities or the current crisis, extract NOTHING. Fewer high-quality posts is always better than noise.

## Minimum Quality Bar

A post MUST have:
- A clear connection to immigrant communities or the immigration crisis
- A specific way to participate (date/time, location, signup link, donation method)

If the content is vague, has no actionable details, or isn't connected to immigrant communities — **skip it entirely**.

Ask: "Would someone supporting immigrant neighbors find this relevant and know exactly what to do?" If no, skip.

## Writing the Description

Write for someone who's ready to act but needs the details. Structure it like this:

1. **Open with context** (1-2 sentences) — What's happening and why this matters right now
2. **The ask** — Exactly what someone can do
3. **Logistics** — Date, time, location (full address), what to bring, how to sign up
4. **Details that reduce friction** — Parking, accessibility, what to expect, who to contact

### Formatting
- **Bold** for critical details (dates, deadlines, addresses, requirements)
- Bullet lists only when listing multiple items (supplies needed, shift times)
- Short paragraphs — dense blocks of text feel like homework

### Tone Calibration
- Urgent but not panicked
- Specific but not bureaucratic
- Warm but not saccharine
- Assume good intent — people want to help, just make it easy

### Voice
Write like a neighbor telling another neighbor how they can help — not a nonprofit writing a grant report. Use active voice. Be direct.

### Avoid
- Nonprofit jargon ("wraparound services", "capacity building", "underserved communities")
- Passive voice ("donations are being accepted" → "we're collecting donations")
- Vague calls to action ("consider supporting" → "donate now" or "drop off supplies Saturday")
- Leading with the organization name — lead with the action or the need

## Splitting Posts

Create separate posts for:
- Different events (a rally on Saturday vs a workshop on Tuesday)
- Different programs (rapid response training vs accompaniment signup)
- A service with distinct roles: one post for people who NEED help (participant) and one for people who GIVE help (volunteer)

Do NOT split when:
- The same event serves both donor and participant roles (e.g., a fundraiser dinner, a benefit concert, a tattoo flash event where attending IS donating — this is ONE post, not two)
- The same action serves multiple purposes (e.g., "drop off supplies" is one post even if it helps families AND gives the donor a way to contribute)"#;

/// Pass 1: Extract narrative posts (title + summary + comprehensive description)
async fn extract_narrative_posts(
    content: &str,
    context: Option<&str>,
    ai: &OpenAi,
) -> Result<Vec<NarrativePost>> {
    if content.trim().is_empty() {
        return Ok(vec![]);
    }

    let system_prompt = match context {
        Some(ctx) => format!("{}\n\n{}", NARRATIVE_EXTRACTION_PROMPT, ctx),
        None => NARRATIVE_EXTRACTION_PROMPT.to_string(),
    };

    let user_prompt = format!("## Content to Extract\n\n{}", content);

    let response: NarrativeExtractionResponse = ai
        .extract(GPT_5_MINI, &system_prompt, &user_prompt)
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
- title: The best title (action-focused, 5-10 words, lead with the need or action, no org names)
- summary: 2-3 sentences (~250 chars). Lead with the human need or the moment, then the action. Write like a neighbor, not a nonprofit. Urgent but not panicked, specific but not bureaucratic.
- description: Merged description with ALL details from duplicates. Use active voice, avoid jargon.
- source_url: The primary source URL (or comma-separated if merged from multiple)

## CRITICAL: Preserve Markdown Formatting

The input descriptions contain rich markdown formatting. You MUST preserve this formatting:
- **Bold text** for key terms
- Bullet lists for multiple items
- Short paragraphs for readability
- Any links, headers, or other markdown

Do NOT strip formatting or convert to plain text. The output descriptions should be as well-formatted as the inputs.

Be aggressive about merging duplicates. Merge posts that describe the same event or opportunity even if worded for different audiences. Only keep posts separate when they describe genuinely different services or programs — not the same event described from different angles.

For example: "Get a Flash Tattoo to Feed Neighbors" and "Book a Tattoo to Keep Families Housed" are the SAME fundraiser event — merge them. But "Volunteer at the Food Shelf" and "Get Food at the Food Shelf" are different because they serve different roles — keep them separate."#;

/// Deduplicate and merge posts using LLM.
async fn dedupe_and_merge_posts(
    posts: Vec<NarrativePost>,
    domain: &str,
    ai: &OpenAi,
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

    let response: NarrativeExtractionResponse = ai
        .extract(GPT_5_MINI, DEDUPE_PROMPT, &user_prompt)
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
5. **Audience**: Who is this for (participant/volunteer/donor)
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

    // Step 1: Agent investigates with tools
    info!(title = %narrative.title, "Running agent with tools (web_search, fetch_page)");

    let agent = (*deps.ai)
        .clone()
        .tool(WebSearchTool::new(deps.web_searcher.clone()))
        .tool(FetchPageTool::new(
            deps.ingestor.clone(),
            deps.db_pool.clone(),
        ));

    let findings = agent
        .prompt(&user_message)
        .preamble(INVESTIGATION_PROMPT)
        .multi_turn(5)
        .send()
        .await?;

    info!(
        title = %narrative.title,
        findings_len = findings.len(),
        "Agent investigation complete"
    );

    debug!(
        title = %narrative.title,
        findings = %findings,
        "Full investigation findings"
    );

    // Step 2: Extract structured info from findings
    let extraction_input = format!(
        "Post Title: {}\n\nOriginal Description:\n{}\n\nInvestigation Findings:\n{}",
        narrative.title, narrative.description, findings
    );

    info!(
        title = %narrative.title,
        extraction_input_len = extraction_input.len(),
        "Extracting structured info from findings"
    );

    let extraction_prompt = build_extraction_prompt(tag_instructions);
    let result = deps
        .ai
        .extract::<ExtractedPostInformation>(GPT_5_MINI, &extraction_prompt, &extraction_input)
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
    let narratives = extract_narrative_posts(content, Some(&context), deps.ai.as_ref()).await?;

    if narratives.is_empty() {
        return Ok(vec![]);
    }

    info!(
        narratives_count = narratives.len(),
        domain = %domain,
        "Pass 1 complete: extracted narrative posts"
    );

    // Pass 2: Deduplicate and merge
    let deduplicated = dedupe_and_merge_posts(narratives, domain, deps.ai.as_ref()).await?;

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
    let ai = deps.ai.as_ref();
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

                let result = extract_narrative_posts(&combined_content, Some(&context), ai).await;

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
    let deduplicated = dedupe_and_merge_posts(all_narratives, domain, deps.ai.as_ref()).await?;

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
