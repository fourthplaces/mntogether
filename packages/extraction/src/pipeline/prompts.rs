//! LLM prompts for the extraction pipeline.
//!
//! These prompts are designed for recall-optimized summarization and
//! evidence-grounded extraction.

use sha2::{Digest, Sha256};

/// Prompt for summarizing a page with recall-optimized signals.
pub const SUMMARIZE_PROMPT: &str = r#"Summarize this webpage for information retrieval.

Your summary must capture:
1. What the page offers (services, programs, opportunities)
2. What the page asks for (volunteers, donations, applications)
3. Calls to action (sign up, apply, contact, donate)
4. Key entities (organization names, locations, dates, contacts)

Output JSON:
{
    "summary": "2-3 sentence overview focusing on actionable content",
    "signals": {
        "offers": ["list of things offered - services, programs, opportunities"],
        "asks": ["list of things requested - volunteers, donations, applications"],
        "calls_to_action": ["list of CTAs - sign up, apply, contact, donate"],
        "entities": ["key proper nouns - org names, locations, dates, contacts"]
    },
    "language": "detected language code (en, es, etc.)"
}

Page URL: {url}
Page Content:
{content}"#;

/// Prompt for expanding a query with related terms.
pub const EXPAND_QUERY_PROMPT: &str = r#"Expand this search query with related terms to improve recall.

Query: {query}

Generate 5-10 related search terms that would help find relevant content.
Include:
- Synonyms
- Related concepts
- Common phrasings
- Industry jargon

Output JSON array of strings:
["term1", "term2", "term3", ...]"#;

/// Prompt for classifying query intent.
pub const CLASSIFY_QUERY_PROMPT: &str = r#"Classify the intent of this search query.

Query: {query}

Categories:
- COLLECTION: "Find all X" - looking for a list of items (volunteer opportunities, services, events)
- SINGULAR: "Find specific info" - looking for one piece of information (phone number, email, address)
- NARRATIVE: "Summarize/describe" - looking for an overview or description

Output JSON:
{
    "strategy": "COLLECTION" | "SINGULAR" | "NARRATIVE",
    "confidence": 0.0 to 1.0,
    "reasoning": "brief explanation"
}"#;

/// Prompt for partitioning pages into distinct items.
pub const PARTITION_PROMPT: &str = r#"Given a query and page summaries, identify distinct items to extract.

Query: {query}

For this query, determine:
1. What constitutes ONE distinct item?
2. Which pages contribute to each item?
3. Why are these pages grouped together?

Page Summaries:
{summaries}

Output JSON array:
[
    {
        "title": "Brief item title",
        "urls": ["url1", "url2"],
        "rationale": "Why these pages are grouped"
    }
]

Rules:
- Each item should be distinct (no duplicates)
- Pages can appear in multiple items if they contain multiple distinct things
- If a page contains only one item, it gets its own partition
- Group pages that discuss the SAME specific thing"#;

/// Prompt for evidence-grounded extraction.
pub const EXTRACT_PROMPT: &str = r#"Extract information about: {query}

From these pages:
{pages}

Rules:
1. For EVERY claim, quote the source text that supports it
2. Note which page (URL) each quote comes from
3. Mark claims as:
   - DIRECT: Exact quote supports the claim
   - INFERRED: Reasonable inference from the source
   - ASSUMED: No direct evidence (WARNING: may be hallucination)
4. Explicitly note what information is MISSING (gaps)
5. If sources contradict each other, note the conflict

{hints_section}

Output JSON:
{
    "content": "Extracted information as markdown",
    "claims": [
        {
            "statement": "The claim being made",
            "evidence": [
                {
                    "quote": "Exact quote from source",
                    "source_url": "https://..."
                }
            ],
            "grounding": "DIRECT" | "INFERRED" | "ASSUMED"
        }
    ],
    "sources": [
        {
            "url": "https://...",
            "role": "PRIMARY" | "SUPPORTING" | "CORROBORATING"
        }
    ],
    "gaps": [
        {
            "field": "What's missing (e.g., 'contact email')",
            "query": "Search query to find it (e.g., 'the contact email for the volunteer coordinator')"
        }
    ],
    "conflicts": [
        {
            "topic": "What the conflict is about",
            "claims": [
                {"statement": "Claim A", "source_url": "url1"},
                {"statement": "Claim B", "source_url": "url2"}
            ]
        }
    ]
}"#;

/// Prompt for single-answer extraction (Singular strategy).
pub const EXTRACT_SINGLE_PROMPT: &str = r#"Find the answer to: {query}

From these pages:
{pages}

Rules:
1. Find the SINGLE best answer
2. Quote the source text that contains the answer
3. If multiple sources give different answers, note the conflict
4. If the answer is not found, say so clearly

Output JSON:
{
    "content": "The answer (or 'Not found' if not present)",
    "found": true | false,
    "source": {
        "url": "https://...",
        "quote": "Exact quote containing the answer"
    },
    "conflicts": [
        {
            "topic": "{query}",
            "claims": [
                {"statement": "Answer A", "source_url": "url1"},
                {"statement": "Answer B", "source_url": "url2"}
            ]
        }
    ]
}"#;

