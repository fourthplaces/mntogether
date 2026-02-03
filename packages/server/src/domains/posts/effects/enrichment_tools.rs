//! Rig-based Tool implementations for post enrichment.
//!
//! These tools replace the JSON-based tool definitions in `get_enrichment_tools()`.
//! Each tool implements rig's `Tool` trait for type-safe tool calling.
//!
//! Tools write to shared state (`SharedEnrichmentData`) which is collected
//! after the agent loop completes.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::common::{CallToAction, ContactInfo, EligibilityInfo, LocationInfo, ScheduleInfo};

// WebSearcher is available but not used in tools due to rig's Sync requirement
#[allow(unused_imports)]
use extraction::WebSearcher;

// =============================================================================
// Shared State
// =============================================================================

/// Collected enrichment data from tool calls.
/// Uses unified types from crate::common.
#[derive(Debug, Clone, Default)]
pub struct EnrichmentState {
    pub contact: Option<ContactInfo>,
    pub location: Option<LocationInfo>,
    pub schedule: Option<ScheduleInfo>,
    pub eligibility: Option<EligibilityInfo>,
    pub call_to_action: Option<CallToAction>,
    pub finalized: bool,
    pub description: Option<String>,
    pub confidence: f32,
    pub notes: Vec<String>,
}

/// Thread-safe shared enrichment state for use across async tool calls
pub type SharedEnrichmentState = Arc<RwLock<EnrichmentState>>;

/// Context for tools that need to search other pages or the web
pub struct ToolContext {
    pub other_pages: HashMap<String, String>,
    pub web_searcher: Option<Arc<dyn WebSearcher>>,
}

// =============================================================================
// Tool Error
// =============================================================================

#[derive(Debug, thiserror::Error)]
#[error("Enrichment tool error: {0}")]
pub struct EnrichmentToolError(pub String);

