//! Note extraction from crawled content sources.
//!
//! Scans crawled website pages and social media posts for noteworthy
//! information and creates notes linked to the organization.
//!
//! Pipeline: Generate → Merge (dedup) → Create notes linked to org.

use anyhow::Result;
use schemars::JsonSchema;
use serde::Deserialize;
use tracing::{info, warn};
use uuid::Uuid;

use crate::common::OrganizationId;
use crate::domains::crawling::models::ExtractionPage;
use crate::domains::notes::models::{Note, Noteable};
use crate::domains::source::models::{Source, WebsiteSource};
use crate::kernel::ServerDeps;

// =============================================================================
// LLM Response Types
// =============================================================================

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ExtractedNotes {
    /// List of noteworthy items found in the content
    pub notes: Vec<ExtractedNote>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ExtractedNote {
    /// The note content — a concise, factual statement
    pub content: String,
    /// Severity: "info", "notice", or "urgent"
    pub severity: String,
    /// The source URL where this information was found (optional)
    pub source_url: Option<String>,
    /// A concise call-to-action summary matching the severity tone
    pub cta_text: Option<String>,
}

// =============================================================================
// LLM Prompt
// =============================================================================

const NOTE_EXTRACTION_PROMPT: &str = r#"You are extracting noteworthy operational information from an organization's web pages and social media.

Look for information that would be important for someone trying to access this organization's services. Focus on:

## URGENT severity (action needed / people should know before visiting)
- Service pauses or shutdowns ("we are pausing all donations", "no longer accepting volunteers")
- Capacity limits ("at capacity", "waitlist only")
- Safety or fraud alerts
- Permanent closures

## NOTICE severity (worth knowing)
- Temporary closures or holiday hours ("closed Dec 24-Jan 1")
- Schedule changes ("new hours starting March")
- Service modifications ("now offering virtual services only")
- Eligibility changes

## INFO severity (general context)
- Location changes ("we've moved to...")
- New programs or services launching
- Major organizational announcements

## CTA Text (cta_text)
For each note, provide a concise summary that captures the core ask or awareness. The cta_text tone should match the severity:
- URGENT: What is urgently needed or asked of people. E.g., "Emergency rent and food assistance needed for families facing eviction"
- NOTICE: What people should be aware of. E.g., "Evening services no longer available starting March 15"
- INFO: What has changed or is new. E.g., "Online applications now accepted"
cta_text is required for all notes.

## Rules
- Only extract genuinely noteworthy operational information
- Do NOT extract marketing content, mission statements, or general descriptions
- Do NOT extract historical information (e.g., "founded in 1995")
- Each note should be a concise, factual statement (1-2 sentences max)
- If nothing noteworthy is found, return an empty notes array
- Prefer fewer, high-quality notes over many low-value ones
- Set severity accurately based on the categories above
- For each note, include the source_url of the page or post where you found the information
"#;

// =============================================================================
// Content Source Abstraction
// =============================================================================

/// A piece of content from any source (website page, social media post, etc.)
pub struct SourceContent {
    /// Unique ID of the source (page ID, social post ID, etc.)
    pub source_id: Uuid,
    /// Source type: "website", "instagram", "facebook", etc.
    pub source_type: String,
    /// URL of the source content
    pub source_url: String,
    /// The text content to analyze
    pub content: String,
}

// =============================================================================
// Content Gathering (extensible per source type)
// =============================================================================

/// Gather content from all websites linked to an organization.
async fn gather_website_content(
    org_id: OrganizationId,
    deps: &ServerDeps,
) -> Result<Vec<SourceContent>> {
    let all_sources = Source::find_by_organization(org_id, &deps.db_pool).await?;
    let website_sources: Vec<_> = all_sources.iter().filter(|s| s.source_type == "website").collect();
    let mut content = Vec::new();

    for source in &website_sources {
        let ws = match WebsiteSource::find_by_source_id(source.id, &deps.db_pool).await {
            Ok(ws) => ws,
            Err(e) => {
                warn!(source_id = %source.id, error = %e, "Failed to load website source details");
                continue;
            }
        };

        let pages = ExtractionPage::find_by_domain(&ws.domain, &deps.db_pool).await?;

        for (page_id, url, text) in pages {
            if text.trim().is_empty() {
                continue;
            }
            content.push(SourceContent {
                source_id: page_id,
                source_type: "website".to_string(),
                source_url: url,
                content: text,
            });
        }
    }

    Ok(content)
}

// =============================================================================
// Note Extraction
// =============================================================================

/// Maximum content size to send to the LLM per call
const MAX_CONTENT_CHARS: usize = 60_000;

/// Build LLM user prompt from source content, capped at MAX_CONTENT_CHARS.
fn build_extraction_content(sources: &[&SourceContent], org_name: &str) -> String {
    let mut content = format!("## Organization: {}\n\n", org_name);
    let mut total = content.len();

    for source in sources {
        let header = format!("### Source ({}) — {}\n\n", source.source_type, source.source_url);
        let entry_size = header.len() + source.content.len() + 10;

        if total + entry_size > MAX_CONTENT_CHARS {
            let remaining = MAX_CONTENT_CHARS.saturating_sub(total + header.len() + 10);
            if remaining > 200 {
                content.push_str(&header);
                content.push_str(&source.content[..remaining.min(source.content.len())]);
                content.push_str("\n\n---\n\n");
            }
            break;
        }

        content.push_str(&header);
        content.push_str(&source.content);
        content.push_str("\n\n---\n\n");
        total += entry_size;
    }

    content
}

/// Check if a note with similar content already exists for this organization.
/// Deduplicates at the org level — catches duplicates across different sources.
async fn is_duplicate_for_org(
    content: &str,
    org_id: OrganizationId,
    pool: &sqlx::PgPool,
) -> Result<bool> {
    let existing = Note::find_active_for_entity("organization", org_id.into_uuid(), pool).await?;
    let normalized = content.trim().to_lowercase();
    Ok(existing.iter().any(|n| n.content.trim().to_lowercase() == normalized))
}

/// Resolve a source URL from the extracted note or fall back to source list.
fn resolve_source_attribution<'a>(
    extracted_source_url: Option<&'a str>,
    all_sources: &'a [SourceContent],
) -> (Option<Uuid>, Option<&'a str>, Option<&'a str>) {
    // If the LLM provided a source_url, find the matching source
    if let Some(url) = extracted_source_url {
        if let Some(source) = all_sources.iter().find(|s| s.source_url == url) {
            return (
                Some(source.source_id),
                Some(source.source_type.as_str()),
                Some(source.source_url.as_str()),
            );
        }
    }

    // Fall back to the first source
    match all_sources.first() {
        Some(s) => (Some(s.source_id), Some(s.source_type.as_str()), Some(s.source_url.as_str())),
        None => (None, None, None),
    }
}

