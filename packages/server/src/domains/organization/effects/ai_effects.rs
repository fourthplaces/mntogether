use anyhow::{Context, Result};
use rig::providers::openai;
use rig::completion::Prompt;
use serde::{Deserialize, Serialize};

/// AI client for extracting volunteer needs from website content
pub struct NeedExtractor {
    client: openai::Client,
}

impl NeedExtractor {
    pub fn new(api_key: String) -> Self {
        let client = openai::Client::new(&api_key);
        Self { client }
    }

    /// Extract volunteer needs from scraped website content
    pub async fn extract_needs(
        &self,
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

IMPORTANT RULES:
- ONLY extract REAL volunteer needs explicitly stated on the page
- DO NOT make up or infer needs that aren't clearly stated
- If the page has no volunteer opportunities, return an empty array
- Extract EVERY distinct need mentioned (don't summarize multiple needs into one)
- Include practical details: time commitment, location, skills needed, etc.

Return ONLY valid JSON (no markdown, no explanation):
[
  {{
    "title": "...",
    "tldr": "...",
    "description": "...",
    "contact": {{ "phone": "...", "email": "...", "website": "..." }},
    "urgency": "normal"
  }}
]"#,
            organization_name = organization_name,
            source_url = source_url,
            website_content = website_content
        );

        // Call GPT-4o with structured output
        let agent = self
            .client
            .agent("gpt-4o")
            .preamble("You are a volunteer opportunity extraction assistant. Extract structured data from website content.")
            .build();

        let response = agent
            .prompt(&prompt)
            .await
            .context("Failed to call OpenAI API")?;

        // Parse JSON response
        let needs: Vec<ExtractedNeed> = serde_json::from_str(&response)
            .context("Failed to parse extracted needs as JSON")?;

        Ok(needs)
    }
}

/// A volunteer need extracted from a website by AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedNeed {
    pub title: String,
    pub tldr: String,
    pub description: String,
    pub contact: Option<ContactInfo>,
    pub urgency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let extractor = NeedExtractor::new(api_key);

        let needs = extractor
            .extract_needs(
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
