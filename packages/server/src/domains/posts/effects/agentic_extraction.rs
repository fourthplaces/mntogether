//! Agentic Post Extraction
//!
//! Instead of one-shot "extract everything", this uses an agent loop:
//! 1. Find post candidates (lightweight - just titles/types)
//! 2. For each candidate, use tools to enrich with full details
//! 3. Merge/dedupe posts across pages
//! 4. Store in page_extractions table
//!
//! The LLM "goes looking" for missing information using tools,
//! resulting in richer, more complete posts.
//!
//! # File Structure
//!
//! This file contains multiple logical modules that should be split for maintainability:
//!
//! | Lines | Section | Future Module |
//! |-------|---------|---------------|
//! | 32-153 | Data Structures | `types.rs` |
//! | 154-352 | Tool Definitions & Prompts | `tools.rs` |
//! | 353-705 | Candidate Extraction & Enrichment | `extraction.rs` |
//! | 706-982 | Tool Execution Loop | `tool_loop.rs` |
//! | 983-1145 | Post Merging & Dedup | `merging.rs` |
//! | 1146-1400 | Pipeline (extract_from_page/website) | `pipeline.rs` |
//! | 1401-1573 | Storage & Sync | `storage.rs` |
//! | 1580-1670 | Conversions | `conversions.rs` |
//!
//! # Architecture Note
//!
//! Despite being in `effects/`, this module contains **action-style** functions:
//! - Take deps explicitly (not EffectContext)
//! - Return simple values (not emit events)
//! - Can be unit tested independently
//!
//! The main entry points (`extract_from_website`, `extract_from_page`) are called
//! by effect handlers and return results that handlers convert to events.
//!
//! # Deprecation Note
//!
//! This module uses `PageSnapshotId` from the deprecated `page_snapshots` schema.
//! Future refactoring should use extraction library's URL-based page references.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use crate::common::{
    CallToAction, ContactInfo, EligibilityInfo, ExtractedPost, LocationInfo, ScheduleInfo,
    WebsiteId,
};
use crate::domains::crawling::models::{PageExtraction, PageSnapshotId};
use crate::domains::posts::models::Post;
use crate::kernel::BaseAI;

use extraction::WebSearcher;

// Import extraction_tools for SharedEnrichmentData
use super::extraction_tools::*;

// Note: rig-based enrichment tools are defined in enrichment_tools.rs
// but not used yet due to rig-core 0.9.1 lacking built-in multi-turn support.
// See the comment in the "Rig-based Enrichment - FUTURE USE" section below.

// =============================================================================
// Data Structures
// =============================================================================

/// A lightweight post candidate (first pass extraction)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostCandidate {
    pub title: String,
    pub post_type: String, // "volunteer", "service", "event", "donation"
    pub brief_description: String,
    /// Where in the page this was found (for tool context)
    pub source_excerpt: String,
}

/// A fully enriched post with all available details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedPost {
    // Core identity
    pub title: String,
    pub post_type: String,
    pub description: String,

    // Contact & Action
    pub contact: Option<ContactInfo>,
    pub call_to_action: Option<CallToAction>,

    // Location & Schedule
    pub location: Option<LocationInfo>,
    pub schedule: Option<ScheduleInfo>,

    // Eligibility & Requirements
    pub eligibility: Option<EligibilityInfo>,

    // Metadata
    pub source_url: Option<String>,
    pub source_page_snapshot_id: Option<Uuid>,
    pub confidence: f32,
    pub enrichment_notes: Vec<String>,
}

// ContactInfo, CallToAction, LocationInfo, ScheduleInfo, EligibilityInfo
// are now imported from crate::common (unified types)

/// Context for enrichment - includes other pages and search capability
pub struct EnrichmentContext<'a> {
    pub page_content: &'a str,
    pub page_url: &'a str,
    pub other_pages: &'a HashMap<String, String>, // url -> content
    pub web_searcher: Option<&'a dyn WebSearcher>,
    pub ai: &'a dyn BaseAI,
}

/// Result of website-wide extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteExtractionResult {
    pub posts: Vec<EnrichedPost>,
    pub pages_processed: usize,
    pub candidates_found: usize,
    pub candidates_skipped: usize, // Already existed in DB
    pub posts_merged: usize,
}

/// Tracks which posts already exist in the database
#[derive(Debug)]
pub struct ExistingPosts {
    /// Map of normalized title -> existing post
    by_title: HashMap<String, Post>,
    /// Set of existing post IDs for quick lookup
    ids: HashSet<Uuid>,
}

impl ExistingPosts {
    /// Load existing posts for a website
    pub async fn load(website_id: WebsiteId, pool: &PgPool) -> Result<Self> {
        let posts = Post::find_active_by_website(website_id, pool).await?;

        let by_title: HashMap<String, Post> = posts
            .iter()
            .map(|p| (normalize_title(&p.title), p.clone()))
            .collect();

        let ids: HashSet<Uuid> = posts.iter().map(|p| p.id.into_uuid()).collect();

        info!(
            website_id = %website_id,
            existing_posts = posts.len(),
            "Loaded existing posts for deduplication"
        );

        Ok(Self { by_title, ids })
    }

    /// Check if a post with similar title already exists
    pub fn find_match(&self, title: &str) -> Option<&Post> {
        let normalized = normalize_title(title);
        self.by_title.get(&normalized)
    }

    /// Check if we already have this exact post
    pub fn has_id(&self, id: Uuid) -> bool {
        self.ids.contains(&id)
    }

    /// Get count of existing posts
    pub fn len(&self) -> usize {
        self.by_title.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_title.is_empty()
    }
}

