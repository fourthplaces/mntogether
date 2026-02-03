//! Rig-based extraction tools for agentic post enrichment
//!
//! Each tool implements the `rig::tool::Tool` trait for proper
//! function calling integration.

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

// =============================================================================
// Shared State for Tools
// =============================================================================

/// Collected enrichment data from tool calls
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnrichmentData {
    pub contact: Option<ContactData>,
    pub location: Option<LocationData>,
    pub schedule: Option<ScheduleData>,
    pub eligibility: Option<EligibilityData>,
    pub call_to_action: Option<CallToActionData>,
    pub finalized: bool,
    pub description: Option<String>,
    pub confidence: f32,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactData {
    pub phone: Option<String>,
    pub email: Option<String>,
    pub intake_form_url: Option<String>,
    pub contact_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationData {
    pub address: Option<String>,
    pub city: Option<String>,
    pub service_area: Option<String>,
    pub is_virtual: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleData {
    pub hours: Option<String>,
    pub dates: Option<String>,
    pub frequency: Option<String>,
    pub duration: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EligibilityData {
    pub who_qualifies: Option<String>,
    pub requirements: Vec<String>,
    pub restrictions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToActionData {
    pub action: String,
    pub url: Option<String>,
    pub instructions: Option<String>,
}

pub type SharedEnrichmentData = Arc<RwLock<EnrichmentData>>;

// =============================================================================
// Tool Error
// =============================================================================

#[derive(Debug, thiserror::Error)]
#[error("Extraction tool error: {0}")]
pub struct ExtractionError(String);

// =============================================================================
// Find Contact Info Tool
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindContactInfoArgs {
    pub phone: Option<String>,
    pub email: Option<String>,
    pub intake_form_url: Option<String>,
    pub contact_name: Option<String>,
}

#[derive(Clone)]
pub struct FindContactInfoTool {
    data: SharedEnrichmentData,
}

impl FindContactInfoTool {
    pub fn new(data: SharedEnrichmentData) -> Self {
        Self { data }
    }
}

impl Tool for FindContactInfoTool {
    const NAME: &'static str = "find_contact_info";
    type Error = ExtractionError;
    type Args = FindContactInfoArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search the page content to find contact information for this post. Look for phone numbers, emails, contact forms, and intake form URLs.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "phone": {"type": "string", "description": "Phone number (format: xxx-xxx-xxxx)"},
                    "email": {"type": "string", "description": "Email address"},
                    "intake_form_url": {"type": "string", "description": "URL to signup/intake form"},
                    "contact_name": {"type": "string", "description": "Contact person name"}
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        tracing::info!(
            phone = ?args.phone,
            email = ?args.email,
            intake_form = ?args.intake_form_url,
            "[tool-call] Recording contact info"
        );

        let mut data = self.data.write().await;
        data.contact = Some(ContactData {
            phone: args.phone,
            email: args.email,
            intake_form_url: args.intake_form_url,
            contact_name: args.contact_name,
        });

        Ok("Contact info recorded".to_string())
    }
}

// =============================================================================
// Find Location Tool
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindLocationArgs {
    pub address: Option<String>,
    pub city: Option<String>,
    pub service_area: Option<String>,
    #[serde(default)]
    pub is_virtual: bool,
}

#[derive(Clone)]
pub struct FindLocationTool {
    data: SharedEnrichmentData,
}

impl FindLocationTool {
    pub fn new(data: SharedEnrichmentData) -> Self {
        Self { data }
    }
}

impl Tool for FindLocationTool {
    const NAME: &'static str = "find_location";
    type Error = ExtractionError;
    type Args = FindLocationArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search the page content to find location/address information."
                .to_string(),
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
        tracing::info!(
            address = ?args.address,
            city = ?args.city,
            "[tool-call] Recording location info"
        );

        let mut data = self.data.write().await;
        data.location = Some(LocationData {
            address: args.address,
            city: args.city,
            service_area: args.service_area,
            is_virtual: args.is_virtual,
        });

        Ok("Location info recorded".to_string())
    }
}

// =============================================================================
// Find Schedule Tool
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindScheduleArgs {
    pub hours: Option<String>,
    pub dates: Option<String>,
    pub frequency: Option<String>,
    pub duration: Option<String>,
}

#[derive(Clone)]
pub struct FindScheduleTool {
    data: SharedEnrichmentData,
}

impl FindScheduleTool {
    pub fn new(data: SharedEnrichmentData) -> Self {
        Self { data }
    }
}

impl Tool for FindScheduleTool {
    const NAME: &'static str = "find_schedule";
    type Error = ExtractionError;
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
                    "frequency": {"type": "string", "description": "How often: weekly, monthly, one-time, ongoing"},
                    "duration": {"type": "string", "description": "Time commitment (e.g., '2 hours')"}
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        tracing::info!(
            hours = ?args.hours,
            dates = ?args.dates,
            "[tool-call] Recording schedule info"
        );

        let mut data = self.data.write().await;
        data.schedule = Some(ScheduleData {
            hours: args.hours,
            dates: args.dates,
            frequency: args.frequency,
            duration: args.duration,
        });

        Ok("Schedule info recorded".to_string())
    }
}

// =============================================================================
// Find Eligibility Tool
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindEligibilityArgs {
    pub who_qualifies: Option<String>,
    #[serde(default)]
    pub requirements: Vec<String>,
    pub restrictions: Option<String>,
}

#[derive(Clone)]
pub struct FindEligibilityTool {
    data: SharedEnrichmentData,
}

impl FindEligibilityTool {
    pub fn new(data: SharedEnrichmentData) -> Self {
        Self { data }
    }
}

impl Tool for FindEligibilityTool {
    const NAME: &'static str = "find_eligibility";
    type Error = ExtractionError;
    type Args = FindEligibilityArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search the page content to find who can use this service or participate."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "who_qualifies": {"type": "string", "description": "Target audience"},
                    "requirements": {"type": "array", "items": {"type": "string"}, "description": "List of requirements"},
                    "restrictions": {"type": "string", "description": "Any restrictions"}
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        tracing::info!(
            who = ?args.who_qualifies,
            "[tool-call] Recording eligibility info"
        );

        let mut data = self.data.write().await;
        data.eligibility = Some(EligibilityData {
            who_qualifies: args.who_qualifies,
            requirements: args.requirements,
            restrictions: args.restrictions,
        });

        Ok("Eligibility info recorded".to_string())
    }
}

// =============================================================================
// Find Call to Action Tool
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindCallToActionArgs {
    pub action: String,
    pub url: Option<String>,
    pub instructions: Option<String>,
}

#[derive(Clone)]
pub struct FindCallToActionTool {
    data: SharedEnrichmentData,
}

impl FindCallToActionTool {
    pub fn new(data: SharedEnrichmentData) -> Self {
        Self { data }
    }
}

impl Tool for FindCallToActionTool {
    const NAME: &'static str = "find_call_to_action";
    type Error = ExtractionError;
    type Args = FindCallToActionArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description:
                "Determine what action someone should take to engage with this opportunity."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {"type": "string", "description": "What to do: 'Sign up online', 'Call to register', etc."},
                    "url": {"type": "string", "description": "URL to take action"},
                    "instructions": {"type": "string", "description": "Any specific instructions"}
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        tracing::info!(
            action = %args.action,
            url = ?args.url,
            "[tool-call] Recording call to action"
        );

        let mut data = self.data.write().await;
        data.call_to_action = Some(CallToActionData {
            action: args.action,
            url: args.url,
            instructions: args.instructions,
        });

        Ok("Call to action recorded".to_string())
    }
}

