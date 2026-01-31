// AI-powered agent configuration generator
//
// Takes a natural language description of what an agent should do
// and generates the technical configuration (search queries, extraction instructions)

use crate::kernel::BaseAI;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfigRequest {
    pub description: String,
    pub location_context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfigSuggestion {
    pub name: String,
    pub query_template: String,
    pub extraction_instructions: String,
    pub system_prompt: String,
}

/// Generate agent configuration from natural language description
pub async fn generate_agent_config(
    ai: &dyn BaseAI,
    request: AgentConfigRequest,
) -> Result<AgentConfigSuggestion> {
    let prompt = format!(
        r#"You are an expert at creating autonomous search agents for community resources.

Given a description of what the agent should search for, generate:
1. A concise agent name (3-5 words)
2. A Tavily search query template (use {{location}} placeholder for location)
3. Detailed extraction instructions for the AI when it scrapes pages
4. A system prompt for the AI extractor

USER DESCRIPTION:
{description}

LOCATION CONTEXT: {location}

GUIDELINES:
- Query template should use OR operators and be broad enough to find relevant sites
- Use {{location}} as a placeholder for the location context
- Extraction instructions should specify EXACTLY what information to extract
- Be specific about data points: eligibility, contact info, hours, languages, fees, etc.
- System prompt should establish the AI's expertise and extraction approach

Return ONLY a JSON object with this EXACT structure (no markdown, no code blocks):
{{
  "name": "Agent Name Here",
  "query_template": "search terms OR \"quoted phrases\" {{location}}",
  "extraction_instructions": "Extract specific data points including...",
  "system_prompt": "You are an expert at..."
}}

EXAMPLES:

Input: "Find legal aid for immigrants"
Output:
{{
  "name": "Legal Aid for Immigrants",
  "query_template": "legal aid immigrants refugees \"immigration lawyer\" pro bono {{location}}",
  "extraction_instructions": "Extract legal services for immigrants including: eligibility requirements (income limits, visa status), languages offered, types of cases handled (asylum, citizenship, deportation defense), contact information (phone, email, website), office hours, and fee structure (free, sliding scale, pro bono). Note if interpretation services are available.",
  "system_prompt": "You are an expert at identifying legal aid services for immigrant communities. Focus on extracting comprehensive eligibility criteria, language accessibility, and specific immigration law services offered. Prioritize information about free or low-cost services."
}}

Input: "Volunteer opportunities helping seniors"
Output:
{{
  "name": "Senior Volunteer Programs",
  "query_template": "volunteer opportunities seniors elderly \"help seniors\" companionship {{location}}",
  "extraction_instructions": "Extract volunteer opportunities focused on seniors including: time commitment required, skills needed, age restrictions, background check requirements, training provided, types of activities (companionship, transportation, meal delivery, technology help), scheduling flexibility, and contact information. Note if remote volunteering is available.",
  "system_prompt": "You are an expert at identifying volunteer opportunities that serve senior citizens. Extract detailed requirements for volunteers, types of support provided to seniors, and logistics like time commitment and scheduling. Highlight opportunities suitable for volunteers of all skill levels."
}}

Now generate the configuration:"#,
        description = request.description,
        location = request.location_context
    );

    let response = ai.complete_json(&prompt).await?;

    // Parse JSON response
    let suggestion: AgentConfigSuggestion = serde_json::from_str(&response)?;

    Ok(suggestion)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::OpenAIClient;

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_generate_agent_config() {
        let api_key = std::env::var("OPENAI_API_KEY")
            .expect("OPENAI_API_KEY must be set for integration tests");

        let ai = OpenAIClient::new(api_key);

        let request = AgentConfigRequest {
            description: "Find food banks and emergency food assistance for families".to_string(),
            location_context: "Minneapolis, Minnesota".to_string(),
        };

        let suggestion = generate_agent_config(&ai, request)
            .await
            .expect("Should generate config");

        println!("Name: {}", suggestion.name);
        println!("Query: {}", suggestion.query_template);
        println!("Extraction: {}", suggestion.extraction_instructions);
        println!("System: {}", suggestion.system_prompt);

        assert!(!suggestion.name.is_empty());
        assert!(suggestion.query_template.contains("{location}"));
        assert!(!suggestion.extraction_instructions.is_empty());
        assert!(!suggestion.system_prompt.is_empty());
    }
}