/// Normalize title for matching (lowercase, trim, collapse whitespace)
fn normalize_title(title: &str) -> String {
    title
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// =============================================================================
// Tool Definitions
// =============================================================================

pub fn get_enrichment_tools() -> serde_json::Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "find_contact_info",
                "description": "Search the page content to find contact information for this specific post/opportunity. Look for phone numbers, emails, contact forms, and intake form URLs.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "phone": {"type": "string", "description": "Phone number found (format: xxx-xxx-xxxx)"},
                        "email": {"type": "string", "description": "Email address found"},
                        "intake_form_url": {"type": "string", "description": "URL to signup/intake/registration form"},
                        "contact_name": {"type": "string", "description": "Name of contact person if mentioned"}
                    }
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "find_location",
                "description": "Search the page content to find location/address information for this post.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "address": {"type": "string", "description": "Street address"},
                        "city": {"type": "string", "description": "City name"},
                        "service_area": {"type": "string", "description": "Geographic area served"},
                        "is_virtual": {"type": "boolean", "description": "True if online/virtual only"}
                    }
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "find_schedule",
                "description": "Search the page content to find schedule/timing information.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "hours": {"type": "string", "description": "Operating hours (e.g., 'Mon-Fri 9am-5pm')"},
                        "dates": {"type": "string", "description": "Specific dates or recurring pattern"},
                        "frequency": {"type": "string", "description": "How often: 'weekly', 'monthly', 'one-time', 'ongoing'"},
                        "duration": {"type": "string", "description": "Time commitment (e.g., '2 hours')"}
                    }
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "find_eligibility",
                "description": "Search the page content to find who can use this service or participate.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "who_qualifies": {"type": "string", "description": "Target audience (e.g., 'Low-income families')"},
                        "requirements": {"type": "array", "items": {"type": "string"}, "description": "List of requirements"},
                        "restrictions": {"type": "string", "description": "Any restrictions"}
                    }
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "find_call_to_action",
                "description": "Determine what action someone should take to engage with this opportunity.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "action": {"type": "string", "description": "What to do: 'Sign up online', 'Call to register', etc."},
                        "url": {"type": "string", "description": "URL to take action"},
                        "instructions": {"type": "string", "description": "Any specific instructions"}
                    }
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "search_other_pages",
                "description": "Search other pages on this website (like /contact, /about, /hours) for missing information. Use this if you can't find contact info, location, or hours on the current page.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "What to search for (e.g., 'phone number', 'address', 'hours of operation')"},
                        "page_types": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Types of pages to search: 'contact', 'about', 'hours', 'location', 'volunteer'"
                        }
                    },
                    "required": ["query"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "search_external",
                "description": "Search the web for additional information about this organization or opportunity. Use this as a last resort if information is not on the website.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Search query (e.g., 'DHHMN phone number', 'Deaf Hard of Hearing Services Minnesota address')"}
                    },
                    "required": ["query"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "finalize_post",
                "description": "Call this when you have gathered all available information for the post.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "description": {"type": "string", "description": "Full, detailed description (2-4 sentences)"},
                        "confidence": {"type": "number", "description": "Confidence score 0-1"},
                        "notes": {"type": "array", "items": {"type": "string"}, "description": "Notes about what was found or couldn't be found"}
                    },
                    "required": ["description", "confidence", "notes"]
                }
            }
        }
    ])
}

// =============================================================================
// Prompts
// =============================================================================

const CANDIDATE_EXTRACTION_PROMPT: &str = r#"You are extracting post candidates from a community organization's web page.

Find ALL distinct opportunities, services, programs, or volunteer positions mentioned.
For each, extract just the basic shape - we'll enrich with details in a second pass.

Post types:
- "volunteer": Volunteer opportunities, ways to help
- "service": Services offered TO the community (food, housing, etc.)
- "event": One-time or recurring events
- "donation": Ways to donate money, goods, or time

For each post found, extract:
1. title: Clear, concise title
2. post_type: One of the types above
3. brief_description: 1 sentence summary
4. source_excerpt: The relevant text from the page (for context in enrichment)

Be thorough - it's better to extract too many candidates than miss something.
"#;

const ENRICHMENT_SYSTEM_PROMPT: &str = r#"You are enriching a community resource post with detailed information.

You have tools to search for specific information:
- Use find_* tools to extract info from the current page
- Use search_other_pages to look at /contact, /about, /hours pages on this site
- Use search_external to search the web (last resort)

Your goal is to make this post as COMPLETE and USEFUL as possible.

Guidelines:
1. First, search the current page content for all relevant info
2. If contact info, location, or hours are missing, use search_other_pages
3. Only use search_external if critical info is still missing
4. Pay special attention to:
   - Intake forms / signup links (often on external domains)
   - Phone numbers and emails
   - Physical addresses and service areas
   - Hours of operation
   - Who is eligible / who this serves

When done searching, call finalize_post with:
- A rich, detailed description (2-4 sentences)
- Confidence score (0-1) based on completeness
- Notes about what you found or couldn't find
"#;

const MERGE_PROMPT: &str = r#"You are deduplicating community resource posts extracted from different pages of the same website.

Given a list of posts, identify which ones are duplicates (same underlying opportunity/service) and merge them.

Rules:
1. Two posts are duplicates if they refer to the same program/service/opportunity
2. When merging, keep the MOST COMPLETE information from each
3. Prefer information from more specific pages over general pages
4. Keep both source URLs in the merged post

Return the deduplicated list with merged information.
"#;

// =============================================================================
// Extraction Functions
// =============================================================================