// =============================================================================
// Main Activities
// =============================================================================

pub struct GenerateNotesResult {
    pub notes_created: i32,
    pub sources_scanned: i32,
}

/// Core extraction logic: takes pre-gathered content, runs LLM extraction,
/// deduplicates against existing org notes, and creates new notes.
///
/// Used by both `generate_notes_for_organization` (standalone/admin) and
/// directly by workflows that already have content in memory.
pub async fn extract_and_create_notes(
    org_id: OrganizationId,
    org_name: &str,
    all_sources: Vec<SourceContent>,
    deps: &ServerDeps,
) -> Result<GenerateNotesResult> {
    let pool = &deps.db_pool;

    if all_sources.is_empty() {
        info!(org_id = %org_id, "No content sources provided, skipping note extraction");
        return Ok(GenerateNotesResult {
            notes_created: 0,
            sources_scanned: 0,
        });
    }

    let sources_scanned = all_sources.len() as i32;

    // Build LLM prompt from all sources
    let source_refs: Vec<&SourceContent> = all_sources.iter().collect();
    let user_content = build_extraction_content(&source_refs, org_name);

    info!(
        org_id = %org_id,
        sources = sources_scanned,
        content_chars = user_content.len(),
        "Extracting notes from content"
    );

    // LLM extraction
    let extracted: ExtractedNotes = deps
        .ai
        .extract("gpt-4o", NOTE_EXTRACTION_PROMPT, &user_content)
        .await
        .map_err(|e| anyhow::anyhow!("Note extraction failed: {}", e))?;

    info!(
        org_id = %org_id,
        notes_found = extracted.notes.len(),
        "LLM extraction complete"
    );

    // Create notes, deduplicating against existing org notes
    let mut notes_created = 0;

    for extracted_note in &extracted.notes {
        let severity = match extracted_note.severity.as_str() {
            "urgent" | "notice" | "info" => extracted_note.severity.as_str(),
            _ => "info",
        };

        // Org-level dedup: check against all active notes for this org
        if is_duplicate_for_org(&extracted_note.content, org_id, pool).await? {
            info!(content = %extracted_note.content, "Skipping duplicate note");
            continue;
        }

        // Resolve source attribution from LLM-provided URL or fall back
        let (source_id, source_type, source_url) = resolve_source_attribution(
            extracted_note.source_url.as_deref(),
            &all_sources,
        );

        let note = Note::create(
            &extracted_note.content,
            severity,
            source_url,
            source_id,
            source_type,
            false, // is_public defaults to false for system-generated
            "system",
            extracted_note.cta_text.as_deref(),
            pool,
        )
        .await?;

        // Generate embedding for semantic matching against posts
        if let Ok(emb) = deps.embedding_service.generate(&extracted_note.content).await {
            if let Err(e) = Note::update_embedding(note.id, &emb, pool).await {
                warn!(note_id = %note.id, error = %e, "Failed to store note embedding");
            }
        }

        // Link to organization
        Noteable::create(note.id, "organization", org_id.into_uuid(), pool).await?;

        info!(
            note_id = %note.id,
            severity = %severity,
            content = %extracted_note.content,
            "Created note for organization"
        );

        notes_created += 1;
    }

    info!(
        org_id = %org_id,
        notes_created,
        sources_scanned,
        "Note generation complete"
    );

    Ok(GenerateNotesResult {
        notes_created,
        sources_scanned,
    })
}

/// Extract noteworthy information from all website sources for an organization
/// and create notes linked to it.
///
/// This is the standalone entry point used by the admin UI and the Restate
/// NotesService. For workflow integration where social content is already
/// available, use `extract_and_create_notes` directly.
pub async fn generate_notes_for_organization(
    org_id: OrganizationId,
    org_name: &str,
    deps: &ServerDeps,
) -> Result<GenerateNotesResult> {
    // Gather content from website sources
    let mut all_sources = Vec::new();

    match gather_website_content(org_id, deps).await {
        Ok(sources) => {
            info!(org_id = %org_id, website_sources = sources.len(), "Gathered website content");
            all_sources.extend(sources);
        }
        Err(e) => {
            warn!(org_id = %org_id, error = %e, "Failed to gather website content");
        }
    }

    extract_and_create_notes(org_id, org_name, all_sources, deps).await
}
