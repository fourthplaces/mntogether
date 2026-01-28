// Business logic for extracting volunteer needs from website content
//
// This is DOMAIN LOGIC that uses infrastructure (AI) from the kernel.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::kernel::BaseAI;

/// A volunteer need extracted from a website by AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedNeed {
    pub title: String,
    pub tldr: String,
    pub description: String,
    pub contact: Option<ContactInfo>,
    pub urgency: Option<String>,
    pub confidence: Option<String>, // "high" | "medium" | "low"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
}

/// Extract volunteer needs from scraped website content using AI
///
/// This is a domain function that constructs the business-specific prompt
/// and uses the generic AI capability from the kernel.
pub async fn extract_needs(
    ai: &dyn BaseAI,
    organization_name: &str,
    website_content: &str,
    source_url: &str,
) -> Result<Vec<ExtractedNeed>> {
    let prompt = format!(
        r#"You are analyzing a website for volunteer opportunities.

Organization: {organization_name}
Website URL: {source_url}

Content:
{website_content}

Extract all volunteer needs/opportunities mentioned on this page.

For each need, provide:
1. **title**: A clear, concise title (5-10 words)
2. **tldr**: A 1-2 sentence summary
3. **description**: Full details (what they need, requirements, impact)
4. **contact**: Any contact information (phone, email, website)
5. **urgency**: Estimate urgency ("urgent", "normal", or "low")
6. **confidence**: Your confidence in this extraction ("high", "medium", or "low")
   - "high": Explicitly stated volunteer opportunity with clear details
   - "medium": Mentioned but some details are inferred
   - "low": Vague or unclear, might not be a real opportunity

IMPORTANT RULES:
- ONLY extract REAL volunteer needs explicitly stated on the page
- DO NOT make up or infer needs that aren't clearly stated
- If the page has no volunteer opportunities, return an empty array
- Extract EVERY distinct need mentioned (don't summarize multiple needs into one)
- Include practical details: time commitment, location, skills needed, etc.
- Be honest about confidence - it helps human reviewers prioritize

Return ONLY valid JSON (no markdown, no explanation):
[
  {{
    "title": "...",
    "tldr": "...",
    "description": "...",
    "contact": {{ "phone": "...", "email": "...", "website": "..." }},
    "urgency": "normal",
    "confidence": "high"
  }}
]"#,
        organization_name = organization_name,
        source_url = source_url,
        website_content = website_content
    );

    // Use generic AI capability to get structured response
    let response = ai
        .complete_json(&prompt)
        .await
        .context("Failed to get AI response")?;

    let needs: Vec<ExtractedNeed> =
        serde_json::from_str(&response).context("Failed to parse AI response as JSON")?;

    Ok(needs)
}

/// Generate a concise summary (tldr) from a longer description
///
/// Uses AI to create a 1-2 sentence summary of the need description.
pub async fn generate_summary(ai: &dyn BaseAI, description: &str) -> Result<String> {
    let prompt = format!(
        r#"Summarize this volunteer need in 1-2 clear sentences. Focus on what help is needed and the impact.

Description:
{}

Return ONLY the summary (no markdown, no explanation)."#,
        description
    );

    let summary = ai
        .complete(&prompt)
        .await
        .context("Failed to generate summary")?;

    Ok(summary.trim().to_string())
}

/// Generate personalized outreach email copy for a volunteer need
///
/// Creates enthusiastic, specific, actionable email text that can be
/// used in mailto: links. Includes subject line and 3-sentence body.
pub async fn generate_outreach_copy(
    ai: &dyn BaseAI,
    organization_name: &str,
    need_title: &str,
    need_description: &str,
    contact_email: Option<&str>,
) -> Result<String> {
    let prompt = format!(
        r#"Generate a personalized outreach email for a volunteer reaching out about this opportunity:

Organization: {organization_name}
Opportunity: {need_title}
Details: {need_description}
Contact Email: {contact_email}

Write email copy that is:
1. **Enthusiastic** - Show genuine interest and excitement
2. **Specific** - Reference the actual opportunity by name
3. **Actionable** - Make it clear what you want (to volunteer/help)

Format as:
Subject: [subject line - max 50 chars]

[3 sentences - introduce yourself, express interest, ask how to get started]

Keep it professional but warm. Use "I" statements. Be concise.

Return ONLY the email text (no JSON, no markdown).
Example:
Subject: Interested in English Tutoring Program

Hi! I saw your English tutoring program and would love to help newly arrived families learn English. I have teaching experience and can commit to 2-3 hours per week. How can I get started?"#,
        organization_name = organization_name,
        need_title = need_title,
        need_description = need_description,
        contact_email = contact_email.unwrap_or("the organization")
    );

    let response = ai
        .complete(&prompt)
        .await
        .context("Failed to generate outreach copy")?;

    Ok(response.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::OpenAIClient;

    const SAMPLE_CONTENT: &str = r#"
# Volunteer Opportunities

## English Tutors Needed
We're looking for volunteers to help teach English to newly arrived refugee families.
Time commitment: 2-3 hours per week. Location: Minneapolis Community Center.
Contact: volunteer@example.org or call (612) 555-1234

## Food Pantry Assistants
Help us sort and distribute food donations every Saturday morning.
No experience necessary. Contact Sarah at (612) 555-5678.
    "#;

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_extract_needs() {
        let api_key = std::env::var("OPENAI_API_KEY")
            .expect("OPENAI_API_KEY must be set for integration tests");

        let ai = OpenAIClient::new(api_key);

        let needs = extract_needs(
            &ai,
            "Community Center",
            SAMPLE_CONTENT,
            "https://example.org/volunteer",
        )
        .await
        .expect("Extraction should succeed");

        assert!(
            needs.len() >= 2,
            "Should extract at least 2 needs from sample content"
        );

        for need in &needs {
            assert!(!need.title.is_empty());
            assert!(!need.description.is_empty());
            println!("Extracted need: {}", need.title);
        }
    }
}