/// Extract post candidates from page content (lightweight first pass)
pub async fn extract_candidates(
    page_content: &str,
    page_url: &str,
    ai: &dyn BaseAI,
) -> Result<Vec<PostCandidate>> {
    let user_prompt = format!(
        "Page URL: {}\n\nPage Content:\n{}\n\nExtract all post candidates.",
        page_url,
        truncate_content(page_content, 12000)
    );

    // OpenAI structured output requires top-level object, so wrap array in object
    let schema = json!({
        "type": "object",
        "properties": {
            "candidates": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "title": {"type": "string"},
                        "post_type": {"type": "string", "enum": ["volunteer", "service", "event", "donation"]},
                        "brief_description": {"type": "string"},
                        "source_excerpt": {"type": "string"}
                    },
                    "required": ["title", "post_type", "brief_description", "source_excerpt"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["candidates"],
        "additionalProperties": false
    });

    let response = ai
        .generate_structured(CANDIDATE_EXTRACTION_PROMPT, &user_prompt, schema)
        .await
        .context("Failed to extract post candidates")?;

    // Parse wrapper object and extract candidates array
    let wrapper: serde_json::Value =
        serde_json::from_str(&response).context("Failed to parse response JSON")?;

    let candidates: Vec<PostCandidate> =
        serde_json::from_value(wrapper.get("candidates").cloned().unwrap_or(json!([])))
            .context("Failed to parse post candidates")?;

    info!(
        page_url = %page_url,
        candidates_found = candidates.len(),
        "Extracted post candidates"
    );

    Ok(candidates)
}

/// Enrich a single post candidate using the agent loop
pub async fn enrich_post(
    candidate: &PostCandidate,
    ctx: &EnrichmentContext<'_>,
    page_snapshot_id: Option<Uuid>,
) -> Result<EnrichedPost> {
    let tools = get_enrichment_tools();

    let user_prompt = format!(
        r#"Enrich this post with all available details:

Post to enrich:
- Title: {}
- Type: {}
- Brief description: {}
- Source excerpt: {}

Page URL: {}

Full page content to search:
{}

Other pages available to search: {}

Use the tools to find contact info, location, schedule, eligibility, and call-to-action.
When done, call finalize_post."#,
        candidate.title,
        candidate.post_type,
        candidate.brief_description,
        candidate.source_excerpt,
        ctx.page_url,
        truncate_content(ctx.page_content, 8000),
        ctx.other_pages
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut enriched = EnrichedPost {
        title: candidate.title.clone(),
        post_type: candidate.post_type.clone(),
        description: candidate.brief_description.clone(),
        contact: None,
        call_to_action: None,
        location: None,
        schedule: None,
        eligibility: None,
        source_url: Some(ctx.page_url.to_string()),
        source_page_snapshot_id: page_snapshot_id,
        confidence: 0.0,
        enrichment_notes: vec![],
    };

    let mut messages = vec![
        json!({"role": "system", "content": ENRICHMENT_SYSTEM_PROMPT}),
        json!({"role": "user", "content": user_prompt}),
    ];

    let max_iterations = 8;
    for iteration in 0..max_iterations {
        let response = ctx
            .ai
            .generate_with_tools(&messages, &tools)
            .await
            .context("Failed to generate enrichment response")?;

        if let Some(tool_calls) = response.get("tool_calls").and_then(|t| t.as_array()) {
            let mut tool_results = vec![];

            for tool_call in tool_calls {
                let function = &tool_call["function"];
                let name = function["name"].as_str().unwrap_or("");
                let args: serde_json::Value = function["arguments"]
                    .as_str()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or(json!({}));
                let tool_call_id = tool_call["id"].as_str().unwrap_or("");

                info!(iteration = iteration, tool = name, "Agent calling tool");

                match name {
                    "find_contact_info" => {
                        enriched.contact = Some(ContactInfo {
                            phone: args.get("phone").and_then(|v| v.as_str()).map(String::from),
                            email: args.get("email").and_then(|v| v.as_str()).map(String::from),
                            intake_form_url: args
                                .get("intake_form_url")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            contact_name: args
                                .get("contact_name")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            ..Default::default()
                        });
                        tool_results.push(json!({"role": "tool", "content": "Contact info recorded", "tool_call_id": tool_call_id}));
                    }
                    "find_location" => {
                        enriched.location = Some(LocationInfo {
                            address: args
                                .get("address")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            city: args.get("city").and_then(|v| v.as_str()).map(String::from),
                            service_area: args
                                .get("service_area")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            is_virtual: args
                                .get("is_virtual")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false),
                            ..Default::default()
                        });
                        tool_results.push(json!({"role": "tool", "content": "Location info recorded", "tool_call_id": tool_call_id}));
                    }
                    "find_schedule" => {
                        enriched.schedule = Some(ScheduleInfo {
                            general: args.get("hours").and_then(|v| v.as_str()).map(String::from),
                            dates: args.get("dates").and_then(|v| v.as_str()).map(String::from),
                            frequency: args
                                .get("frequency")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            duration: args
                                .get("duration")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            ..Default::default()
                        });
                        tool_results.push(json!({"role": "tool", "content": "Schedule info recorded", "tool_call_id": tool_call_id}));
                    }
                    "find_eligibility" => {
                        let requirements = args
                            .get("requirements")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default();
                        enriched.eligibility = Some(EligibilityInfo {
                            who_qualifies: args
                                .get("who_qualifies")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            requirements,
                            restrictions: args
                                .get("restrictions")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                        });
                        tool_results.push(json!({"role": "tool", "content": "Eligibility info recorded", "tool_call_id": tool_call_id}));
                    }
                    "find_call_to_action" => {
                        enriched.call_to_action = Some(CallToAction {
                            action: args
                                .get("action")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Contact for more info")
                                .to_string(),
                            url: args.get("url").and_then(|v| v.as_str()).map(String::from),
                            instructions: args
                                .get("instructions")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                        });
                        tool_results.push(json!({"role": "tool", "content": "Call to action recorded", "tool_call_id": tool_call_id}));
                    }
                    "search_other_pages" => {
                        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                        let page_types: Vec<&str> = args
                            .get("page_types")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                            .unwrap_or_else(|| vec!["contact", "about", "hours"]);

                        let mut found_content = String::new();
                        for (url, content) in ctx.other_pages.iter() {
                            let url_lower = url.to_lowercase();
                            if page_types.iter().any(|pt| url_lower.contains(pt)) {
                                if content.to_lowercase().contains(&query.to_lowercase()) {
                                    found_content.push_str(&format!(
                                        "\n\n--- From {} ---\n{}",
                                        url,
                                        truncate_content(content, 2000)
                                    ));
                                }
                            }
                        }

                        let result = if found_content.is_empty() {
                            format!("No matches found for '{}' in other pages", query)
                        } else {
                            format!("Found relevant content:{}", found_content)
                        };

                        enriched
                            .enrichment_notes
                            .push(format!("Searched other pages for: {}", query));
                        tool_results.push(json!({"role": "tool", "content": result, "tool_call_id": tool_call_id}));
                    }
                    "search_external" => {
                        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");

                        let result = if let Some(searcher) = ctx.web_searcher {
                            match searcher.search_with_limit(query, 3).await {
                                Ok(results) => {
                                    let formatted: Vec<String> = results
                                        .iter()
                                        .map(|r| {
                                            format!(
                                                "- {}: {}",
                                                r.title.as_deref().unwrap_or("Untitled"),
                                                truncate_content(r.snippet.as_deref().unwrap_or(""), 500)
                                            )
                                        })
                                        .collect();
                                    if formatted.is_empty() {
                                        "No external results found".to_string()
                                    } else {
                                        format!(
                                            "External search results:\n{}",
                                            formatted.join("\n")
                                        )
                                    }
                                }
                                Err(e) => format!("Search failed: {}", e),
                            }
                        } else {
                            "External search not available".to_string()
                        };

                        enriched
                            .enrichment_notes
                            .push(format!("External search: {}", query));
                        tool_results.push(json!({"role": "tool", "content": result, "tool_call_id": tool_call_id}));
                    }
                    "finalize_post" => {
                        if let Some(desc) = args.get("description").and_then(|v| v.as_str()) {
                            enriched.description = desc.to_string();
                        }
                        enriched.confidence = args
                            .get("confidence")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.5) as f32;

                        let notes: Vec<String> = args
                            .get("notes")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default();
                        enriched.enrichment_notes.extend(notes);

                        info!(
                            title = %enriched.title,
                            confidence = enriched.confidence,
                            "Post enrichment finalized"
                        );

                        return Ok(enriched);
                    }
                    _ => {
                        tool_results.push(json!({"role": "tool", "content": "Unknown tool", "tool_call_id": tool_call_id}));
                    }
                }
            }

            messages.push(json!({"role": "assistant", "tool_calls": tool_calls}));
            for result in tool_results {
                messages.push(result);
            }
        } else {
            enriched
                .enrichment_notes
                .push("Agent stopped without calling finalize_post".to_string());
            break;
        }
    }

    enriched
        .enrichment_notes
        .push("Reached max iterations".to_string());
    Ok(enriched)
}