/// Prompt for narrative extraction (Narrative strategy).
pub const EXTRACT_NARRATIVE_PROMPT: &str = r#"Summarize information about: {query}

From these pages:
{pages}

Create a cohesive narrative that:
1. Synthesizes information from all relevant pages
2. Organizes information logically
3. Cites sources for key facts
4. Notes any contradictions between sources

Output JSON:
{
    "content": "Narrative summary as markdown with inline citations [1], [2], etc.",
    "sources": [
        {"number": 1, "url": "https://...", "title": "Page title"}
    ],
    "key_points": ["Main point 1", "Main point 2"],
    "conflicts": []
}"#;

/// Generate a hash of the summarization prompt for cache invalidation.
pub fn summarize_prompt_hash() -> String {
    let mut hasher = Sha256::new();
    hasher.update(SUMMARIZE_PROMPT.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Format the summarize prompt with content.
pub fn format_summarize_prompt(url: &str, content: &str) -> String {
    SUMMARIZE_PROMPT
        .replace("{url}", url)
        .replace("{content}", content)
}

/// Format the expand query prompt.
pub fn format_expand_query_prompt(query: &str) -> String {
    EXPAND_QUERY_PROMPT.replace("{query}", query)
}

/// Format the classify query prompt.
pub fn format_classify_query_prompt(query: &str) -> String {
    CLASSIFY_QUERY_PROMPT.replace("{query}", query)
}

/// Format the partition prompt with summaries.
pub fn format_partition_prompt(query: &str, summaries: &[(String, String)]) -> String {
    let summaries_text = summaries
        .iter()
        .map(|(url, text)| format!("URL: {}\nSummary: {}\n", url, text))
        .collect::<Vec<_>>()
        .join("\n---\n");

    PARTITION_PROMPT
        .replace("{query}", query)
        .replace("{summaries}", &summaries_text)
}

/// Format the extract prompt with pages and optional hints.
pub fn format_extract_prompt(
    query: &str,
    pages: &[(String, String)],
    hints: Option<&[String]>,
) -> String {
    let pages_text = pages
        .iter()
        .map(|(url, content)| format!("=== PAGE: {} ===\n{}\n", url, content))
        .collect::<Vec<_>>()
        .join("\n---\n");

    let hints_section = match hints {
        Some(h) if !h.is_empty() => {
            format!("Focus on extracting these fields: {}", h.join(", "))
        }
        _ => String::new(),
    };

    EXTRACT_PROMPT
        .replace("{query}", query)
        .replace("{pages}", &pages_text)
        .replace("{hints_section}", &hints_section)
}

/// Format the single extraction prompt.
pub fn format_extract_single_prompt(query: &str, pages: &[(String, String)]) -> String {
    let pages_text = pages
        .iter()
        .map(|(url, content)| format!("=== PAGE: {} ===\n{}\n", url, content))
        .collect::<Vec<_>>()
        .join("\n---\n");

    EXTRACT_SINGLE_PROMPT
        .replace("{query}", query)
        .replace("{pages}", &pages_text)
}

/// Format the narrative extraction prompt.
pub fn format_extract_narrative_prompt(query: &str, pages: &[(String, String)]) -> String {
    let pages_text = pages
        .iter()
        .map(|(url, content)| format!("=== PAGE: {} ===\n{}\n", url, content))
        .collect::<Vec<_>>()
        .join("\n---\n");

    EXTRACT_NARRATIVE_PROMPT
        .replace("{query}", query)
        .replace("{pages}", &pages_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_hash_is_consistent() {
        let hash1 = summarize_prompt_hash();
        let hash2 = summarize_prompt_hash();
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 hex
    }

    #[test]
    fn test_format_summarize_prompt() {
        let formatted = format_summarize_prompt("https://example.com", "Hello world");
        assert!(formatted.contains("https://example.com"));
        assert!(formatted.contains("Hello world"));
    }

    #[test]
    fn test_format_partition_prompt() {
        let summaries = vec![
            ("https://a.com".to_string(), "Summary A".to_string()),
            ("https://b.com".to_string(), "Summary B".to_string()),
        ];
        let formatted = format_partition_prompt("find things", &summaries);
        assert!(formatted.contains("find things"));
        assert!(formatted.contains("https://a.com"));
        assert!(formatted.contains("Summary B"));
    }

    #[test]
    fn test_format_extract_prompt_with_hints() {
        let pages = vec![("https://example.com".to_string(), "Content".to_string())];
        let hints = vec!["title".to_string(), "date".to_string()];

        let formatted = format_extract_prompt("query", &pages, Some(&hints));
        assert!(formatted.contains("title, date"));
    }

    #[test]
    fn test_format_extract_prompt_without_hints() {
        let pages = vec![("https://example.com".to_string(), "Content".to_string())];

        let formatted = format_extract_prompt("query", &pages, None);
        assert!(!formatted.contains("Focus on extracting"));
    }
}