// =============================================================================
// Tool Argument Types (JsonSchema for automatic schema generation)
// =============================================================================

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct FindContactArgs {
    /// Phone number found (format: xxx-xxx-xxxx)
    pub phone: Option<String>,
    /// Email address found
    pub email: Option<String>,
    /// URL to signup/intake/registration form
    pub intake_form_url: Option<String>,
    /// Name of contact person if mentioned
    pub contact_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct FindLocationArgs {
    /// Street address
    pub address: Option<String>,
    /// City name
    pub city: Option<String>,
    /// Geographic area served
    pub service_area: Option<String>,
    /// True if online/virtual only
    pub is_virtual: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct FindScheduleArgs {
    /// Operating hours (e.g., 'Mon-Fri 9am-5pm')
    pub hours: Option<String>,
    /// Specific dates or recurring pattern
    pub dates: Option<String>,
    /// How often: 'weekly', 'monthly', 'one-time', 'ongoing'
    pub frequency: Option<String>,
    /// Time commitment (e.g., '2 hours')
    pub duration: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct FindEligibilityArgs {
    /// Target audience (e.g., 'Low-income families')
    pub who_qualifies: Option<String>,
    /// List of requirements
    pub requirements: Option<Vec<String>>,
    /// Any restrictions
    pub restrictions: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct FindCallToActionArgs {
    /// What to do: 'Sign up online', 'Call to register', etc.
    pub action: Option<String>,
    /// URL to take action
    pub url: Option<String>,
    /// Any specific instructions
    pub instructions: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SearchOtherPagesArgs {
    /// What to search for (e.g., 'phone number', 'address', 'hours of operation')
    pub query: String,
    /// Types of pages to search: 'contact', 'about', 'hours', 'location', 'volunteer'
    pub page_types: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SearchExternalArgs {
    /// Search query (e.g., 'DHHMN phone number', 'Deaf Hard of Hearing Services Minnesota address')
    pub query: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct FinalizePostArgs {
    /// Full, detailed description (2-4 sentences)
    pub description: String,
    /// Confidence score 0-1
    pub confidence: f64,
    /// Notes about what was found or couldn't be found
    pub notes: Vec<String>,
}

// =============================================================================
// Tool Implementations
// =============================================================================

/// Tool to find and record contact information
pub struct FindContactTool {
    state: SharedEnrichmentState,
}

impl FindContactTool {
    pub fn new(state: SharedEnrichmentState) -> Self {
        Self { state }
    }
}

impl Tool for FindContactTool {
    const NAME: &'static str = "find_contact_info";
    type Error = EnrichmentToolError;
    type Args = FindContactArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search the page content to find contact information for this specific post/opportunity. Look for phone numbers, emails, contact forms, and intake form URLs.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "phone": {"type": "string", "description": "Phone number found (format: xxx-xxx-xxxx)"},
                    "email": {"type": "string", "description": "Email address found"},
                    "intake_form_url": {"type": "string", "description": "URL to signup/intake/registration form"},
                    "contact_name": {"type": "string", "description": "Name of contact person if mentioned"}
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        info!("[tool-call] find_contact_info");
        let mut state = self.state.write().await;
        state.contact = Some(ContactInfo {
            phone: args.phone,
            email: args.email,
            intake_form_url: args.intake_form_url,
            contact_name: args.contact_name,
            ..Default::default()
        });
        Ok("Contact info recorded".to_string())
    }
}

/// Tool to find and record location information
pub struct FindLocationTool {
    state: SharedEnrichmentState,
}

impl FindLocationTool {
    pub fn new(state: SharedEnrichmentState) -> Self {
        Self { state }
    }
}

impl Tool for FindLocationTool {
    const NAME: &'static str = "find_location";
    type Error = EnrichmentToolError;
    type Args = FindLocationArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search the page content to find location/address information for this post.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "address": {"type": "string", "description": "Street address"},
                    "city": {"type": "string", "description": "City name"},
                    "service_area": {"type": "string", "description": "Geographic area served"},
                    "is_virtual": {"type": "boolean", "description": "True if online/virtual only"}
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        info!("[tool-call] find_location");
        let mut state = self.state.write().await;
        state.location = Some(LocationInfo {
            address: args.address,
            city: args.city,
            service_area: args.service_area,
            is_virtual: args.is_virtual.unwrap_or(false),
            ..Default::default()
        });
        Ok("Location info recorded".to_string())
    }
}

/// Tool to find and record schedule information
pub struct FindScheduleTool {
    state: SharedEnrichmentState,
}

impl FindScheduleTool {
    pub fn new(state: SharedEnrichmentState) -> Self {
        Self { state }
    }
}

impl Tool for FindScheduleTool {
    const NAME: &'static str = "find_schedule";
    type Error = EnrichmentToolError;
    type Args = FindScheduleArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search the page content to find schedule/timing information.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "hours": {"type": "string", "description": "Operating hours (e.g., 'Mon-Fri 9am-5pm')"},
                    "dates": {"type": "string", "description": "Specific dates or recurring pattern"},
                    "frequency": {"type": "string", "description": "How often: 'weekly', 'monthly', 'one-time', 'ongoing'"},
                    "duration": {"type": "string", "description": "Time commitment (e.g., '2 hours')"}
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        info!("[tool-call] find_schedule");
        let mut state = self.state.write().await;
        state.schedule = Some(ScheduleInfo {
            general: args.hours,
            dates: args.dates,
            frequency: args.frequency,
            duration: args.duration,
            ..Default::default()
        });
        Ok("Schedule info recorded".to_string())
    }
}

/// Tool to find and record eligibility information
pub struct FindEligibilityTool {
    state: SharedEnrichmentState,
}

impl FindEligibilityTool {
    pub fn new(state: SharedEnrichmentState) -> Self {
        Self { state }
    }
}

impl Tool for FindEligibilityTool {
    const NAME: &'static str = "find_eligibility";
    type Error = EnrichmentToolError;
    type Args = FindEligibilityArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search the page content to find who can use this service or participate.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "who_qualifies": {"type": "string", "description": "Target audience (e.g., 'Low-income families')"},
                    "requirements": {"type": "array", "items": {"type": "string"}, "description": "List of requirements"},
                    "restrictions": {"type": "string", "description": "Any restrictions"}
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        info!("[tool-call] find_eligibility");
        let mut state = self.state.write().await;
        state.eligibility = Some(EligibilityInfo {
            who_qualifies: args.who_qualifies,
            requirements: args.requirements.unwrap_or_default(),
            restrictions: args.restrictions,
        });
        Ok("Eligibility info recorded".to_string())
    }
}

/// Tool to find and record call to action
pub struct FindCallToActionTool {
    state: SharedEnrichmentState,
}

impl FindCallToActionTool {
    pub fn new(state: SharedEnrichmentState) -> Self {
        Self { state }
    }
}

impl Tool for FindCallToActionTool {
    const NAME: &'static str = "find_call_to_action";
    type Error = EnrichmentToolError;
    type Args = FindCallToActionArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Determine what action someone should take to engage with this opportunity.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {"type": "string", "description": "What to do: 'Sign up online', 'Call to register', etc."},
                    "url": {"type": "string", "description": "URL to take action"},
                    "instructions": {"type": "string", "description": "Any specific instructions"}
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        info!("[tool-call] find_call_to_action");
        let mut state = self.state.write().await;
        state.call_to_action = Some(CallToAction {
            action: args.action.unwrap_or_else(|| "Contact for more info".to_string()),
            url: args.url,
            instructions: args.instructions,
        });
        Ok("Call to action recorded".to_string())
    }
}

/// Tool to search other pages on the website
pub struct SearchOtherPagesTool {
    state: SharedEnrichmentState,
    other_pages: HashMap<String, String>,
}

impl SearchOtherPagesTool {
    pub fn new(state: SharedEnrichmentState, other_pages: HashMap<String, String>) -> Self {
        Self { state, other_pages }
    }
}

impl Tool for SearchOtherPagesTool {
    const NAME: &'static str = "search_other_pages";
    type Error = EnrichmentToolError;
    type Args = SearchOtherPagesArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search other pages on this website (like /contact, /about, /hours) for missing information. Use this if you can't find contact info, location, or hours on the current page.".to_string(),
            parameters: json!({
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
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        info!("[tool-call] search_other_pages: {}", args.query);

        let page_types = args.page_types.unwrap_or_else(|| vec!["contact".to_string(), "about".to_string(), "hours".to_string()]);

        let mut found_content = String::new();
        for (url, content) in &self.other_pages {
            let url_lower = url.to_lowercase();
            if page_types.iter().any(|pt| url_lower.contains(pt)) {
                if content.to_lowercase().contains(&args.query.to_lowercase()) {
                    found_content.push_str(&format!(
                        "\n\n--- From {} ---\n{}",
                        url,
                        truncate_content(content, 2000)
                    ));
                }
            }
        }

        let result = if found_content.is_empty() {
            format!("No matches found for '{}' in other pages", args.query)
        } else {
            format!("Found relevant content:{}", found_content)
        };

        // Record the search in notes
        let mut state = self.state.write().await;
        state.notes.push(format!("Searched other pages for: {}", args.query));

        Ok(result)
    }
}

/// Tool to search the web for external information.
///
/// Note: Due to rig's Tool trait requiring Sync futures, this tool currently
/// only records the search query but does not perform actual web searches.
/// The actual search results would need to be pre-fetched or this tool
/// would need to be implemented differently (e.g., using message passing).
pub struct SearchExternalTool {
    state: SharedEnrichmentState,
    #[allow(dead_code)]
    web_searcher_available: bool,
}

impl SearchExternalTool {
    pub fn new(state: SharedEnrichmentState, web_searcher: Option<Arc<dyn WebSearcher>>) -> Self {
        Self {
            state,
            web_searcher_available: web_searcher.is_some(),
        }
    }
}

impl Tool for SearchExternalTool {
    const NAME: &'static str = "search_external";
    type Error = EnrichmentToolError;
    type Args = SearchExternalArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search the web for additional information about this organization or opportunity. Use this as a last resort if information is not on the website.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Search query (e.g., 'DHHMN phone number', 'Deaf Hard of Hearing Services Minnesota address')"}
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        info!("[tool-call] search_external: {}", args.query);

        // Record the search in notes
        // Note: Actual web search is not performed in rig-based tools due to Sync requirements.
        // The LLM should focus on extracting from page content and other pages on the site.
        let mut state = self.state.write().await;
        state.notes.push(format!("External search requested: {}", args.query));

        Ok("External search is not available in this enrichment context. Please use the information from the current page and search_other_pages to find what you need.".to_string())
    }
}

/// Tool to finalize the post enrichment
pub struct FinalizePostTool {
    state: SharedEnrichmentState,
}

impl FinalizePostTool {
    pub fn new(state: SharedEnrichmentState) -> Self {
        Self { state }
    }
}

impl Tool for FinalizePostTool {
    const NAME: &'static str = "finalize_post";
    type Error = EnrichmentToolError;
    type Args = FinalizePostArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Call this when you have gathered all available information for the post.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "description": {"type": "string", "description": "Full, detailed description (2-4 sentences)"},
                    "confidence": {"type": "number", "description": "Confidence score 0-1"},
                    "notes": {"type": "array", "items": {"type": "string"}, "description": "Notes about what was found or couldn't be found"}
                },
                "required": ["description", "confidence", "notes"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        info!("[tool-call] finalize_post with confidence {}", args.confidence);

        let mut state = self.state.write().await;
        state.description = Some(args.description);
        state.confidence = args.confidence as f32;
        state.notes.extend(args.notes);
        state.finalized = true;

        Ok("Post enrichment finalized".to_string())
    }
}

// =============================================================================
// Helper Functions
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_find_contact_tool() {
        let state = Arc::new(RwLock::new(EnrichmentState::default()));
        let tool = FindContactTool::new(state.clone());

        let args = FindContactArgs {
            phone: Some("612-555-1234".to_string()),
            email: Some("test@example.com".to_string()),
            intake_form_url: None,
            contact_name: Some("John Doe".to_string()),
        };

        let result = tool.call(args).await.unwrap();
        assert_eq!(result, "Contact info recorded");

        let state = state.read().await;
        assert!(state.contact.is_some());
        let contact = state.contact.as_ref().unwrap();
        assert_eq!(contact.phone, Some("612-555-1234".to_string()));
        assert_eq!(contact.email, Some("test@example.com".to_string()));
    }

    #[tokio::test]
    async fn test_finalize_post_tool() {
        let state = Arc::new(RwLock::new(EnrichmentState::default()));
        let tool = FinalizePostTool::new(state.clone());

        let args = FinalizePostArgs {
            description: "A great volunteer opportunity".to_string(),
            confidence: 0.85,
            notes: vec!["Found contact on about page".to_string()],
        };

        let result = tool.call(args).await.unwrap();
        assert_eq!(result, "Post enrichment finalized");

        let state = state.read().await;
        assert!(state.finalized);
        assert_eq!(state.confidence, 0.85);
        assert_eq!(state.description, Some("A great volunteer opportunity".to_string()));
    }
}