// =============================================================================
// Rig-based Enrichment - FUTURE USE
// =============================================================================
//
// Note: rig-core 0.9.1 does not include a built-in multi-turn tool calling loop.
// The multi_turn_agent example shows that this must be implemented manually.
//
// For now, we keep using the BaseAI-based tool loop in `enrich_post_with_tools`
// which already implements the manual tool calling loop correctly.
//
// The rig Tool implementations in `enrichment_tools.rs` are ready for use when:
// 1. rig adds built-in multi-turn support, OR
// 2. We implement a custom multi-turn loop using rig's Agent and completion APIs
//
// Benefits already gained from rig:
// - Type-safe tool definitions with JsonSchema (ready in enrichment_tools.rs)
// - Cleaner OpenAI client abstraction (can be used in extraction library)
// - Structured extraction via rig::extractor (can be used for candidate extraction)

// =============================================================================
// BaseAI-based Enrichment (using generate_with_tools) - LEGACY
// =============================================================================

const BASEAI_ENRICHMENT_PREAMBLE: &str = r#"You are enriching a community resource post with detailed information.

You have tools to record what you find:
- find_contact_info: Record phone, email, intake form URL
- find_location: Record address, city, service area
- find_schedule: Record hours, dates, frequency
- find_eligibility: Record who qualifies, requirements
- find_call_to_action: Record what action to take
- search_other_pages: Search /contact, /about pages for missing info
- finalize_post: Call when done with description and confidence

Your goal is to make this post as COMPLETE and USEFUL as possible.
Search the page content carefully, then call finalize_post when done."#;