// =============================================================================
// Finalize Post Tool
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalizePostArgs {
    pub description: String,
    pub confidence: f32,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Clone)]
pub struct FinalizePostTool {
    data: SharedEnrichmentData,
}

impl FinalizePostTool {
    pub fn new(data: SharedEnrichmentData) -> Self {
        Self { data }
    }
}

impl Tool for FinalizePostTool {
    const NAME: &'static str = "finalize_post";
    type Error = ExtractionError;
    type Args = FinalizePostArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Call this when you have gathered all available information for the post."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "description": {"type": "string", "description": "Full, detailed description (2-4 sentences)"},
                    "confidence": {"type": "number", "description": "Confidence score 0-1"},
                    "notes": {"type": "array", "items": {"type": "string"}, "description": "Notes about what was found"}
                },
                "required": ["description", "confidence"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        tracing::info!(
            confidence = args.confidence,
            notes_count = args.notes.len(),
            "[tool-call] Finalizing post"
        );

        let mut data = self.data.write().await;
        data.description = Some(args.description);
        data.confidence = args.confidence;
        data.notes = args.notes;
        data.finalized = true;

        Ok("Post finalized".to_string())
    }
}

// =============================================================================
// Search Other Pages Tool
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOtherPagesArgs {
    pub query: String,
    #[serde(default)]
    pub page_types: Vec<String>,
}

/// Tool that searches other pages on the same website
#[derive(Clone)]
pub struct SearchOtherPagesTool {
    /// Map of page URL -> content
    other_pages: Arc<std::collections::HashMap<String, String>>,
}

impl SearchOtherPagesTool {
    pub fn new(other_pages: std::collections::HashMap<String, String>) -> Self {
        Self {
            other_pages: Arc::new(other_pages),
        }
    }
}

impl Tool for SearchOtherPagesTool {
    const NAME: &'static str = "search_other_pages";
    type Error = ExtractionError;
    type Args = SearchOtherPagesArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search other pages on this website (like /contact, /about, /hours) for missing information.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "What to search for (e.g., 'phone number', 'address')"},
                    "page_types": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Types of pages to search: 'contact', 'about', 'hours'"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let page_types = if args.page_types.is_empty() {
            vec![
                "contact".to_string(),
                "about".to_string(),
                "hours".to_string(),
            ]
        } else {
            args.page_types
        };

        tracing::info!(
            query = %args.query,
            page_types = ?page_types,
            "[tool-call] Searching other pages"
        );

        let mut found_content = String::new();
        for (url, content) in self.other_pages.iter() {
            let url_lower = url.to_lowercase();
            if page_types.iter().any(|pt| url_lower.contains(pt)) {
                if content.to_lowercase().contains(&args.query.to_lowercase()) {
                    // Truncate to relevant portion
                    let truncated = if content.len() > 2000 {
                        &content[..2000]
                    } else {
                        content
                    };
                    found_content.push_str(&format!("\n\n--- From {} ---\n{}", url, truncated));
                }
            }
        }

        if found_content.is_empty() {
            Ok(format!(
                "No matches found for '{}' in other pages",
                args.query
            ))
        } else {
            Ok(format!("Found relevant content:{}", found_content))
        }
    }
}