/// Enrich a post using BaseAI's generate_with_tools
///
/// Uses the server's BaseAI trait which is implemented by the extraction
/// library's OpenAI client, enabling any AI backend.
pub async fn enrich_post_with_tools(
    candidate: &PostCandidate,
    page_content: &str,
    page_url: &str,
    page_snapshot_id: Option<Uuid>,
    other_pages: HashMap<String, String>,
    ai: &dyn BaseAI,
) -> Result<EnrichedPost> {
    // Create shared state for tools to write to
    let enrichment_data: SharedEnrichmentData = Arc::new(RwLock::new(EnrichmentData::default()));
    let tools = get_enrichment_tools();

    // Build prompt
    let user_prompt = format!(
        r#"Enrich this post with all available details:

Post to enrich:
- Title: {}
- Type: {}
- Brief description: {}

Page URL: {}

Page content to search:
{}

Other pages available: {}

Use the tools to find and record contact info, location, schedule, eligibility, and call-to-action.
When done, call finalize_post with a detailed description and confidence score."#,
        candidate.title,
        candidate.post_type,
        candidate.brief_description,
        page_url,
        truncate_content(page_content, 8000),
        other_pages.keys().cloned().collect::<Vec<_>>().join(", ")
    );

    let mut messages = vec![
        json!({"role": "system", "content": BASEAI_ENRICHMENT_PREAMBLE}),
        json!({"role": "user", "content": user_prompt}),
    ];

    let max_iterations = 8;
    for iteration in 0..max_iterations {
        let response = ai
            .generate_with_tools(&messages, &tools)
            .await
            .context("Failed to generate enrichment response")?;

        if let Some(tool_calls) = response.get("tool_calls").and_then(|t| t.as_array()) {
            let mut tool_results = vec![];

            for tool_call in tool_calls {
                let function = &tool_call["function"];
                let name = function["name"].as_str().unwrap_or("");
                let args: serde_json::Value = function["arguments"]
                    .as_str()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or(json!({}));
                let tool_call_id = tool_call["id"].as_str().unwrap_or("");

                info!(iteration = iteration, tool = name, "[tool-call] Agent calling tool");

                let mut data = enrichment_data.write().await;

                match name {
                    "find_contact_info" => {
                        data.contact = Some(ContactInfo {
                            phone: args.get("phone").and_then(|v| v.as_str()).map(String::from),
                            email: args.get("email").and_then(|v| v.as_str()).map(String::from),
                            intake_form_url: args
                                .get("intake_form_url")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            contact_name: args
                                .get("contact_name")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            ..Default::default()
                        });
                        tool_results.push(json!({"role": "tool", "content": "Contact info recorded", "tool_call_id": tool_call_id}));
                    }
                    "find_location" => {
                        data.location = Some(LocationInfo {
                            address: args.get("address").and_then(|v| v.as_str()).map(String::from),
                            city: args.get("city").and_then(|v| v.as_str()).map(String::from),
                            service_area: args
                                .get("service_area")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            is_virtual: args.get("is_virtual").and_then(|v| v.as_bool()).unwrap_or(false),
                            ..Default::default()
                        });
                        tool_results.push(json!({"role": "tool", "content": "Location info recorded", "tool_call_id": tool_call_id}));
                    }
                    "find_schedule" => {
                        data.schedule = Some(ScheduleInfo {
                            general: args.get("hours").and_then(|v| v.as_str()).map(String::from),
                            dates: args.get("dates").and_then(|v| v.as_str()).map(String::from),
                            frequency: args.get("frequency").and_then(|v| v.as_str()).map(String::from),
                            duration: args.get("duration").and_then(|v| v.as_str()).map(String::from),
                            ..Default::default()
                        });
                        tool_results.push(json!({"role": "tool", "content": "Schedule info recorded", "tool_call_id": tool_call_id}));
                    }
                    "find_eligibility" => {
                        let requirements = args
                            .get("requirements")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                            .unwrap_or_default();
                        data.eligibility = Some(EligibilityInfo {
                            who_qualifies: args
                                .get("who_qualifies")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            requirements,
                            restrictions: args.get("restrictions").and_then(|v| v.as_str()).map(String::from),
                        });
                        tool_results.push(json!({"role": "tool", "content": "Eligibility info recorded", "tool_call_id": tool_call_id}));
                    }
                    "find_call_to_action" => {
                        data.call_to_action = Some(CallToAction {
                            action: args
                                .get("action")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Contact for more info")
                                .to_string(),
                            url: args.get("url").and_then(|v| v.as_str()).map(String::from),
                            instructions: args.get("instructions").and_then(|v| v.as_str()).map(String::from),
                        });
                        tool_results.push(json!({"role": "tool", "content": "Call to action recorded", "tool_call_id": tool_call_id}));
                    }
                    "search_other_pages" => {
                        drop(data); // Release lock before searching
                        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                        let page_types: Vec<&str> = args
                            .get("page_types")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                            .unwrap_or_else(|| vec!["contact", "about", "hours"]);

                        let mut found_content = String::new();
                        for (url, content) in &other_pages {
                            let url_lower = url.to_lowercase();
                            if page_types.iter().any(|pt| url_lower.contains(pt)) {
                                if content.to_lowercase().contains(&query.to_lowercase()) {
                                    found_content.push_str(&format!(
                                        "\n\n--- From {} ---\n{}",
                                        url,
                                        truncate_content(content, 2000)
                                    ));
                                }
                            }
                        }

                        let result = if found_content.is_empty() {
                            format!("No matches found for '{}' in other pages", query)
                        } else {
                            format!("Found relevant content:{}", found_content)
                        };

                        let mut data = enrichment_data.write().await;
                        data.notes.push(format!("Searched other pages for: {}", query));
                        drop(data);

                        tool_results.push(json!({"role": "tool", "content": result, "tool_call_id": tool_call_id}));
                    }
                    "finalize_post" => {
                        if let Some(desc) = args.get("description").and_then(|v| v.as_str()) {
                            data.description = Some(desc.to_string());
                        }
                        data.confidence = args.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32;
                        let notes: Vec<String> = args
                            .get("notes")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                            .unwrap_or_default();
                        data.notes.extend(notes);
                        data.finalized = true;

                        info!(
                            title = %candidate.title,
                            confidence = data.confidence,
                            "Post enrichment finalized"
                        );

                        // Build and return the enriched post
                        // Since EnrichmentData now uses the same unified types, we can clone directly
                        return Ok(EnrichedPost {
                            title: candidate.title.clone(),
                            post_type: candidate.post_type.clone(),
                            description: data.description.clone().unwrap_or_else(|| candidate.brief_description.clone()),
                            contact: data.contact.clone(),
                            location: data.location.clone(),
                            schedule: data.schedule.clone(),
                            eligibility: data.eligibility.clone(),
                            call_to_action: data.call_to_action.clone(),
                            source_url: Some(page_url.to_string()),
                            source_page_snapshot_id: page_snapshot_id,
                            confidence: data.confidence,
                            enrichment_notes: data.notes.clone(),
                        });
                    }
                    _ => {
                        drop(data);
                        tool_results.push(json!({"role": "tool", "content": "Unknown tool", "tool_call_id": tool_call_id}));
                    }
                }
            }

            messages.push(json!({"role": "assistant", "tool_calls": tool_calls}));
            for result in tool_results {
                messages.push(result);
            }
        } else {
            // No tool calls - agent stopped without finalizing
            let mut data = enrichment_data.write().await;
            data.notes.push("Agent stopped without calling finalize_post".to_string());
            break;
        }
    }

    // Reached max iterations or agent stopped - return what we have
    let data = enrichment_data.read().await;
    let mut enriched = EnrichedPost {
        title: candidate.title.clone(),
        post_type: candidate.post_type.clone(),
        description: data.description.clone().unwrap_or_else(|| candidate.brief_description.clone()),
        contact: data.contact.clone(),
        location: data.location.clone(),
        schedule: data.schedule.clone(),
        eligibility: data.eligibility.clone(),
        call_to_action: data.call_to_action.clone(),
        source_url: Some(page_url.to_string()),
        source_page_snapshot_id: page_snapshot_id,
        confidence: data.confidence,
        enrichment_notes: data.notes.clone(),
    };
    enriched.enrichment_notes.push("Reached max iterations".to_string());

    info!(
        title = %enriched.title,
        confidence = enriched.confidence,
        has_contact = enriched.contact.is_some(),
        has_location = enriched.location.is_some(),
        has_schedule = enriched.schedule.is_some(),
        finalized = data.finalized,
        "BaseAI agent enriched post"
    );

    Ok(enriched)
}

// =============================================================================
// Merge / Dedupe
// =============================================================================

/// Merge duplicate posts from different pages
pub async fn merge_posts(posts: Vec<EnrichedPost>, ai: &dyn BaseAI) -> Result<Vec<EnrichedPost>> {
    if posts.len() <= 1 {
        return Ok(posts);
    }

    let posts_json = serde_json::to_string_pretty(&posts)?;

    // Use regular JSON completion for complex nested structure
    // OpenAI's strict structured output doesn't handle nullable nested objects well
    let prompt = format!(
        r#"{}

Here are {} posts extracted from different pages of the same website.
Identify duplicates and merge them, keeping the most complete information.

Posts:
{}

Return ONLY a JSON object with this structure:
{{"posts": [array of deduplicated posts with same structure as input]}}

Each post should have: title, post_type, description, and optionally contact, location, schedule, eligibility, call_to_action, source_url, confidence, enrichment_notes."#,
        MERGE_PROMPT,
        posts.len(),
        posts_json
    );

    let response = match ai.complete_json(&prompt).await {
        Ok(r) => {
            info!(
                response_length = r.len(),
                "Received merge response from LLM"
            );
            r
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                posts_count = posts.len(),
                "Merge LLM call failed, returning original posts"
            );
            return Ok(posts);
        }
    };

    // Try to extract JSON from the response (may have markdown code blocks or extra text)
    let json_str = extract_json_from_response(&response);

    // Try to parse as wrapper object first, then as direct array
    let merged: Vec<EnrichedPost> = match serde_json::from_str::<serde_json::Value>(&json_str) {
        Ok(wrapper) => {
            if let Some(posts_array) = wrapper.get("posts") {
                match serde_json::from_value::<Vec<EnrichedPost>>(posts_array.clone()) {
                    Ok(parsed) => parsed,
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            response_preview = %truncate_content(&json_str, 500),
                            "Failed to deserialize posts array, returning original posts"
                        );
                        return Ok(posts);
                    }
                }
            } else if wrapper.is_array() {
                match serde_json::from_value::<Vec<EnrichedPost>>(wrapper) {
                    Ok(parsed) => parsed,
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            response_preview = %truncate_content(&json_str, 500),
                            "Failed to deserialize posts array directly, returning original posts"
                        );
                        return Ok(posts);
                    }
                }
            } else {
                tracing::warn!(
                    response_preview = %truncate_content(&json_str, 500),
                    "Merge response is neither object with 'posts' nor array, returning original posts"
                );
                return Ok(posts);
            }
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                response_preview = %truncate_content(&response, 500),
                "Failed to parse merge response as JSON, returning original posts"
            );
            // Return original posts rather than failing completely
            return Ok(posts);
        }
    };

    // Restore source_page_snapshot_id from original posts by matching titles
    // When posts are merged, the LLM doesn't preserve this field, so we need to restore it
    let title_to_page_ids: HashMap<String, Vec<Uuid>> = posts
        .iter()
        .filter_map(|p| {
            p.source_page_snapshot_id
                .map(|id| (p.title.to_lowercase(), id))
        })
        .fold(HashMap::new(), |mut acc, (title, id)| {
            acc.entry(title).or_default().push(id);
            acc
        });

    // Also build a map for fuzzy matching by checking if merged title contains original title
    let mut merged_with_page_ids: Vec<EnrichedPost> = merged
        .into_iter()
        .map(|mut post| {
            // First try exact match
            if let Some(ids) = title_to_page_ids.get(&post.title.to_lowercase()) {
                post.source_page_snapshot_id = ids.first().copied();
            } else {
                // Try fuzzy match - if merged title contains any original title
                for (orig_title, ids) in &title_to_page_ids {
                    if post.title.to_lowercase().contains(orig_title)
                        || orig_title.contains(&post.title.to_lowercase())
                    {
                        post.source_page_snapshot_id = ids.first().copied();
                        break;
                    }
                }
            }
            post
        })
        .collect();

    // If we still have posts without page IDs but original posts had them, use the first available
    let all_page_ids: Vec<Uuid> = posts
        .iter()
        .filter_map(|p| p.source_page_snapshot_id)
        .collect();

    if !all_page_ids.is_empty() {
        for post in &mut merged_with_page_ids {
            if post.source_page_snapshot_id.is_none() {
                post.source_page_snapshot_id = all_page_ids.first().copied();
            }
        }
    }

    let merged_count = posts.len() - merged_with_page_ids.len();
    info!(
        original_count = posts.len(),
        merged_count = merged_count,
        final_count = merged_with_page_ids.len(),
        posts_with_page_ids = merged_with_page_ids
            .iter()
            .filter(|p| p.source_page_snapshot_id.is_some())
            .count(),
        "Posts merged"
    );

    Ok(merged_with_page_ids)
}

// =============================================================================
// Full Pipeline
// =============================================================================

/// Extract posts from a single page
pub async fn extract_from_page(
    page_content: &str,
    page_url: &str,
    page_snapshot_id: Option<Uuid>,
    other_pages: &HashMap<String, String>,
    web_searcher: Option<&dyn WebSearcher>,
    ai: &dyn BaseAI,
) -> Result<Vec<EnrichedPost>> {
    let candidates = extract_candidates(page_content, page_url, ai).await?;

    if candidates.is_empty() {
        return Ok(vec![]);
    }

    let ctx = EnrichmentContext {
        page_content,
        page_url,
        other_pages,
        web_searcher,
        ai,
    };

    let mut enriched_posts = Vec::with_capacity(candidates.len());

    for candidate in &candidates {
        match enrich_post(candidate, &ctx, page_snapshot_id).await {
            Ok(enriched) => {
                info!(
                    title = %enriched.title,
                    confidence = enriched.confidence,
                    has_contact = enriched.contact.is_some(),
                    has_location = enriched.location.is_some(),
                    "Enriched post"
                );
                enriched_posts.push(enriched);
            }
            Err(e) => {
                tracing::warn!(title = %candidate.title, error = %e, "Failed to enrich post");
            }
        }
    }

    Ok(enriched_posts)
}

/// Extract posts from all pages of a website, merge duplicates, skip existing
pub async fn extract_from_website(
    website_id: WebsiteId,
    pages: &[(Uuid, String, String)], // (page_snapshot_id, url, content)
    pool: &PgPool,
    web_searcher: Option<&dyn WebSearcher>,
    ai: &dyn BaseAI,
) -> Result<WebsiteExtractionResult> {
    // Load existing posts to skip redundant extraction
    let existing = ExistingPosts::load(website_id, pool).await?;

    // Build map of all pages for cross-referencing
    let other_pages: HashMap<String, String> = pages
        .iter()
        .map(|(_, url, content)| (url.clone(), content.clone()))
        .collect();

    let mut all_posts = Vec::new();
    let mut total_candidates = 0;
    let mut skipped_candidates = 0;

    // Extract from each page
    for (page_snapshot_id, page_url, page_content) in pages {
        info!(page_url = %page_url, "Extracting posts from page");

        // First, get candidates (lightweight)
        let candidates = match extract_candidates(page_content, page_url, ai).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(page_url = %page_url, error = %e, "Failed to extract candidates");
                continue;
            }
        };

        total_candidates += candidates.len();

        // Filter out candidates that already exist in DB
        let new_candidates: Vec<_> = candidates
            .into_iter()
            .filter(|c| {
                if let Some(existing_post) = existing.find_match(&c.title) {
                    info!(
                        title = %c.title,
                        existing_post_id = %existing_post.id,
                        "Skipping candidate - already exists in DB"
                    );
                    skipped_candidates += 1;
                    false
                } else {
                    true
                }
            })
            .collect();

        if new_candidates.is_empty() {
            info!(page_url = %page_url, "All candidates already exist, skipping enrichment");
            continue;
        }

        info!(
            page_url = %page_url,
            new_candidates = new_candidates.len(),
            "Enriching new candidates"
        );

        // Remove current page from other_pages for this extraction
        let mut other = other_pages.clone();
        other.remove(page_url);

        let ctx = EnrichmentContext {
            page_content,
            page_url,
            other_pages: &other,
            web_searcher,
            ai,
        };

        // Enrich only new candidates
        for candidate in &new_candidates {
            match enrich_post(candidate, &ctx, Some(*page_snapshot_id)).await {
                Ok(enriched) => {
                    info!(
                        title = %enriched.title,
                        confidence = enriched.confidence,
                        has_contact = enriched.contact.is_some(),
                        has_location = enriched.location.is_some(),
                        "Enriched new post"
                    );
                    all_posts.push(enriched);
                }
                Err(e) => {
                    tracing::warn!(title = %candidate.title, error = %e, "Failed to enrich post");
                }
            }
        }
    }

    // Merge duplicates across pages (for NEW posts only)
    info!(
        website_id = %website_id,
        posts_count = all_posts.len(),
        "Starting merge step"
    );

    let pre_merge_count = all_posts.len();
    let merged_posts = if all_posts.len() > 1 {
        match merge_posts(all_posts.clone(), ai).await {
            Ok(merged) => {
                info!(
                    website_id = %website_id,
                    original = pre_merge_count,
                    merged = merged.len(),
                    "Merge completed successfully"
                );
                merged
            }
            Err(e) => {
                tracing::warn!(
                    website_id = %website_id,
                    error = %e,
                    "Merge failed, using unmerged posts"
                );
                all_posts
            }
        }
    } else {
        all_posts
    };
    let posts_merged = pre_merge_count - merged_posts.len();

    info!(
        website_id = %website_id,
        pages_processed = pages.len(),
        candidates_found = total_candidates,
        candidates_skipped = skipped_candidates,
        new_posts = merged_posts.len(),
        posts_merged = posts_merged,
        "Website extraction complete"
    );

    Ok(WebsiteExtractionResult {
        posts: merged_posts,
        pages_processed: pages.len(),
        candidates_found: total_candidates,
        candidates_skipped: skipped_candidates,
        posts_merged,
    })
}

// =============================================================================
// Storage
// =============================================================================

/// Store extraction results in page_extractions table
pub async fn store_extraction(
    pool: &PgPool,
    page_snapshot_id: PageSnapshotId,
    posts: &[EnrichedPost],
    model: Option<String>,
    prompt_version: Option<String>,
) -> Result<PageExtraction> {
    let content = serde_json::to_value(posts).context("Failed to serialize posts")?;

    PageExtraction::create(
        pool,
        page_snapshot_id,
        "posts",
        content,
        model,
        prompt_version,
        None,
    )
    .await
}

/// Store website-level merged extraction
pub async fn store_website_extraction(
    pool: &PgPool,
    website_snapshot_id: Uuid,
    result: &WebsiteExtractionResult,
    model: Option<String>,
) -> Result<()> {
    // Store as a special "website_posts" extraction linked to first page
    // In practice, you might want a separate website_extractions table
    let content = serde_json::to_value(result).context("Failed to serialize website extraction")?;

    sqlx::query(
        r#"
        INSERT INTO page_extractions (page_snapshot_id, extraction_type, content, model, is_current)
        SELECT ws.page_snapshot_id, 'website_posts', $2, $3, TRUE
        FROM website_snapshots ws
        WHERE ws.id = $1
        LIMIT 1
        "#,
    )
    .bind(website_snapshot_id)
    .bind(&content)
    .bind(&model)
    .execute(pool)
    .await
    .context("Failed to store website extraction")?;

    Ok(())
}

// =============================================================================
// Sync to Database
// =============================================================================

/// Create a new post directly from an enriched post (simplified sync)
pub async fn create_post_from_enriched(
    enriched: &EnrichedPost,
    website_id: WebsiteId,
    organization_name: &str,
    pool: &PgPool,
) -> Result<Post> {
    // Build location string
    let location = enriched.location.as_ref().map(|loc| {
        let mut parts = Vec::new();
        if let Some(addr) = &loc.address {
            parts.push(addr.clone());
        }
        if let Some(city) = &loc.city {
            parts.push(city.clone());
        }
        if let Some(area) = &loc.service_area {
            parts.push(format!("({})", area));
        }
        if loc.is_virtual {
            parts.push("Virtual".to_string());
        }
        parts.join(", ")
    });

    // Build description with contact and schedule info embedded
    let mut full_description = enriched.description.clone();

    if let Some(schedule) = &enriched.schedule {
        let mut schedule_parts = Vec::new();
        if let Some(hours) = &schedule.general {
            schedule_parts.push(format!("Hours: {}", hours));
        }
        if let Some(dates) = &schedule.dates {
            schedule_parts.push(format!("When: {}", dates));
        }
        if !schedule_parts.is_empty() {
            full_description.push_str(&format!("\n\n{}", schedule_parts.join(" | ")));
        }
    }

    if let Some(eligibility) = &enriched.eligibility {
        if let Some(who) = &eligibility.who_qualifies {
            full_description.push_str(&format!("\n\nWho this is for: {}", who));
        }
    }

    // Map post_type to database values
    let post_type = match enriched.post_type.as_str() {
        "volunteer" => "opportunity",
        "donation" => "opportunity",
        "event" => "opportunity",
        "service" => "service",
        _ => "service",
    };

    // Determine urgency based on confidence and content
    let urgency = if enriched.confidence > 0.8 {
        "medium"
    } else {
        "low"
    };

    let post = sqlx::query_as::<_, Post>(
        r#"
        INSERT INTO posts (
            organization_name,
            title,
            description,
            post_type,
            category,
            urgency,
            status,
            source_language,
            location,
            submission_type,
            website_id,
            source_url
        )
        VALUES ($1, $2, $3, $4, 'community', $5, 'pending_approval', 'en', $6, 'scraped', $7, $8)
        RETURNING *
        "#,
    )
    .bind(organization_name)
    .bind(&enriched.title)
    .bind(&full_description)
    .bind(post_type)
    .bind(urgency)
    .bind(&location)
    .bind(website_id)
    .bind(&enriched.source_url)
    .fetch_one(pool)
    .await
    .context("Failed to create post from enriched data")?;

    // Store contact info in post_contacts if available
    if let Some(contact) = &enriched.contact {
        if contact.phone.is_some() || contact.email.is_some() || contact.intake_form_url.is_some() {
            let _ = sqlx::query(
                r#"
                INSERT INTO post_contacts (post_id, phone, email, intake_form_url)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (post_id) DO UPDATE SET
                    phone = COALESCE(EXCLUDED.phone, post_contacts.phone),
                    email = COALESCE(EXCLUDED.email, post_contacts.email),
                    intake_form_url = COALESCE(EXCLUDED.intake_form_url, post_contacts.intake_form_url)
                "#,
            )
            .bind(post.id)
            .bind(&contact.phone)
            .bind(&contact.email)
            .bind(&contact.intake_form_url)
            .execute(pool)
            .await;
        }
    }

    info!(
        post_id = %post.id,
        title = %post.title,
        has_contact = enriched.contact.is_some(),
        has_location = enriched.location.is_some(),
        "Created post from enriched extraction"
    );

    Ok(post)
}

/// Sync all enriched posts to database (simplified sync)
pub async fn sync_enriched_posts(
    result: &WebsiteExtractionResult,
    website_id: WebsiteId,
    organization_name: &str,
    pool: &PgPool,
) -> Result<SyncResult> {
    let mut created = 0;
    let mut failed = 0;

    for enriched in &result.posts {
        match create_post_from_enriched(enriched, website_id, organization_name, pool).await {
            Ok(_) => created += 1,
            Err(e) => {
                tracing::warn!(
                    title = %enriched.title,
                    error = %e,
                    "Failed to create post"
                );
                failed += 1;
            }
        }
    }

    info!(
        website_id = %website_id,
        created = created,
        failed = failed,
        skipped = result.candidates_skipped,
        "Sync complete"
    );

    Ok(SyncResult {
        created,
        failed,
        skipped: result.candidates_skipped,
    })
}

/// Result of syncing posts to database
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub created: usize,
    pub failed: usize,
    pub skipped: usize,
}

// =============================================================================
// Conversions for Pipeline Integration
// =============================================================================

impl EnrichedPost {
    /// Convert to the common ExtractedPost type for pipeline compatibility
    pub fn to_extracted_post(&self) -> ExtractedPost {
        // Build location string
        let location = self.location.as_ref().map(|loc| {
            let mut parts = Vec::new();
            if let Some(addr) = &loc.address {
                parts.push(addr.clone());
            }
            if let Some(city) = &loc.city {
                parts.push(city.clone());
            }
            if let Some(area) = &loc.service_area {
                parts.push(format!("({})", area));
            }
            if loc.is_virtual {
                parts.push("Virtual".to_string());
            }
            parts.join(", ")
        });

        // Build tldr from brief info
        let tldr = if self.description.len() > 150 {
            format!("{}...", &self.description[..147])
        } else {
            self.description.clone()
        };

        // Contact info is now unified - just clone it
        let contact = self.contact.clone();

        // Determine confidence level
        let confidence = if self.confidence > 0.8 {
            Some("high".to_string())
        } else if self.confidence > 0.5 {
            Some("medium".to_string())
        } else {
            Some("low".to_string())
        };

        // Map post_type to audience_roles
        let audience_roles = match self.post_type.as_str() {
            "volunteer" => vec!["volunteer".to_string()],
            "donation" => vec!["donor".to_string()],
            "service" => vec!["recipient".to_string()],
            "event" => vec!["participant".to_string()],
            _ => vec![],
        };

        ExtractedPost {
            title: self.title.clone(),
            tldr,
            description: self.description.clone(),
            contact,
            location,
            urgency: None,
            confidence,
            audience_roles,
            source_page_snapshot_id: self.source_page_snapshot_id,
        }
    }
}

/// Convert a list of EnrichedPosts to ExtractedPosts
pub fn to_extracted_posts(enriched: &[EnrichedPost]) -> Vec<ExtractedPost> {
    enriched.iter().map(|e| e.to_extracted_post()).collect()
}

// =============================================================================
// Helpers
// =============================================================================

fn truncate_content(content: &str, max_chars: usize) -> &str {
    if content.len() <= max_chars {
        content
    } else {
        content
            .char_indices()
            .take_while(|(i, _)| *i < max_chars)
            .last()
            .map(|(i, c)| &content[..i + c.len_utf8()])
            .unwrap_or(content)
    }
}

/// Extract JSON from a response that may have markdown code blocks or extra text
fn extract_json_from_response(response: &str) -> String {
    let trimmed = response.trim();

    // Try to extract from markdown code block (```json ... ``` or ``` ... ```)
    if let Some(start) = trimmed.find("```json") {
        if let Some(end) = trimmed[start + 7..].find("```") {
            return trimmed[start + 7..start + 7 + end].trim().to_string();
        }
    }
    if let Some(start) = trimmed.find("```") {
        let after_start = start + 3;
        // Skip the language identifier if present (e.g., "json\n")
        let content_start = trimmed[after_start..]
            .find('\n')
            .map(|i| after_start + i + 1)
            .unwrap_or(after_start);
        if let Some(end) = trimmed[content_start..].find("```") {
            return trimmed[content_start..content_start + end]
                .trim()
                .to_string();
        }
    }

    // Try to find JSON object or array boundaries
    if let Some(obj_start) = trimmed.find('{') {
        if let Some(obj_end) = trimmed.rfind('}') {
            if obj_end > obj_start {
                return trimmed[obj_start..=obj_end].to_string();
            }
        }
    }
    if let Some(arr_start) = trimmed.find('[') {
        if let Some(arr_end) = trimmed.rfind(']') {
            if arr_end > arr_start {
                return trimmed[arr_start..=arr_end].to_string();
            }
        }
    }

    // Return as-is if no JSON structure found
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_content() {
        assert_eq!(truncate_content("hello", 10), "hello");
        assert_eq!(truncate_content("hello world", 5), "hello");
    }

    #[test]
    fn test_tools_json_valid() {
        let tools = get_enrichment_tools();
        assert!(tools.is_array());
        assert_eq!(tools.as_array().unwrap().len(), 8); // Now 8 tools
    }
}
